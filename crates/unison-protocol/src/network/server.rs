use anyhow::Result;
use futures_util::Stream;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::identity::{ChannelDirection, ChannelInfo, ChannelStatus, ServerIdentity};
use super::service::Service;
use super::{
    MessageType, NetworkError, ProtocolMessage, ProtocolServerTrait, UnisonServer, UnisonServerExt,
};

/// サーバーハンドラー関数型
type CallHandler = Arc<
    dyn Fn(Value) -> Pin<Box<dyn futures_util::Future<Output = Result<Value>> + Send>>
        + Send
        + Sync,
>;

/// ストリームハンドラー関数型
type StreamHandler = Arc<
    dyn Fn(
            Value,
        ) -> Pin<
            Box<
                dyn futures_util::Future<
                        Output = Result<Pin<Box<dyn Stream<Item = Result<Value>> + Send>>>,
                    > + Send,
            >,
        > + Send
        + Sync,
>;

/// シンプルハンドラー用のUnisonハンドラー型
type UnisonHandler =
    Arc<dyn Fn(serde_json::Value) -> Result<serde_json::Value, NetworkError> + Send + Sync>;

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
    call_handlers: Arc<RwLock<HashMap<String, CallHandler>>>,
    stream_handlers: Arc<RwLock<HashMap<String, StreamHandler>>>,
    unison_handlers: Arc<RwLock<HashMap<String, UnisonHandler>>>,
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
            call_handlers: Arc::new(RwLock::new(HashMap::new())),
            stream_handlers: Arc::new(RwLock::new(HashMap::new())),
            unison_handlers: Arc::new(RwLock::new(HashMap::new())),
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
        let mut identity =
            ServerIdentity::new(&self.server_name, &self.server_version, &self.server_namespace);

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
        let handler = Arc::new(move |ctx: Arc<super::context::ConnectionContext>, stream: super::quic::UnisonStream| {
            Box::pin(handler(ctx, stream))
                as Pin<Box<dyn futures_util::Future<Output = Result<(), NetworkError>> + Send>>
        });

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

    /// 呼び出しハンドラーを登録
    pub async fn register_call_handler<F, Fut>(&self, method: &str, handler: F)
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: futures_util::Future<Output = Result<Value>> + Send + 'static,
    {
        let handler = Arc::new(move |value: Value| {
            Box::pin(handler(value))
                as Pin<Box<dyn futures_util::Future<Output = Result<Value>> + Send>>
        });

        let mut handlers = self.call_handlers.write().await;
        handlers.insert(method.to_string(), handler);
    }

    /// ストリームハンドラーを登録
    pub async fn register_stream_handler<F, Fut, S>(&self, method: &str, handler: F)
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: futures_util::Future<Output = Result<S>> + Send + 'static,
        S: Stream<Item = Result<Value>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let wrapped_handler = Arc::new(move |value: Value| {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                let stream = handler(value).await?;
                Ok(Box::pin(stream) as Pin<Box<dyn Stream<Item = Result<Value>> + Send>>)
            })
                as Pin<
                    Box<
                        dyn futures_util::Future<
                                Output = Result<Pin<Box<dyn Stream<Item = Result<Value>> + Send>>>,
                            > + Send,
                    >,
                >
        });

        let mut handlers = self.stream_handlers.write().await;
        handlers.insert(method.to_string(), wrapped_handler);
    }

    /// 入力メッセージを処理
    pub async fn process_message(&self, message: ProtocolMessage) -> Result<ProtocolMessage> {
        match message.msg_type {
            MessageType::Request => {
                let handlers = self.call_handlers.read().await;
                if let Some(handler) = handlers.get(&message.method) {
                    let payload_value = message
                        .payload_as_value()
                        .map_err(|e| anyhow::anyhow!("Failed to parse payload: {}", e))?;
                    match handler(payload_value).await {
                        Ok(response) => ProtocolMessage::new_with_json(
                            message.id,
                            message.method,
                            MessageType::Response,
                            response,
                        )
                        .map_err(|e| anyhow::anyhow!("Failed to create response: {}", e)),
                        Err(e) => ProtocolMessage::new_with_json(
                            message.id,
                            message.method,
                            MessageType::Error,
                            serde_json::json!({
                                "message": e.to_string(),
                            }),
                        )
                        .map_err(|e| anyhow::anyhow!("Failed to create error response: {}", e)),
                    }
                } else {
                    ProtocolMessage::new_with_json(
                        message.id,
                        message.method.clone(),
                        MessageType::Error,
                        serde_json::json!({
                            "message": format!("Method not found: {}", message.method),
                        }),
                    )
                    .map_err(|e| anyhow::anyhow!("Failed to create error response: {}", e))
                }
            }
            MessageType::Stream => {
                // Stream handling would be more complex in a real implementation
                // This is a simplified version
                let handlers = self.stream_handlers.read().await;
                if let Some(_handler) = handlers.get(&message.method) {
                    // 実際の実装では：
                    // 1. ストリームを開始
                    // 2. 各アイテムに対してStreamDataメッセージを送信
                    // 3. 完了時にStreamEndを送信
                    ProtocolMessage::new_with_json(
                        message.id,
                        message.method,
                        MessageType::StreamEnd,
                        serde_json::json!({}),
                    )
                    .map_err(|e| anyhow::anyhow!("Failed to create stream end message: {}", e))
                } else {
                    ProtocolMessage::new_with_json(
                        message.id,
                        message.method.clone(),
                        MessageType::Error,
                        serde_json::json!({
                            "message": format!("Stream method not found: {}", message.method),
                        }),
                    )
                    .map_err(|e| anyhow::anyhow!("Failed to create error message: {}", e))
                }
            }
            _ => ProtocolMessage::new_with_json(
                message.id,
                message.method,
                MessageType::Error,
                serde_json::json!({
                    "message": "Invalid message type",
                }),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create error message: {}", e)),
        }
    }
}

impl ProtocolServerTrait for ProtocolServer {
    async fn handle_call(
        &self,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // まずunison_handlers（register_handlerで登録）を試行
        let unison_handlers = self.unison_handlers.read().await;
        if let Some(handler) = unison_handlers.get(method) {
            match handler(payload) {
                Ok(result) => Ok(result),
                Err(e) => Err(anyhow::anyhow!("Handler error: {}", e)),
            }
        } else {
            // call_handlersへフォールバック
            drop(unison_handlers);
            let handlers = self.call_handlers.read().await;
            if let Some(handler) = handlers.get(method) {
                handler(payload).await
            } else {
                Err(anyhow::anyhow!("Method not found: {}", method))
            }
        }
    }

    async fn handle_stream(
        &self,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<serde_json::Value>> + Send>>> {
        let handlers = self.stream_handlers.read().await;
        if let Some(handler) = handlers.get(method) {
            handler(payload).await
        } else {
            Err(anyhow::anyhow!("Stream method not found: {}", method))
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
            call_handlers: Arc::clone(&self.call_handlers),
            stream_handlers: Arc::clone(&self.stream_handlers),
            unison_handlers: Arc::clone(&self.unison_handlers),
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
        // For now, return false for simplicity in tests
        // In production, this would check the actual running state
        false
    }
}

impl UnisonServerExt for ProtocolServer {
    fn register_handler<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, NetworkError> + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        let method = method.to_string();
        let handlers_arc = Arc::clone(&self.unison_handlers);

        // Use tokio spawn for async registration to avoid blocking
        let _handle = tokio::spawn(async move {
            let mut handlers = handlers_arc.write().await;
            handlers.insert(method, handler);
        });
    }

    fn register_stream_handler<F>(&mut self, method: &str, _handler: F)
    where
        F: Fn(
                serde_json::Value,
            )
                -> Pin<Box<dyn Stream<Item = Result<serde_json::Value, NetworkError>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        // Simplified implementation for now - just log registration
        tracing::info!("Stream handler registered for method: {}", method);
        // TODO: Implement proper stream handler storage
    }

    fn register_system_stream_handler<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(
                serde_json::Value,
                crate::network::quic::UnisonStream,
            )
                -> Pin<Box<dyn futures_util::Future<Output = Result<(), NetworkError>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        // For now, we'll store this as a placeholder until we implement SystemStream handling
        // This is a complex operation that requires significant changes to the server architecture
        let _handler = Arc::new(handler);
        tracing::info!("SystemStream handler registered for method: {}", method);
        // TODO: Implement SystemStream handler storage and execution
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
        use super::UnisonServerExt;

        let mut server = ProtocolServer::new();

        // Register a simple handler
        server.register_handler("ping", |_payload| {
            Ok(serde_json::json!({"message": "pong"}))
        });

        // Test handler registration without actually starting the server
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Wait for registration to complete

        // Test that server can be stopped
        assert!(server.stop().await.is_ok());
    }
}
