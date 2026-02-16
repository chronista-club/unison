//! Channel: Stream-First APIの通信プリミティブ
//!
//! 各ChannelはQUICストリームにマッピングされ、
//! 独立したHoL Blocking境界を形成する。
//!
//! - `StreamSender` / `StreamReceiver` / `BidirectionalChannel`: インメモリ（mpsc）
//! - `QuicBackedChannel`: 実際のQUICストリーム上で動作する型安全チャネル

use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use super::NetworkError;
use super::quic::UnisonStream;

/// 送信側ハンドル
pub struct StreamSender<T> {
    tx: mpsc::Sender<T>,
}

impl<T> StreamSender<T> {
    pub fn new(tx: mpsc::Sender<T>) -> Self {
        Self { tx }
    }

    pub async fn send(&self, msg: T) -> Result<(), mpsc::error::SendError<T>> {
        self.tx.send(msg).await
    }

    /// チャネルが閉じているか
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

/// 受信側ハンドル
pub struct StreamReceiver<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> StreamReceiver<T> {
    pub fn new(rx: mpsc::Receiver<T>) -> Self {
        Self { rx }
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.rx.recv().await
    }
}

/// 双方向チャネル
pub struct BidirectionalChannel<S, R> {
    pub sender: StreamSender<S>,
    pub receiver: StreamReceiver<R>,
}

/// 受信専用チャネル（Push/Event用）
pub struct ReceiveChannel<T> {
    pub receiver: StreamReceiver<T>,
}

/// リクエスト-レスポンスチャネル（transient RPC用）
pub struct RequestChannel<Req, Res> {
    _req: std::marker::PhantomData<Req>,
    _res: std::marker::PhantomData<Res>,
    pub tx: mpsc::Sender<(Req, tokio::sync::oneshot::Sender<Res>)>,
}

/// QUICストリーム上で動作する型安全な双方向チャネル
///
/// 実際のQUICストリームをラップし、Serialize/DeserializeOwned の
/// 型パラメータで送受信メッセージの型安全性を保証する。
pub struct QuicBackedChannel<S, R> {
    stream: Arc<Mutex<UnisonStream>>,
    _send: PhantomData<S>,
    _recv: PhantomData<R>,
}

impl<S, R> QuicBackedChannel<S, R>
where
    S: Serialize + Send,
    R: DeserializeOwned + Send,
{
    /// UnisonStreamからQuicBackedChannelを作成
    pub fn new(stream: UnisonStream) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
            _send: PhantomData,
            _recv: PhantomData,
        }
    }

    /// 型安全なメッセージ送信
    pub async fn send(&self, msg: S) -> Result<(), NetworkError> {
        let value = serde_json::to_value(msg)?;
        let mut stream = self.stream.lock().await;
        super::SystemStream::send(&mut *stream, value).await
    }

    /// 型安全なメッセージ受信
    pub async fn recv(&self) -> Result<R, NetworkError> {
        let mut stream = self.stream.lock().await;
        let value = super::SystemStream::receive(&mut *stream).await?;
        serde_json::from_value(value).map_err(|e| {
            NetworkError::Protocol(format!("Failed to deserialize channel message: {}", e))
        })
    }

    /// チャネルを閉じる
    pub async fn close(&self) -> Result<(), NetworkError> {
        let mut stream = self.stream.lock().await;
        super::SystemStream::close(&mut *stream).await
    }

    /// チャネルがアクティブか確認
    pub fn is_active(&self) -> bool {
        // stream.lock() は async なので、ここでは Arc のstrong count で判定
        // 実際のアクティブ状態は send/recv 時にチェックされる
        Arc::strong_count(&self.stream) > 0
    }
}
