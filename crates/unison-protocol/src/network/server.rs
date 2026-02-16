use anyhow::Result;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::identity::{ChannelDirection, ChannelInfo, ChannelStatus, ServerIdentity};
use super::service::Service;
use super::{NetworkError, UnisonServer};

/// チャネルハンドラー型（接続コンテキスト + UnisonStreamを受け取る）
pub type ChannelHandler = Arc<
    dyn Fn(
            Arc<super::context::ConnectionContext>,
            super::quic::UnisonStream,
        ) -> Pin<Box<dyn futures_util::Future<Output = Result<(), NetworkError>> + Send>>
        + Send
        + Sync,
>;

/// プロトコルサーバー実装
pub struct ProtocolServer {
    services: Arc<RwLock<HashMap<String, crate::network::service::UnisonService>>>,
    running: Arc<RwLock<bool>>,
    /// サーバー識別情報
    server_name: String,
    server_version: String,
    server_namespace: String,
    /// チャネルハンドラー（チャネル名 → ハンドラー関数）
    channel_handlers: Arc<RwLock<HashMap<String, ChannelHandler>>>,
}

impl ProtocolServer {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            server_name: "unison".to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            server_namespace: "default".to_string(),
            channel_handlers: Arc::new(RwLock::new(HashMap::new())),
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

    /// チャネルハンドラーを取得
    pub async fn get_channel_handler(&self, name: &str) -> Option<ChannelHandler> {
        let handlers = self.channel_handlers.read().await;
        handlers.get(name).cloned()
    }

    /// サーバーにサービスインスタンスを登録
    pub async fn register_service(&self, service: crate::network::service::UnisonService) {
        let service_name = service.service_name().to_string();
        let mut services = self.services.write().await;
        services.insert(service_name, service);
    }

    /// 登録されたサービスリストを取得
    pub async fn list_services(&self) -> Vec<String> {
        let services = self.services.read().await;
        services.keys().cloned().collect()
    }

    /// 登録されたサービスへのルーティングによるサービスリクエストの処理
    pub async fn handle_service_request(
        &self,
        service_name: &str,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_name) {
            service
                .handle_request(method, payload)
                .await
                .map_err(|e| anyhow::anyhow!("Service error: {}", e))
        } else {
            Err(anyhow::anyhow!("Service not found: {}", service_name))
        }
    }

}

impl Default for ProtocolServer {
    fn default() -> Self {
        Self::new()
    }
}

impl UnisonServer for ProtocolServer {
    async fn listen(&mut self, addr: &str) -> Result<(), NetworkError> {
        use super::quic::QuicServer;

        // 実行状態を設定
        {
            let mut running = self.running.write().await;
            *running = true;
        }

        // プロトコルハンドラーとして自分自身を使用してQUICサーバーを作成
        let protocol_server = Arc::new(ProtocolServer {
            services: Arc::clone(&self.services),
            running: Arc::clone(&self.running),
            server_name: self.server_name.clone(),
            server_version: self.server_version.clone(),
            server_namespace: self.server_namespace.clone(),
            channel_handlers: Arc::clone(&self.channel_handlers),
        });

        let mut quic_server = QuicServer::new(protocol_server);
        quic_server
            .bind(addr)
            .await
            .map_err(|e| NetworkError::Quic(e.to_string()))?;

        tracing::info!("🎵 Unison Protocol server listening on {} via QUIC", addr);

        quic_server
            .start()
            .await
            .map_err(|e| NetworkError::Quic(e.to_string()))?;

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), NetworkError> {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("🎵 Unison Protocol server stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        false
    }
}

/// ProtocolServerのサービス管理拡張
impl ProtocolServer {
    /// 自動起動でサービスを登録
    pub async fn register_and_start_service(
        &self,
        mut service: crate::network::service::UnisonService,
    ) -> Result<String, NetworkError> {
        let service_name = service.service_name().to_string();

        // 設定されている場合はサービスハートビートを開始
        service.start_service_heartbeat(30).await?;

        // サービスを登録
        self.register_service(service).await;

        tracing::info!("🎵 Service '{}' registered and started", service_name);
        Ok(service_name)
    }

    /// すべてのサービスを正常に停止
    pub async fn shutdown_all_services(&self) -> Result<(), NetworkError> {
        let mut services = self.services.write().await;

        for (name, service) in services.iter_mut() {
            tracing::info!("🛑 Shutting down service: {}", name);
            if let Err(e) = service.shutdown().await {
                tracing::error!("Error shutting down service {}: {}", name, e);
            }
        }

        services.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
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

        assert!(server.list_services().await.is_empty());
    }
}
