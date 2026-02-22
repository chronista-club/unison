//! ConnectionContext: QUIC接続ごとの状態管理
//!
//! 各接続に対して、Identity情報とアクティブチャネルを追跡する。
//! 複数のストリームハンドラーから並行アクセスされるため Arc<RwLock<>> で保護。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::identity::{ChannelDirection, ServerIdentity};

/// 接続ごとの状態を管理する構造体
#[derive(Debug)]
pub struct ConnectionContext {
    /// 接続の一意識別子
    pub connection_id: Uuid,
    /// サーバーから受信したIdentity情報
    identity: Arc<RwLock<Option<ServerIdentity>>>,
    /// アクティブなチャネルのマップ（チャネル名 → ハンドル）
    channels: Arc<RwLock<HashMap<String, ChannelHandle>>>,
}

/// チャネルのメタデータ
#[derive(Debug, Clone)]
pub struct ChannelHandle {
    pub channel_name: String,
    pub stream_id: u64,
    pub direction: ChannelDirection,
}

impl ConnectionContext {
    /// 新しいConnectionContextを作成
    pub fn new() -> Self {
        Self {
            connection_id: Uuid::new_v4(),
            identity: Arc::new(RwLock::new(None)),
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Identity情報を設定
    pub async fn set_identity(&self, identity: ServerIdentity) {
        let mut guard = self.identity.write().await;
        *guard = Some(identity);
    }

    /// Identity情報を取得
    pub async fn identity(&self) -> Option<ServerIdentity> {
        self.identity.read().await.clone()
    }

    /// チャネルを登録
    pub async fn register_channel(&self, handle: ChannelHandle) {
        let mut channels = self.channels.write().await;
        channels.insert(handle.channel_name.clone(), handle);
    }

    /// チャネルを取得
    pub async fn get_channel(&self, name: &str) -> Option<ChannelHandle> {
        let channels = self.channels.read().await;
        channels.get(name).cloned()
    }

    /// チャネルを削除
    pub async fn remove_channel(&self, name: &str) -> Option<ChannelHandle> {
        let mut channels = self.channels.write().await;
        channels.remove(name)
    }

    /// 全チャネル名を取得
    pub async fn channel_names(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.keys().cloned().collect()
    }
}

impl Default for ConnectionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_context_creation() {
        let ctx = ConnectionContext::new();
        assert!(ctx.identity().await.is_none());
        assert!(ctx.channel_names().await.is_empty());
    }

    #[tokio::test]
    async fn test_identity_set_and_get() {
        let ctx = ConnectionContext::new();
        let identity = ServerIdentity::new("test-server", "0.1.0", "test");
        ctx.set_identity(identity.clone()).await;

        let retrieved = ctx.identity().await.unwrap();
        assert_eq!(retrieved.name, "test-server");
        assert_eq!(retrieved.version, "0.1.0");
    }

    #[tokio::test]
    async fn test_channel_registration() {
        let ctx = ConnectionContext::new();

        let handle = ChannelHandle {
            channel_name: "events".to_string(),
            stream_id: 1,
            direction: ChannelDirection::ServerToClient,
        };
        ctx.register_channel(handle).await;

        let retrieved = ctx.get_channel("events").await.unwrap();
        assert_eq!(retrieved.stream_id, 1);
        assert_eq!(retrieved.direction, ChannelDirection::ServerToClient);

        let names = ctx.channel_names().await;
        assert_eq!(names, vec!["events"]);
    }

    #[tokio::test]
    async fn test_channel_removal() {
        let ctx = ConnectionContext::new();

        let handle = ChannelHandle {
            channel_name: "control".to_string(),
            stream_id: 2,
            direction: ChannelDirection::Bidirectional,
        };
        ctx.register_channel(handle).await;

        let removed = ctx.remove_channel("control").await;
        assert!(removed.is_some());
        assert!(ctx.get_channel("control").await.is_none());
    }
}
