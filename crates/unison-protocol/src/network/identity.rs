//! Identity Channel: QUICエンドポイントのリアルタイム自己紹介
//!
//! 各サーバーは接続時にServerIdentityを送信し、
//! チャネルの追加・削除・状態変更をリアルタイムに通知する。

use serde::{Deserialize, Serialize};

/// サーバーの自己紹介情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerIdentity {
    pub name: String,
    pub version: String,
    pub namespace: String,
    pub channels: Vec<ChannelInfo>,
    pub metadata: serde_json::Value,
}

/// チャネルの情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub name: String,
    pub direction: ChannelDirection,
    pub lifetime: String,
    pub status: ChannelStatus,
}

/// チャネルの方向
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelDirection {
    ServerToClient,
    ClientToServer,
    Bidirectional,
}

/// チャネルの状態
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelStatus {
    Available,
    Busy,
    Unavailable,
}

/// チャネルのリアルタイム更新
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "channel")]
pub enum ChannelUpdate {
    Added(ChannelInfo),
    Removed(String),
    StatusChanged { name: String, status: ChannelStatus },
}
