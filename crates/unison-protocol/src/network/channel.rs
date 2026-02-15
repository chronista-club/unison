//! Channel: Stream-First APIの通信プリミティブ
//!
//! 各ChannelはQUICストリームにマッピングされ、
//! 独立したHoL Blocking境界を形成する。

use tokio::sync::mpsc;

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
    tx: mpsc::Sender<(Req, tokio::sync::oneshot::Sender<Res>)>,
}
