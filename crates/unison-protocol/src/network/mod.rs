use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::codec::{CodecError, Decodable, Encodable, JsonCodec};
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
pub use server::{ConnectionEvent, ConnectionEventReceiver, ProtocolServer, ServerHandle};

/// グローバルなリクエストID生成（モジュール間で一意）
pub(crate) fn generate_request_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Unison Protocolのネットワークエラー
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Codec error: {0}")]
    Codec(#[from] CodecError),
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
    pub payload: Vec<u8>, // Codec がエンコードしたバイト列
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

    /// エンコード済みバイト列から ProtocolMessage を直接作成
    pub fn new_encoded(
        id: u64,
        method: String,
        msg_type: MessageType,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            id,
            method,
            msg_type,
            payload,
        }
    }

    /// JSON でエンコードして ProtocolMessage を作成
    pub fn new_with_json(
        id: u64,
        method: String,
        msg_type: MessageType,
        payload: serde_json::Value,
    ) -> Result<Self, NetworkError> {
        let bytes =
            Encodable::<JsonCodec>::encode(&payload).map_err(NetworkError::Codec)?;
        Ok(Self::new_encoded(id, method, msg_type, bytes))
    }

    /// JSON で payload をデコード
    pub fn payload_as_value(&self) -> Result<serde_json::Value, NetworkError> {
        Ok(<serde_json::Value as Decodable<JsonCodec>>::decode(
            &self.payload,
        )?)
    }

    /// 任意の Codec + 型で payload をデコード
    pub fn decode_payload<T, C: crate::codec::Codec>(&self) -> Result<T, NetworkError>
    where
        T: Decodable<C>,
    {
        Ok(T::decode(&self.payload)?)
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
