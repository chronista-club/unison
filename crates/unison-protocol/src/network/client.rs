use anyhow::Result;
use std::sync::Arc;

use super::channel::UnisonChannel;
use super::context::ConnectionContext;
use super::identity::ServerIdentity;
use super::quic::{FRAME_TYPE_PROTOCOL, QuicClient, UnisonStream, write_typed_frame};
use super::{MessageType, NetworkError, ProtocolMessage};

/// QUIC protocol client implementation
pub struct ProtocolClient {
    transport: Arc<QuicClient>,
    /// 接続コンテキスト（Identity情報・チャネル状態）
    context: Arc<ConnectionContext>,
}

impl ProtocolClient {
    pub fn new(transport: QuicClient) -> Self {
        Self {
            transport: Arc::new(transport),
            context: Arc::new(ConnectionContext::new()),
        }
    }

    /// Create a new client with QUIC transport
    pub fn new_default() -> Result<Self> {
        let transport = QuicClient::new()?;
        Ok(Self {
            transport: Arc::new(transport),
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
        write_typed_frame(&mut send_stream, FRAME_TYPE_PROTOCOL, &frame_bytes)
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
                direction: super::identity::ChannelDirection::Bidirectional,
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

    /// Unisonサーバーへの接続（Identity Handshake 含む）
    pub async fn connect(&self, url: &str) -> Result<(), NetworkError> {
        self.transport
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

    /// サーバーからの切断
    pub async fn disconnect(&self) -> Result<(), NetworkError> {
        self.transport
            .disconnect()
            .await
            .map_err(|e| NetworkError::Connection(e.to_string()))
    }

    /// クライアント接続状態の確認
    pub async fn is_connected(&self) -> bool {
        self.transport.is_connected().await
    }
}

fn generate_request_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}
