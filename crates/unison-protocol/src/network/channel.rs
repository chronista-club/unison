//! Channel: Unified Channel 通信プリミティブ
//!
//! 各ChannelはQUICストリームにマッピングされ、
//! 独立したHoL Blocking境界を形成する。
//!
//! `UnisonChannel` — 統合チャネル型（request/response + event push）

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;

use super::{NetworkError, ProtocolMessage, MessageType};
use super::quic::UnisonStream;

/// 統合チャネル型 — Request/Response と Event の両パターンをサポート
///
/// 内部に recv ループを持ち、受信メッセージを振り分ける:
/// - `Response` → pending の oneshot に送る
/// - `Event` / その他 → event_rx に流す
pub struct UnisonChannel {
    /// QUIC ストリームへの参照（送信用）
    stream: Arc<Mutex<UnisonStream>>,
    /// 応答待ちの Request を管理（message_id → oneshot::Sender）
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<ProtocolMessage>>>>,
    /// Event 受信キュー
    event_rx: Mutex<mpsc::Receiver<ProtocolMessage>>,
    /// メッセージ ID カウンター
    next_id: AtomicU64,
    /// バックグラウンド受信タスク
    recv_task: Mutex<Option<JoinHandle<()>>>,
}

impl UnisonChannel {
    /// UnisonStream から UnisonChannel を構築し、recv ループを起動する
    pub fn new(stream: UnisonStream) -> Self {
        let stream = Arc::new(Mutex::new(stream));
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<ProtocolMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (event_tx, event_rx) = mpsc::channel(256);

        // recv ループ
        let recv_stream = Arc::clone(&stream);
        let recv_pending = Arc::clone(&pending);
        let recv_task = tokio::spawn(async move {
            loop {
                let msg = {
                    let mut s = recv_stream.lock().await;
                    match super::SystemStream::receive(&mut *s).await {
                        Ok(value) => {
                            // ProtocolMessage として解釈
                            match serde_json::from_value::<ProtocolMessage>(value) {
                                Ok(msg) => msg,
                                Err(_) => continue,
                            }
                        }
                        Err(_) => {
                            // 接続断 — 全 pending を Error で解決
                            let mut map = recv_pending.lock().await;
                            for (_, sender) in map.drain() {
                                if let Ok(err_msg) = ProtocolMessage::new_with_json(
                                    0,
                                    "error".to_string(),
                                    MessageType::Error,
                                    serde_json::json!({"error": "connection closed"}),
                                ) {
                                    let _ = sender.send(err_msg);
                                }
                            }
                            break;
                        }
                    }
                };

                match msg.msg_type {
                    MessageType::Response => {
                        // pending マップから oneshot を取得して解決
                        let mut map = recv_pending.lock().await;
                        if let Some(sender) = map.remove(&msg.id) {
                            let _ = sender.send(msg);
                        }
                    }
                    MessageType::Error => {
                        // response_to が設定されていれば pending を解決
                        // そうでなければ event として流す
                        let mut map = recv_pending.lock().await;
                        // Error メッセージの id で pending を探す
                        if let Some(sender) = map.remove(&msg.id) {
                            let _ = sender.send(msg);
                        } else {
                            drop(map);
                            let _ = event_tx.send(msg).await;
                        }
                    }
                    _ => {
                        // Event, Request, その他 → event_rx に流す
                        let _ = event_tx.send(msg).await;
                    }
                }
            }
        });

        Self {
            stream,
            pending,
            event_rx: Mutex::new(event_rx),
            next_id: AtomicU64::new(1),
            recv_task: Mutex::new(Some(recv_task)),
        }
    }

    /// Request/Response パターン
    ///
    /// メッセージ ID を自動生成し、pending マップに登録。
    /// Response が返るまで await する。
    pub async fn request(
        &self,
        method: &str,
        payload: Value,
    ) -> Result<Value, NetworkError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        // pending に登録
        {
            let mut map = self.pending.lock().await;
            map.insert(id, tx);
        }

        // Request メッセージを送信
        let msg = ProtocolMessage::new_with_json(
            id,
            method.to_string(),
            MessageType::Request,
            payload,
        )?;
        {
            let mut stream = self.stream.lock().await;
            let value = serde_json::to_value(&msg).map_err(|e| {
                NetworkError::Protocol(format!("Failed to serialize message: {}", e))
            })?;
            super::SystemStream::send(&mut *stream, value).await?;
        }

        // Response を待つ
        let response = rx.await.map_err(|_| {
            NetworkError::Protocol("Request cancelled: channel closed".to_string())
        })?;

        match response.msg_type {
            MessageType::Error => {
                let payload = response.payload_as_value()?;
                Err(NetworkError::Protocol(format!(
                    "Request error: {}",
                    payload
                )))
            }
            _ => response.payload_as_value(),
        }
    }

    /// 一方向 Event 送信（応答不要）
    pub async fn send_event(
        &self,
        method: &str,
        payload: Value,
    ) -> Result<(), NetworkError> {
        let msg = ProtocolMessage::new_with_json(
            0,
            method.to_string(),
            MessageType::Event,
            payload,
        )?;
        let mut stream = self.stream.lock().await;
        let value = serde_json::to_value(&msg).map_err(|e| {
            NetworkError::Protocol(format!("Failed to serialize message: {}", e))
        })?;
        super::SystemStream::send(&mut *stream, value).await
    }

    /// Request に対する Response 送信（サーバー側パターン）
    ///
    /// `recv()` で受け取った Request の `id` を指定して
    /// Response メッセージを返す。
    pub async fn send_response(
        &self,
        request_id: u64,
        method: &str,
        payload: Value,
    ) -> Result<(), NetworkError> {
        let msg = ProtocolMessage::new_with_json(
            request_id,
            method.to_string(),
            MessageType::Response,
            payload,
        )?;
        let mut stream = self.stream.lock().await;
        let value = serde_json::to_value(&msg).map_err(|e| {
            NetworkError::Protocol(format!("Failed to serialize message: {}", e))
        })?;
        super::SystemStream::send(&mut *stream, value).await
    }

    /// Event 受信（サーバーからのプッシュ、または非 Response メッセージ）
    pub async fn recv(&self) -> Result<ProtocolMessage, NetworkError> {
        let mut rx = self.event_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            NetworkError::Protocol("Channel closed".to_string())
        })
    }

    /// チャネルを閉じる
    pub async fn close(&self) -> Result<(), NetworkError> {
        // recv タスクを中止
        if let Some(task) = self.recv_task.lock().await.take() {
            task.abort();
        }
        // ストリームを閉じる
        let mut stream = self.stream.lock().await;
        super::SystemStream::close(&mut *stream).await
    }
}
