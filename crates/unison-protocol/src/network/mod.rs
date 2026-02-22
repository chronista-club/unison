use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::packet::{RkyvPayload, SerializationError, UnisonPacket};

pub mod channel;
pub mod client;
pub mod context;
pub mod identity;
pub mod quic;
pub mod server;

pub use channel::UnisonChannel;
pub use client::ProtocolClient;
pub use quic::{QuicClient, QuicServer, TypedFrame, UnisonStream};
pub use server::{ConnectionEvent, ProtocolServer, ServerHandle};

/// Unison Protocolのネットワークエラー
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Frame serialization error: {0}")]
    FrameSerialization(#[from] SerializationError),
    #[error("QUIC error: {0}")]
    Quic(String),
    #[error("Timeout error")]
    Timeout,
    #[error("Handler not found for method: {method}")]
    HandlerNotFound { method: String },
    #[error("Not connected")]
    NotConnected,
    #[error("Unsupported transport: {0}")]
    UnsupportedTransport(String),
}

/// プロトコルメッセージラッパー
#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[archive(check_bytes)]
pub struct ProtocolMessage {
    pub id: u64,
    pub method: String,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub payload: String, // JSON文字列として保持してrkyv互換に
}

/// フレームでラップされたプロトコルメッセージの型エイリアス
pub type ProtocolFrame = UnisonPacket<RkyvPayload<ProtocolMessage>>;

impl ProtocolMessage {
    /// ProtocolMessageをフレームに変換
    pub fn into_frame(self) -> Result<ProtocolFrame, SerializationError> {
        let payload = RkyvPayload::new(self);
        UnisonPacket::new(payload)
    }

    /// フレームからProtocolMessageを復元
    pub fn from_frame(frame: &ProtocolFrame) -> Result<Self, SerializationError> {
        let payload = frame.payload()?;
        Ok(payload.data.clone())
    }

    /// JSON文字列からprotocolメッセージを作成
    pub fn new_with_json(
        id: u64,
        method: String,
        msg_type: MessageType,
        payload: serde_json::Value,
    ) -> Result<Self, NetworkError> {
        Ok(Self {
            id,
            method,
            msg_type,
            payload: serde_json::to_string(&payload)?,
        })
    }

    /// payloadをserde_json::Valueとして取得
    pub fn payload_as_value(&self) -> Result<serde_json::Value, NetworkError> {
        Ok(serde_json::from_str(&self.payload)?)
    }
}

/// メッセージ種別
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Request,
    Response,
    /// 一方向プッシュ（応答不要）
    Event,
    Error,
}

/// プロトコルエラー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolError {
    pub code: i32,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

