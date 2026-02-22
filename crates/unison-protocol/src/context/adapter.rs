//! 既存システムとCGPの統合アダプター

use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

use super::{
    CgpProtocolContext, Handler, HandlerRegistry, MessageHandler, ServiceRegistry, TransportLayer,
};
use crate::network::{MessageType, NetworkError, ProtocolClient, ProtocolMessage, ProtocolServer};
use crate::network::quic::{FRAME_TYPE_PROTOCOL, write_typed_frame};

// ========================================
// Transport Adapter
// ========================================

/// QuicClientをTransportLayerトレイトに適合させるアダプター
pub struct QuicTransportAdapter {
    client: Arc<crate::network::quic::QuicClient>,
    connected: Arc<AtomicBool>,
}

impl QuicTransportAdapter {
    pub fn new(client: crate::network::quic::QuicClient) -> Self {
        Self {
            client: Arc::new(client),
            connected: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl TransportLayer for QuicTransportAdapter {
    type Message = ProtocolMessage;
    type Error = NetworkError;

    async fn send(&self, message: Self::Message) -> Result<(), Self::Error> {
        let connection_guard = self.client.connection().read().await;
        let connection = connection_guard
            .as_ref()
            .ok_or(NetworkError::NotConnected)?;

        let (mut send_stream, _recv_stream) = connection
            .open_bi()
            .await
            .map_err(|e| NetworkError::Quic(format!("Failed to open stream: {}", e)))?;

        let frame = message
            .into_frame()
            .map_err(|e| NetworkError::Protocol(format!("Failed to create frame: {}", e)))?;
        let frame_bytes = frame.to_bytes();

        write_typed_frame(&mut send_stream, FRAME_TYPE_PROTOCOL, &frame_bytes)
            .await
            .map_err(|e| NetworkError::Quic(format!("Failed to send: {}", e)))?;

        Ok(())
    }

    async fn receive(&self) -> Result<Self::Message, Self::Error> {
        self.client
            .receive()
            .await
            .map_err(|e| NetworkError::Protocol(e.to_string()))
    }

    async fn connect(&self, url: &str) -> Result<(), Self::Error> {
        let result = self
            .client
            .connect(url)
            .await
            .map_err(|e| NetworkError::Connection(e.to_string()));
        if result.is_ok() {
            self.connected.store(true, Ordering::SeqCst);
        }
        result
    }

    async fn disconnect(&self) -> Result<(), Self::Error> {
        let result = self
            .client
            .disconnect()
            .await
            .map_err(|e| NetworkError::Connection(e.to_string()));
        self.connected.store(false, Ordering::SeqCst);
        result
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}

// ========================================
// Service Registry Adapter
// ========================================

/// 汎用ServiceRegistryアダプター（Value ベース）
pub struct ServiceRegistryAdapter {
    services: Arc<RwLock<std::collections::HashMap<String, Value>>>,
}

impl Default for ServiceRegistryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceRegistryAdapter {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

impl ServiceRegistry for ServiceRegistryAdapter {
    type Service = Value;
    type Error = NetworkError;

    async fn register(&self, name: String, service: Self::Service) -> Result<(), Self::Error> {
        let mut services = self.services.write().await;
        services.insert(name, service);
        Ok(())
    }

    async fn get(&self, name: &str) -> Option<Self::Service> {
        let services = self.services.read().await;
        services.get(name).cloned()
    }

    async fn list(&self) -> Vec<String> {
        let services = self.services.read().await;
        services.keys().cloned().collect()
    }

    async fn remove(&self, name: &str) -> Result<(), Self::Error> {
        let mut services = self.services.write().await;
        services.remove(name);
        Ok(())
    }
}

// ========================================
// CGP-Enhanced Client
// ========================================

/// CGPで強化されたプロトコルクライアント
pub struct CgpEnhancedClient<T, R, H>
where
    T: TransportLayer,
    R: ServiceRegistry,
    H: MessageHandler,
{
    context: CgpProtocolContext<T, R, H>,
    legacy_client: Option<ProtocolClient>,
}

impl<T, R, H> CgpEnhancedClient<T, R, H>
where
    T: TransportLayer<Message = ProtocolMessage, Error = NetworkError>,
    R: ServiceRegistry,
    H: MessageHandler,
{
    pub fn new(context: CgpProtocolContext<T, R, H>) -> Self {
        Self {
            context,
            legacy_client: None,
        }
    }

    /// レガシークライアントとの互換性を保つ
    pub fn with_legacy(mut self, client: ProtocolClient) -> Self {
        self.legacy_client = Some(client);
        self
    }

    /// CGPコンテキストを使ったメッセージ送信
    pub async fn send_message(&self, method: &str, payload: Value) -> Result<Value, NetworkError> {
        let message = ProtocolMessage::new_with_json(
            generate_id(),
            method.to_string(),
            MessageType::Request,
            payload,
        )?;

        self.context.transport().send(message.clone()).await?;
        let response = self.context.transport().receive().await?;

        response.payload_as_value()
    }
}

// ========================================
// CGP-Enhanced Server
// ========================================

/// CGPで強化されたプロトコルサーバー
pub struct CgpEnhancedServer {
    handler_registry: HandlerRegistry,
    legacy_server: Option<Arc<ProtocolServer>>,
}

impl Default for CgpEnhancedServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CgpEnhancedServer {
    pub fn new() -> Self {
        Self {
            handler_registry: HandlerRegistry::new(),
            legacy_server: None,
        }
    }

    /// レガシーサーバーとの互換性を保つ
    pub fn with_legacy(mut self, server: Arc<ProtocolServer>) -> Self {
        self.legacy_server = Some(server);
        self
    }

    /// CGPハンドラーを登録
    pub async fn register_cgp_handler(&self, method: &str, handler: impl Handler + 'static) {
        self.handler_registry.register(method, handler).await;
    }

    /// リクエストを処理
    pub async fn handle_request(&self, message: ProtocolMessage) -> Result<Value, NetworkError> {
        use super::MessageDispatcher;
        self.handler_registry.dispatch(message).await
    }

    /// 既存のProtocolServerハンドラーをCGPハンドラーに変換
    pub async fn migrate_legacy_handlers(&self, _server: &ProtocolServer) {
        // レガシーハンドラーをCGPハンドラーとして再登録
        // 実際の実装では、内部状態へのアクセスが必要
    }
}

// ========================================
// Bridge Implementations
// ========================================

/// レガシーハンドラーをCGPハンドラーに変換するブリッジ
pub struct LegacyHandlerBridge<F>
where
    F: Fn(Value) -> Result<Value, NetworkError> + Send + Sync,
{
    handler: F,
}

impl<F> LegacyHandlerBridge<F>
where
    F: Fn(Value) -> Result<Value, NetworkError> + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

impl<F> Handler for LegacyHandlerBridge<F>
where
    F: Fn(Value) -> Result<Value, NetworkError> + Send + Sync,
{
    fn handle(
        &self,
        payload: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, NetworkError>> + Send + '_>>
    {
        let result = (self.handler)(payload);
        Box::pin(async move { result })
    }
}

// ========================================
// Helper Functions
// ========================================

fn generate_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

// ========================================
// Migration Utilities
// ========================================

/// 既存のUnison ProtocolをCGPに段階的に移行するためのユーティリティ
pub struct MigrationHelper;

impl MigrationHelper {
    /// ProtocolClientをCGPクライアントに変換
    pub async fn migrate_client(
        client: ProtocolClient,
    ) -> CgpEnhancedClient<QuicTransportAdapter, ServiceRegistryAdapter, HandlerRegistry> {
        let transport = QuicTransportAdapter::new(crate::network::quic::QuicClient::new().unwrap());
        let registry = ServiceRegistryAdapter::new();
        let handler = HandlerRegistry::new();

        let context = CgpProtocolContext::new(transport, registry, handler);

        CgpEnhancedClient::new(context).with_legacy(client)
    }

    /// ProtocolServerをCGPサーバーに変換
    pub async fn migrate_server(server: Arc<ProtocolServer>) -> CgpEnhancedServer {
        let cgp_server = CgpEnhancedServer::new().with_legacy(server.clone());

        // レガシーハンドラーを移行
        cgp_server.migrate_legacy_handlers(&server).await;

        cgp_server
    }
}
