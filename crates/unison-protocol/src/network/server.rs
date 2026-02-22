use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::identity::{ChannelDirection, ChannelInfo, ChannelStatus, ServerIdentity};
use super::NetworkError;

/// 接続イベント通知
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// 新しい接続が確立された
    Connected {
        remote_addr: SocketAddr,
        context: Arc<super::context::ConnectionContext>,
    },
    /// 接続が切断された
    Disconnected { remote_addr: SocketAddr },
}

/// チャネルハンドラー型（接続コンテキスト + UnisonStreamを受け取る）
pub type ChannelHandler = Arc<
    dyn Fn(
            Arc<super::context::ConnectionContext>,
            super::quic::UnisonStream,
        ) -> Pin<Box<dyn futures_util::Future<Output = Result<(), NetworkError>> + Send>>
        + Send
        + Sync,
>;

/// サーバーのライフサイクルを管理するハンドル
///
/// `spawn_listen()` が返す。shutdown シグナル送信と完了待ちを提供。
pub struct ServerHandle {
    join_handle: JoinHandle<Result<(), NetworkError>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    local_addr: SocketAddr,
}

impl ServerHandle {
    /// サーバーをグレースフルにシャットダウンし、完了を待つ
    pub async fn shutdown(mut self) -> Result<(), NetworkError> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.join_handle
            .await
            .map_err(|e| NetworkError::Quic(format!("Server task panicked: {}", e)))?
    }

    /// サーバータスクが終了済みかどうか
    pub fn is_finished(&self) -> bool {
        self.join_handle.is_finished()
    }

    /// サーバーがバインドしたローカルアドレスを取得
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

/// プロトコルサーバー実装
pub struct ProtocolServer {
    running: Arc<AtomicBool>,
    /// サーバー識別情報
    server_name: String,
    server_version: String,
    server_namespace: String,
    /// チャネルハンドラー（チャネル名 → ハンドラー関数）
    channel_handlers: Arc<RwLock<HashMap<String, ChannelHandler>>>,
    /// 接続イベント broadcast チャネル（複数サブスクライバ対応）
    connection_event_tx: tokio::sync::broadcast::Sender<ConnectionEvent>,
}

impl ProtocolServer {
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(64);
        Self {
            running: Arc::new(AtomicBool::new(false)),
            server_name: "unison".to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            server_namespace: "default".to_string(),
            channel_handlers: Arc::new(RwLock::new(HashMap::new())),
            connection_event_tx: tx,
        }
    }

    /// サーバー識別情報を設定して作成
    pub fn with_identity(name: &str, version: &str, namespace: &str) -> Self {
        Self {
            server_name: name.to_string(),
            server_version: version.to_string(),
            server_namespace: namespace.to_string(),
            ..Self::new()
        }
    }

    /// サーバー実行状態の確認
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 登録済みチャネルからServerIdentityを構築
    pub async fn build_identity(&self) -> ServerIdentity {
        let mut identity = ServerIdentity::new(
            &self.server_name,
            &self.server_version,
            &self.server_namespace,
        );

        // チャネルハンドラーからChannelInfoを構築
        let handlers = self.channel_handlers.read().await;
        for channel_name in handlers.keys() {
            identity.add_channel(ChannelInfo {
                name: channel_name.clone(),
                direction: ChannelDirection::Bidirectional,
                lifetime: "persistent".to_string(),
                status: ChannelStatus::Available,
            });
        }

        identity
    }

    /// チャネルハンドラーを登録
    pub async fn register_channel<F, Fut>(&self, name: &str, handler: F)
    where
        F: Fn(Arc<super::context::ConnectionContext>, super::quic::UnisonStream) -> Fut
            + Send
            + Sync
            + 'static,
        Fut: futures_util::Future<Output = Result<(), NetworkError>> + Send + 'static,
    {
        let handler = Arc::new(
            move |ctx: Arc<super::context::ConnectionContext>,
                  stream: super::quic::UnisonStream| {
                Box::pin(handler(ctx, stream))
                    as Pin<Box<dyn futures_util::Future<Output = Result<(), NetworkError>> + Send>>
            },
        );

        let mut handlers = self.channel_handlers.write().await;
        handlers.insert(name.to_string(), handler);
    }

    /// 接続イベントを購読する
    ///
    /// 接続/切断時に `ConnectionEvent` を受信できる。
    /// 複数のサブスクライバが同時に購読可能。
    pub fn subscribe_connection_events(
        &self,
    ) -> tokio::sync::broadcast::Receiver<ConnectionEvent> {
        self.connection_event_tx.subscribe()
    }

    /// 接続イベントを送信（内部用）
    pub(crate) fn emit_connection_event(&self, event: ConnectionEvent) {
        let _ = self.connection_event_tx.send(event);
    }

    /// チャネルハンドラーを取得
    pub async fn get_channel_handler(&self, name: &str) -> Option<ChannelHandler> {
        let handlers = self.channel_handlers.read().await;
        handlers.get(name).cloned()
    }

    /// 接続の待ち受け開始（self を消費してブロック）
    ///
    /// サーバーを起動し、接続を受け付ける。終了するまでブロックする。
    /// 非ブロッキングで起動する場合は `spawn_listen()` を使用する。
    pub async fn listen(self, addr: &str) -> Result<(), NetworkError> {
        use super::quic::QuicServer;

        let protocol_server = Arc::new(self);
        protocol_server.running.store(true, Ordering::SeqCst);

        let mut quic_server = QuicServer::new(Arc::clone(&protocol_server));
        quic_server
            .bind(addr)
            .await
            .map_err(|e| NetworkError::Quic(e.to_string()))?;

        tracing::info!("Unison Protocol server listening on {} via QUIC", addr);

        let result = quic_server
            .start()
            .await
            .map_err(|e| NetworkError::Quic(e.to_string()));

        protocol_server.running.store(false, Ordering::SeqCst);
        result
    }

    /// バックグラウンドでサーバーを起動し、ServerHandle を返す
    ///
    /// `ServerHandle::shutdown()` でグレースフルに停止できる。
    pub async fn spawn_listen(self, addr: &str) -> Result<ServerHandle, NetworkError> {
        use super::quic::QuicServer;

        let protocol_server = Arc::new(self);

        let mut quic_server = QuicServer::new(Arc::clone(&protocol_server));
        quic_server
            .bind(addr)
            .await
            .map_err(|e| NetworkError::Quic(e.to_string()))?;

        let local_addr = quic_server
            .local_addr()
            .ok_or_else(|| NetworkError::Quic("Server not bound".to_string()))?;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        protocol_server.running.store(true, Ordering::SeqCst);

        tracing::info!("Unison Protocol server spawned on {} via QUIC", local_addr);

        let server_clone = Arc::clone(&protocol_server);
        let join_handle = tokio::spawn(async move {
            let result = quic_server
                .start_with_shutdown(shutdown_rx)
                .await
                .map_err(|e| NetworkError::Quic(e.to_string()));

            server_clone.running.store(false, Ordering::SeqCst);

            result
        });

        Ok(ServerHandle {
            join_handle,
            shutdown_tx: Some(shutdown_tx),
            local_addr,
        })
    }
}

impl Default for ProtocolServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = ProtocolServer::new();
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_server_lifecycle() {
        let server = ProtocolServer::new();

        // チャネルハンドラーを登録
        server
            .register_channel("ping", |_ctx, _stream| async { Ok(()) })
            .await;

        // チャネルハンドラーが取得できること
        let handler = server.get_channel_handler("ping").await;
        assert!(handler.is_some());
    }
}
