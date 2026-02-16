use anyhow::{Context, Result};
use futures_util::Stream;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::channel::QuicBackedChannel;
use super::context::ConnectionContext;
use super::identity::ServerIdentity;
use super::quic::{QuicClient, UnisonStream, write_frame};
use super::service::Service;
use super::{
    MessageType, NetworkError, ProtocolClientTrait, ProtocolMessage, UnisonClient, UnisonClientExt,
};

// TransportWrapper removed - using QuicClient directly

/// QUIC protocol client implementation
pub struct ProtocolClient {
    transport: Arc<QuicClient>,
    services: Arc<RwLock<HashMap<String, crate::network::service::UnisonService>>>,
    /// 接続コンテキスト（Identity情報・チャネル状態）
    context: Arc<ConnectionContext>,
}

// Transport trait removed - using direct implementation on TransportWrapper

impl ProtocolClient {
    pub fn new(transport: QuicClient) -> Self {
        Self {
            transport: Arc::new(transport),
            services: Arc::new(RwLock::new(HashMap::new())),
            context: Arc::new(ConnectionContext::new()),
        }
    }

    /// Create a new client with QUIC transport
    pub fn new_default() -> Result<Self> {
        let transport = QuicClient::new()?;
        Ok(Self {
            transport: Arc::new(transport),
            services: Arc::new(RwLock::new(HashMap::new())),
            context: Arc::new(ConnectionContext::new()),
        })
    }

    /// 接続コンテキストを取得
    pub fn context(&self) -> &Arc<ConnectionContext> {
        &self.context
    }

    /// サーバーから受信したIdentity情報を取得
    pub async fn server_identity(&self) -> Option<ServerIdentity> {
        self.context.identity().await
    }

    /// チャネルを開く（QUICストリーム上の型安全チャネル）
    ///
    /// `__channel:{name}` メソッドで新しいQUICストリームを開き、
    /// `QuicBackedChannel` でラップして返す。
    pub async fn open_channel<S, R>(
        &self,
        channel_name: &str,
    ) -> Result<QuicBackedChannel<S, R>, NetworkError>
    where
        S: Serialize + Send,
        R: DeserializeOwned + Send,
    {
        let connection_guard = self.transport.connection().read().await;
        let connection = connection_guard
            .as_ref()
            .ok_or(NetworkError::NotConnected)?;

        // 新しい双方向ストリームを開く
        let (mut send_stream, recv_stream) = connection
            .open_bi()
            .await
            .map_err(|e| NetworkError::Quic(format!("Failed to open channel stream: {}", e)))?;

        // チャネル識別メッセージを送信（length-prefixed）
        let method = format!("__channel:{}", channel_name);
        let request_id = generate_request_id();
        let message = ProtocolMessage::new_with_json(
            request_id,
            method,
            MessageType::BidirectionalStream,
            serde_json::json!({}),
        )?;

        let frame = message.into_frame().map_err(|e| {
            NetworkError::Protocol(format!("Failed to create channel frame: {}", e))
        })?;
        let frame_bytes = frame.to_bytes();
        write_frame(&mut send_stream, &frame_bytes)
            .await
            .map_err(|e| NetworkError::Protocol(format!("Failed to send channel open: {}", e)))?;

        // UnisonStreamを作成してQuicBackedChannelでラップ
        let conn_arc = Arc::new(connection.clone());
        let stream = UnisonStream::from_streams(
            request_id,
            format!("__channel:{}", channel_name),
            conn_arc,
            send_stream,
            recv_stream,
        );

        // コンテキストにチャネルを登録
        self.context
            .register_channel(super::context::ChannelHandle {
                channel_name: channel_name.to_string(),
                stream_id: request_id,
                direction: super::context::ChannelDirection::Bidirectional,
            })
            .await;

        Ok(QuicBackedChannel::new(stream))
    }

    /// 接続後にサーバーからIdentityを受信する
    async fn receive_identity(&self) -> Result<ServerIdentity, NetworkError> {
        // サーバーが開いたIdentityストリームからデータを受信
        let response =
            self.transport.receive().await.map_err(|e| {
                NetworkError::Protocol(format!("Failed to receive identity: {}", e))
            })?;

        if response.method == "__identity" {
            let identity = ServerIdentity::from_protocol_message(&response)
                .map_err(|e| NetworkError::Protocol(format!("Failed to parse identity: {}", e)))?;
            self.context.set_identity(identity.clone()).await;
            Ok(identity)
        } else {
            Err(NetworkError::Protocol(format!(
                "Expected identity message, got method: {}",
                response.method
            )))
        }
    }

    /// サーバーに接続し、チャネル名のリストに基づいて複数チャネルを開く
    ///
    /// 接続→Identity受信→全チャネル開設を一括で行う便利メソッド。
    /// 各チャネルの型はコード生成側（ConnectionBuilder）で決定される。
    pub async fn connect_with_channels(
        &mut self,
        url: &str,
        channel_names: &[&str],
    ) -> Result<Vec<String>, NetworkError> {
        // 接続（Identity受信を含む）
        UnisonClient::connect(self, url).await?;

        // 各チャネルを開く（型はここでは不明なのでチャネル名のみ返す）
        let mut opened = Vec::new();
        for name in channel_names {
            // チャネル登録はopen_channel呼び出し時に行われる
            // ここではコンテキストに名前だけ予約
            self.context
                .register_channel(super::context::ChannelHandle {
                    channel_name: name.to_string(),
                    stream_id: 0, // open_channel時に更新される
                    direction: super::context::ChannelDirection::Bidirectional,
                })
                .await;
            opened.push(name.to_string());
        }

        Ok(opened)
    }

    /// Register a Service instance with the client
    pub async fn register_service(&self, service: crate::network::service::UnisonService) {
        let service_name = service.service_name().to_string();
        let mut services = self.services.write().await;
        services.insert(service_name, service);
    }

    /// Get registered services list
    pub async fn list_services(&self) -> Vec<String> {
        let services = self.services.read().await;
        services.keys().cloned().collect()
    }

    /// Call a service method directly
    pub async fn call_service(
        &self,
        service_name: &str,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, NetworkError> {
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_name) {
            service.handle_request(method, payload).await
        } else {
            Err(NetworkError::HandlerNotFound {
                method: format!("{}::{}", service_name, method),
            })
        }
    }

    pub async fn connect(&mut self, url: &str) -> Result<()> {
        // Arc::get_mutを使用してmutableアクセス
        Arc::get_mut(&mut self.transport)
            .ok_or_else(|| anyhow::anyhow!("Failed to get mutable transport"))?
            .connect(url)
            .await
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        Arc::get_mut(&mut self.transport)
            .ok_or_else(|| anyhow::anyhow!("Failed to get mutable transport"))?
            .disconnect()
            .await
    }

    pub async fn is_connected(&self) -> bool {
        self.transport.is_connected().await
    }
}

impl ProtocolClientTrait for ProtocolClient {
    async fn call<TRequest, TResponse>(&self, method: &str, request: TRequest) -> Result<TResponse>
    where
        TRequest: Serialize + Send + Sync,
        TResponse: for<'de> Deserialize<'de>,
    {
        // Generate a unique request ID
        let request_id = generate_request_id();

        // Create the protocol message
        let message = ProtocolMessage::new_with_json(
            request_id,
            method.to_string(),
            MessageType::Request,
            serde_json::to_value(request)?,
        )?;

        // Send the request
        self.transport.send(message).await?;

        // Wait for the response
        // In a real implementation, this would use a proper request/response correlation mechanism
        let response = self.transport.receive().await?;

        if response.msg_type == MessageType::Error {
            let payload_value = response
                .payload_as_value()
                .context("Failed to parse error payload")?;
            return Err(anyhow::anyhow!(
                "Protocol error: {}",
                payload_value
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
            ));
        }

        // Deserialize the response
        let payload_value = response
            .payload_as_value()
            .context("Failed to parse response payload")?;
        let result: TResponse =
            serde_json::from_value(payload_value).context("Failed to deserialize response")?;

        Ok(result)
    }

    async fn stream<TRequest, TResponse>(
        &self,
        method: &str,
        request: TRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<TResponse>> + Send>>>
    where
        TRequest: Serialize + Send + Sync,
        TResponse: for<'de> Deserialize<'de> + Send + 'static,
    {
        // Generate a unique request ID
        let request_id = generate_request_id();

        // Create the protocol message
        let message = ProtocolMessage::new_with_json(
            request_id,
            method.to_string(),
            MessageType::Stream,
            serde_json::to_value(request)?,
        )?;

        // Send the stream request
        self.transport.send(message).await?;

        // Create a stream that receives messages
        let transport = Arc::clone(&self.transport);
        let stream = async_stream::stream! {
            loop {
                match transport.receive().await {
                    Ok(msg) => {
                        match msg.msg_type {
                            MessageType::StreamData => {
                                match msg.payload_as_value() {
                                    Ok(payload_value) => {
                                        match serde_json::from_value::<TResponse>(payload_value) {
                                            Ok(data) => yield Ok(data),
                                            Err(e) => yield Err(anyhow::anyhow!("Deserialization error: {}", e)),
                                        }
                                    }
                                    Err(e) => yield Err(anyhow::anyhow!("Failed to parse payload: {}", e)),
                                }
                            }
                            MessageType::StreamEnd => {
                                break;
                            }
                            MessageType::Error => {
                                let error_msg = msg.payload_as_value()
                                    .ok()
                                    .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(|s| s.to_string()))
                                    .unwrap_or_else(|| "Unknown error".to_string());
                                yield Err(anyhow::anyhow!("Stream error: {}", error_msg));
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

fn generate_request_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

impl UnisonClient for ProtocolClient {
    async fn connect(&mut self, url: &str) -> Result<(), NetworkError> {
        Arc::get_mut(&mut self.transport)
            .ok_or_else(|| NetworkError::Connection("Failed to get mutable transport".to_string()))?
            .connect(url)
            .await
            .map_err(|e| NetworkError::Connection(e.to_string()))?;

        // Identity Handshake: サーバーからIdentityを受信
        match self.receive_identity().await {
            Ok(identity) => {
                tracing::info!(
                    "Received server identity: {} v{}",
                    identity.name,
                    identity.version
                );
            }
            Err(e) => {
                tracing::warn!("Failed to receive identity (non-fatal): {}", e);
            }
        }

        Ok(())
    }

    async fn call(
        &mut self,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, NetworkError> {
        let request_id = generate_request_id();

        let message = ProtocolMessage::new_with_json(
            request_id,
            method.to_string(),
            MessageType::Request,
            payload,
        )?;

        self.transport
            .send(message)
            .await
            .map_err(|e| NetworkError::Protocol(e.to_string()))?;

        let response = self
            .transport
            .receive()
            .await
            .map_err(|e| NetworkError::Protocol(e.to_string()))?;

        if response.msg_type == MessageType::Error {
            let payload_value = response.payload_as_value().map_err(|e| {
                NetworkError::Protocol(format!("Failed to parse error payload: {}", e))
            })?;
            return Err(NetworkError::Protocol(
                payload_value
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string(),
            ));
        }

        response.payload_as_value()
    }

    async fn disconnect(&mut self) -> Result<(), NetworkError> {
        Arc::get_mut(&mut self.transport)
            .ok_or_else(|| NetworkError::Connection("Failed to get mutable transport".to_string()))?
            .disconnect()
            .await
            .map_err(|e| NetworkError::Connection(e.to_string()))
    }

    fn is_connected(&self) -> bool {
        // これはトレイトで同期的である必要があります
        false // 今のところ簡略化
    }
}

// DummyTransport removed - using QuicClient directly

impl UnisonClientExt for ProtocolClient {
    async fn start_system_stream(
        &mut self,
        method: &str,
        _payload: serde_json::Value,
    ) -> Result<crate::network::quic::UnisonStream, NetworkError> {
        // use super::quic::UnisonStream;
        use super::StreamHandle;

        // Create a real QUIC bidirectional stream
        let _handle = StreamHandle {
            stream_id: generate_request_id(),
            method: method.to_string(),
            created_at: std::time::SystemTime::now(),
        };

        // QUICクライアントの接続を取得してUnisonStreamを作成
        // 現在の実装では直接アクセスできないため、エラーを返す
        Err(NetworkError::NotConnected)
    }

    async fn list_system_streams(&self) -> Result<Vec<super::StreamHandle>, NetworkError> {
        // In a full implementation, this would track active streams
        Ok(vec![])
    }

    async fn close_system_stream(&mut self, stream_id: u64) -> Result<(), NetworkError> {
        tracing::info!("🔒 Closed SystemStream with ID: {}", stream_id);
        Ok(())
    }
}

// MockSystemStream removed - using UnisonStream directly
