# Unison Protocol

_次世代型の型安全通信プロトコルフレームワーク_

[![Crates.io](https://img.shields.io/crates/v/unison.svg)](https://crates.io/crates/unison)
[![Documentation](https://docs.rs/unison/badge.svg)](https://docs.rs/unison)
[![Build Status](https://github.com/chronista-club/unison/workflows/CI/badge.svg)](https://github.com/chronista-club/unison/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 概要

**Unison Protocol** は、KDL (KDL Document Language) ベースの型安全な通信プロトコルフレームワークである。QUIC トランスポートを活用し、高速・安全・拡張可能な分散システムとリアルタイムアプリケーションの構築を支援する。

### 主要機能

- **Unified Channel**: 全通信がチャネル経由。Request/Response、Event push、Raw bytes を1つの `UnisonChannel` で統一
- **Raw Bytes ストリーミング**: type tag 付きフレームにより、rkyv/zstd をバイパスした最小オーバーヘッドのバイナリ通信をサポート
- **Identity Handshake**: 接続時にサーバーが自己紹介情報（利用可能チャネル、状態）を自動通知
- **グレースフルシャットダウン**: `spawn_listen()` + `ServerHandle` によるライフサイクル管理
- **接続イベント通知**: `ConnectionEvent` で接続/切断をリアルタイムに監視
- **型安全な通信**: KDL スキーマベースの自動コード生成により、コンパイル時の型チェックを保証
- **超低レイテンシー**: QUIC (HTTP/3) トランスポートによる次世代の高速通信
- **IPv6 専用設計**: 最新のネットワーク標準である IPv6 のみをサポート（設計のシンプル化）
- **組み込みセキュリティ**: TLS 1.3 完全暗号化と開発用証明書の自動生成
- **ゼロコピー通信**: rkyv ベースの効率的なパケットシリアライゼーション + 2KB 以上の自動 zstd 圧縮

## クイックスタート

### インストール

```toml
[dependencies]
unison = "^0.3"
tokio = { version = "1.40", features = ["full"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
```

### 基本的な使用方法

#### 1. プロトコル定義 (KDL)

```kdl
// schemas/my_service.kdl
protocol "my-service" version="1.0.0" {
    namespace "com.example.myservice"

    channel "users" from="client" lifetime="persistent" {
        request "CreateUser" {
            field "name" type="string" required=#true
            field "email" type="string" required=#true

            returns "UserCreated" {
                field "id" type="string" required=#true
                field "created_at" type="timestamp" required=#true
            }
        }
    }

    channel "events" from="server" lifetime="persistent" {
        event "UserEvent" {
            field "event_type" type="string" required=#true
            field "user_id" type="string" required=#true
            field "timestamp" type="string"
        }
    }
}
```

#### 2. サーバー実装

```rust
use unison::{ProtocolServer, NetworkError};
use unison::network::UnisonChannel;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ProtocolServer::with_identity(
        "my-server", "1.0.0", "com.example.myservice",
    );

    // 接続イベントの購読
    let mut events = server.subscribe_connection_events().await;
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            println!("接続イベント: {:?}", event);
        }
    });

    // チャネルハンドラーの登録
    server.register_channel("users", |_ctx, stream| async move {
        let channel = UnisonChannel::new(stream);
        loop {
            match channel.recv().await {
                Ok(msg) => {
                    // request を処理して response を返す
                    channel.send_response(msg.id, &msg.method, json!({"id": "1"})).await?;
                }
                Err(_) => break,
            }
        }
        Ok(())
    }).await;

    // spawn_listen でバックグラウンド起動（グレースフルシャットダウン対応）
    let handle = server.spawn_listen("[::1]:8080").await?;
    println!("サーバー起動: {}", handle.local_addr());

    // シャットダウン
    // handle.shutdown().await?;
    Ok(())
}
```

#### 3. クライアント実装

```rust
use unison::ProtocolClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ProtocolClient::new_default()?;
    client.connect("[::1]:8080").await?;

    // チャネル経由の Request/Response
    let users = client.open_channel("users").await?;
    let response = users.request("CreateUser", json!({
        "name": "Alice",
        "email": "alice@example.com"
    })).await?;
    println!("作成されたユーザー: {}", response);

    // チャネル経由のイベント受信
    let events = client.open_channel("events").await?;
    loop {
        match events.recv().await {
            Ok(event) => println!("イベント: {:?}", event),
            Err(_) => break,
        }
    }

    Ok(())
}
```

#### 4. Raw Bytes ストリーミング

```rust
// オーディオデータ等のバイナリストリーミング
let audio = client.open_channel("audio").await?;

// 送信側: rkyv/zstd をバイパスした最小オーバーヘッド通信
audio.send_raw(&pcm_data).await?;

// 受信側
let data = audio.recv_raw().await?;
```

## アーキテクチャ

### Unified Channel

v0.2.0 で RPC を全廃し、全通信をチャネルに統合した。

```
Client                              Server
  |                                    |
  |-- open_channel("users") ---------> |  (QUIC bidi stream)
  |                                    |
  |<-- UnisonChannel ----------------->|  UnisonChannel
  |    .request()    → Protocol frame  |    .recv()
  |    .send_event() → Protocol frame  |    .send_response()
  |    .send_raw()   → Raw frame      |    .send_raw()
  |    .recv()       ← Protocol frame  |    .recv_raw()
  |    .recv_raw()   ← Raw frame      |
```

### Typed Frame フォーマット

```
[4 bytes: length][1 byte: type tag][payload]

type tag:
  0x00 = Protocol frame (ProtocolMessage → rkyv/zstd)
  0x01 = Raw frame (生バイト、シリアライズなし)
```

### コンポーネント構造

```
unison/
|-- コア層
|   |-- parser/          # KDL スキーマパーサー
|   |-- codegen/         # コードジェネレーター (Rust/TypeScript)
|   +-- packet/          # UnisonPacket (rkyv + zstd)
|
|-- ネットワーク層
|   |-- quic/            # QUIC トランスポート + Typed Frame I/O
|   |-- client/          # ProtocolClient (open_channel → UnisonChannel)
|   |-- server/          # ProtocolServer (register_channel, spawn_listen)
|   |-- channel/         # UnisonChannel (Request/Response + Event + Raw)
|   |-- identity/        # Identity Handshake (ServerIdentity, ChannelInfo)
|   |-- context/         # ConnectionContext (接続状態管理)
|   +-- service/         # サービス抽象化層
|
+-- コンテキスト層 (CGP)
    |-- adapter/         # 既存システム統合
    +-- handlers/        # 拡張可能ハンドラー
```

### コアコンポーネント

#### UnisonChannel -- 統合チャネル型

QUIC ストリーム上で動作する統合チャネル。Request/Response、Event push、Raw bytes の3パターンをサポートする。内部に recv ループを持ち、type tag で受信フレームを自動振り分けする。

```rust
impl UnisonChannel {
    // Request/Response パターン
    pub async fn request(&self, method: &str, payload: Value) -> Result<Value, NetworkError>;
    pub async fn send_response(&self, id: u64, method: &str, payload: Value) -> Result<(), NetworkError>;

    // Event push（一方向、応答不要）
    pub async fn send_event(&self, method: &str, payload: Value) -> Result<(), NetworkError>;
    pub async fn recv(&self) -> Result<ProtocolMessage, NetworkError>;

    // Raw bytes（シリアライズをバイパス）
    pub async fn send_raw(&self, data: &[u8]) -> Result<(), NetworkError>;
    pub async fn recv_raw(&self) -> Result<Vec<u8>, NetworkError>;

    pub async fn close(&self) -> Result<(), NetworkError>;
}
```

#### ServerHandle -- サーバーライフサイクル管理

```rust
let server = ProtocolServer::new();
// チャネルハンドラー登録...

// バックグラウンドで起動
let handle = server.spawn_listen("[::1]:8080").await?;

// 状態確認
println!("アドレス: {}", handle.local_addr());
println!("終了済み: {}", handle.is_finished());

// グレースフルシャットダウン
handle.shutdown().await?;
```

#### ConnectionEvent -- 接続イベント通知

```rust
let mut events = server.subscribe_connection_events().await;
tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        match event {
            ConnectionEvent::Connected { remote_addr, context } => {
                println!("接続: {}", remote_addr);
            }
            ConnectionEvent::Disconnected { remote_addr } => {
                println!("切断: {}", remote_addr);
            }
        }
    }
});
```

#### UnisonPacket -- ゼロコピーパケット型

```rust
use unison::packet::{UnisonPacket, StringPayload};

let payload = StringPayload::from_string("Hello, World!");
let packet = UnisonPacket::builder()
    .with_stream_id(123)
    .with_sequence(1)
    .build(payload)?;

// Bytes に変換（2KB 以上は自動 zstd 圧縮）
let bytes = packet.to_bytes();

// Bytes から復元（ゼロコピーデシリアライゼーション）
let restored = UnisonPacket::<StringPayload>::from_bytes(&bytes)?;
```

## テスト

```bash
# 標準テスト実行
RUSTFLAGS="-C symbol-mangling-version=v0" cargo test --tests --workspace -- --skip packet

# clippy
cargo clippy --lib --workspace -- -D warnings

# 統合テストのみ
cargo test --test quic_integration_test

# 詳細ログ付き
RUST_LOG=debug cargo test -- --nocapture
```

## ドキュメント

- [API リファレンス](https://docs.rs/unison)
- **仕様書** (spec/)
  - [コアコンセプト](spec/01-core-concept/SPEC.md) -- Everything is a Channel、3 層アーキテクチャ
  - [Unified Channel プロトコル](spec/02-protocol-rpc/SPEC.md) -- KDL スキーマ、request/event 構文、コード生成
  - [Stream Channel](spec/03-stream-channels/SPEC.md) -- UnisonChannel、チャネル型仕様
- **設計ドキュメント** (design/)
  - [アーキテクチャ設計](design/architecture.md)
  - [パケット実装仕様](design/packet.md)
  - [QUIC ランタイム設計](design/quic-runtime.md) -- Stream-First API の QUIC ランタイム実装
- **実装ガイド** (guides/)
  - [Quinn API ガイド](guides/quinn-stream-api.md)
  - [チャネルガイド](guides/channel-guide.md) -- Unified Channel の実践ガイド

## 開発

### ビルド要件

- Rust 1.93 以上（MSRV）
- Rust 2024 エディション
- Tokio 1.40 以上

### 開発環境のセットアップ

```bash
git clone https://github.com/chronista-club/unison
cd unison
cargo build

# テストの実行
RUSTFLAGS="-C symbol-mangling-version=v0" cargo test --tests --workspace -- --skip packet
```

> **macOS 開発者向けの注意**: macOS の標準リンカーには制限があるため、テストを実行するには `lld` リンカーが必要な場合がある。`brew install lld` でインストール後、`.cargo/config.toml` に以下を追加:
>
> ```toml
> [target.aarch64-apple-darwin]
> linker = "clang"
> rustflags = ["-C", "link-arg=-fuse-ld=/opt/homebrew/bin/ld64.lld"]
> ```

## ライセンス

MIT License - 詳細は [LICENSE](LICENSE) ファイルを参照。

## 謝辞

- [Quinn](https://github.com/quinn-rs/quinn) - QUIC 実装
- [KDL](https://kdl.dev/) - 設定言語
- [Tokio](https://tokio.rs/) - 非同期ランタイム
- [rkyv](https://github.com/rkyv/rkyv) - ゼロコピーシリアライゼーション

---

**Unison Protocol** - _言語とプラットフォームを越えた通信の調和_

[GitHub](https://github.com/chronista-club/unison) | [Crates.io](https://crates.io/crates/unison)
