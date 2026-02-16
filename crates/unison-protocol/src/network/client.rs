use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::channel::UnisonChannel;
use super::context::ConnectionContext;
use super::identity::ServerIdentity;
use super::quic::{QuicClient, UnisonStream, write_frame};
use super::service::Service;
use super::{MessageType, NetworkError, ProtocolMessage, UnisonClient};

/// QUIC protocol client implementation
pub struct ProtocolClient {
    transport: Arc<QuicClient>,
    services: Arc<RwLock<HashMap<String, crate::network::service::UnisonService>>>,
    /// 接続コンテキスト（Identity情報・チャネル状態）
    context: Arc<ConnectionContext>,
}

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

    /// チャネルを開く（UnisonChannel を返す）
    ///
    /// `__channel:{name}` メソッドで新しいQUICストリームを開き、
    /// `UnisonChannel` でラップして返す。
    pub async fn open_channel(
        &self,
        channel_name: &str,
    ) -> Result<UnisonChannel, NetworkError> {
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
            MessageType::Request,
            serde_json::json!({}),
        )?;

        let frame = message.into_frame().map_err(|e| {
            NetworkError::Protocol(format!("Failed to create channel frame: {}", e))
        })?;
        let frame_bytes = frame.to_bytes();
        write_frame(&mut send_stream, &frame_bytes)
            .await
            .map_err(|e| NetworkError::Protocol(format!("Failed to send channel open: {}", e)))?;

        // UnisonStreamを作成してUnisonChannelでラップ
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

        Ok(UnisonChannel::new(stream))
    }

    /// 接続後にサーバーからIdentityを受信する
    async fn receive_identity(&self) -> Result<ServerIdentity, NetworkError> {
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
    pub async fn connect_with_channels(
        &mut self,
        url: &str,
        channel_names: &[&str],
    ) -> Result<Vec<String>, NetworkError> {
        UnisonClient::connect(self, url).await?;

        let mut opened = Vec::new();
        for name in channel_names {
            self.context
                .register_channel(super::context::ChannelHandle {
                    channel_name: name.to_string(),
                    stream_id: 0,
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

    async fn disconnect(&mut self) -> Result<(), NetworkError> {
        Arc::get_mut(&mut self.transport)
            .ok_or_else(|| NetworkError::Connection("Failed to get mutable transport".to_string()))?
            .disconnect()
            .await
            .map_err(|e| NetworkError::Connection(e.to_string()))
    }

    fn is_connected(&self) -> bool {
        false
    }
}
