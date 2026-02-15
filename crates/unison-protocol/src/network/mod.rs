use anyhow::Result;
use futures_util::Stream;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use thiserror::Error;

use crate::packet::{RkyvPayload, SerializationError, UnisonPacket};

pub mod client;
pub mod quic;
pub mod server;
pub mod service;
pub mod identity;
pub mod channel;

pub use client::ProtocolClient;
pub use quic::{QuicClient, QuicServer, UnisonStream};
pub use server::ProtocolServer;
pub use service::{
    RealtimeService, Service, ServiceConfig, ServicePriority, ServiceStats, UnisonService,
};

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
    Stream,
    StreamData,
    StreamEnd,
    StreamError,
    // 双方向ストリーミング種別
    BidirectionalStream,
    StreamSend,
    StreamReceive,
    Error,
}

/// プロトコルエラー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolError {
    pub code: i32,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// プロトコル呼び出し用クライアントトレイト (Rust 2024対応)
pub trait ProtocolClientTrait: Send + Sync {
    /// 単項RPC呼び出しの実行
    fn call<TRequest, TResponse>(
        &self,
        method: &str,
        request: TRequest,
    ) -> impl std::future::Future<Output = Result<TResponse>> + Send
    where
        TRequest: Serialize + Send + Sync,
        TResponse: for<'de> Deserialize<'de>;

    /// ストリーミングRPC呼び出しの開始
    fn stream<TRequest, TResponse>(
        &self,
        method: &str,
        request: TRequest,
    ) -> impl std::future::Future<
        Output = Result<Pin<Box<dyn Stream<Item = Result<TResponse>> + Send>>>,
    > + Send
    where
        TRequest: Serialize + Send + Sync,
        TResponse: for<'de> Deserialize<'de> + Send + 'static;
}

/// プロトコルリクエスト処理用サーバートレイト (Rust 2024対応)
pub trait ProtocolServerTrait: Send + Sync {
    /// 単項RPC呼び出しの処理
    fn handle_call(
        &self,
        method: &str,
        payload: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value>> + Send;

    /// ストリーミングRPC呼び出しの処理
    fn handle_stream(
        &self,
        method: &str,
        payload: serde_json::Value,
    ) -> impl std::future::Future<
        Output = Result<Pin<Box<dyn Stream<Item = Result<serde_json::Value>> + Send>>>,
    > + Send;
}

/// Unison Protocolクライアントトレイト (Rust 2024対応)
pub trait UnisonClient: Send + Sync {
    /// Unisonサーバーへの接続
    fn connect(
        &mut self,
        url: &str,
    ) -> impl std::future::Future<Output = Result<(), NetworkError>> + Send;

    /// リモートプロシージャ呼び出しの実行
    fn call(
        &mut self,
        method: &str,
        payload: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, NetworkError>> + Send;

    /// サーバーからの切断
    fn disconnect(&mut self) -> impl std::future::Future<Output = Result<(), NetworkError>> + Send;

    /// クライアント接続状態の確認
    fn is_connected(&self) -> bool;
}

/// Unison Protocolサーバートレイト (Rust 2024対応)
pub trait UnisonServer: Send + Sync {
    /// 接続の待ち受け開始
    fn listen(
        &mut self,
        addr: &str,
    ) -> impl std::future::Future<Output = Result<(), NetworkError>> + Send;

    /// サーバーの停止
    fn stop(&mut self) -> impl std::future::Future<Output = Result<(), NetworkError>> + Send;

    /// サーバー実行状態の確認
    fn is_running(&self) -> bool;
}

/// ハンドラー登録トレイト（ジェネリクスのためdyn非互換）
pub trait UnisonServerExt: UnisonServer {
    /// 特定メソッド用ハンドラーの登録
    fn register_handler<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, NetworkError> + Send + Sync + 'static;

    /// 特定メソッド用ストリームハンドラーの登録
    fn register_stream_handler<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(
                serde_json::Value,
            )
                -> Pin<Box<dyn Stream<Item = Result<serde_json::Value, NetworkError>> + Send>>
            + Send
            + Sync
            + 'static;

    /// 双方向ストリーミング用SystemStreamハンドラーの登録
    fn register_system_stream_handler<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(
                serde_json::Value,
                crate::network::quic::UnisonStream,
            )
                -> Pin<Box<dyn futures_util::Future<Output = Result<(), NetworkError>> + Send>>
            + Send
            + Sync
            + 'static;
}

/// SystemStream - QUIC用双方向ストリームトレイト (Rust 2024対応)
pub trait SystemStream: Send + Sync {
    /// ストリームでのデータ送信
    fn send(
        &mut self,
        data: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<(), NetworkError>> + Send;

    /// ストリームからのデータ受信
    fn receive(
        &mut self,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, NetworkError>> + Send;

    /// ストリーム稼働状態の確認
    fn is_active(&self) -> bool;

    /// ストリームの終了
    fn close(&mut self) -> impl std::future::Future<Output = Result<(), NetworkError>> + Send;

    /// ストリームメタデータの取得
    fn get_handle(&self) -> StreamHandle;
}

// SystemStreamWrapper削除 - UnisonStreamを直接使用

// ServiceWrapper削除 - UnisonServiceを直接使用

/// 双方向ストリーム管理用ストリームハンドル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamHandle {
    pub stream_id: u64,
    pub method: String,
    pub created_at: std::time::SystemTime,
}

/// SystemStreamサポート付き拡張クライアントトレイト (Rust 2024対応)
pub trait UnisonClientExt: UnisonClient {
    /// 双方向SystemStreamの開始
    fn start_system_stream(
        &mut self,
        method: &str,
        payload: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<crate::network::quic::UnisonStream, NetworkError>> + Send;

    /// アクティブなSystemStreamの一覧
    fn list_system_streams(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<StreamHandle>, NetworkError>> + Send;

    /// ID指定によるSystemStreamの終了
    fn close_system_stream(
        &mut self,
        stream_id: u64,
    ) -> impl std::future::Future<Output = Result<(), NetworkError>> + Send;
}
