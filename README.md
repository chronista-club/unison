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

## ワークスペースクレート

| クレート | 説明 | 状態 |
|---------|------|------|
| [`unison-protocol`](crates/unison-protocol) | コアプロトコルライブラリ — KDL スキーマ、QUIC トランスポート、チャネル、パケット | Active |
| [`unison-agent`](crates/unison-agent) | Claude Agent SDK と Unison Protocol の統合 | Active |
| [`unison-cli`](crates/unison-cli) | CLI ツール — スキーマ生成、開発ユーティリティ | Scaffolded |
| [`unison-network`](crates/unison-network) | 高レベルネットワーク抽象化（P2P、メッシュ） | Placeholder |

### unison-protocol

コアクレート。crates.io では `unison` として公開。以下を含む:
- KDL スキーマパーサーとコードジェネレーター（Rust/TypeScript）
- QUIC トランスポート層（Quinn 経由）
- `ProtocolServer` / `ProtocolClient` / `UnisonChannel`
- `UnisonPacket`（rkyv ゼロコピーシリアライゼーション + zstd 圧縮）
- Identity Handshake と Connection Events

### unison-agent

[Claude Agent SDK](https://crates.io/crates/claude-agent-sdk) と Unison Protocol を統合するクレート。`AgentClient`（単発・バッチクエリ）、`InteractiveClient`（マルチターン会話）、`UnisonTools`（Unison チャネルを MCP ツールとして Claude Agent に公開）を提供。

**Examples** (`crates/unison-agent/examples/`):

| Example | 説明 |
|---------|------|
| `simple_query` | AgentClient 経由の単発クエリ |
| `batch_query` | 複数クエリの逐次処理 |
| `interactive_chat` | InteractiveClient によるマルチターン会話 |
| `unison_direct_demo` | Unison Protocol 直接統合 |
| `unison_mcp_demo` | Unison チャネルを MCP ツールとして公開 |

```bash
# Example の実行
cargo run -p unison-agent --example simple_query
```

### unison-cli

Unison Protocol の CLI。バイナリ名: `unison`。スキーマ生成、開発ツール、ユーティリティを提供予定。現在はスキャフォールド段階（main 関数のみ）。

### unison-network

P2P やメッシュネットワーキングの高レベル抽象化を予定。現在はプレースホルダー（実装なし）。`unison-protocol` と Quinn に依存。

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
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ProtocolServer::with_identity(
        "my-server", "1.0.0", "com.example.myservice",
    );

    // 接続イベントの購読（同期メソッド — broadcast::Receiver を直接返す）
    let mut events = server.subscribe_connection_events();
    tokio::spawn(async move {
        // broadcast::Receiver::recv() は Result を返す（Option ではない）
        while let Ok(event) = events.recv().await {
            println!("接続イベント: {:?}", event);
        }
    });

    // チャネルハンドラーの登録
    // ハンドラーは (Arc<ConnectionContext>, UnisonStream) を受け取り Result<(), NetworkError> を返す
    server.register_channel("users", |_ctx, stream| async move {
        let channel = UnisonChannel::new(stream);
        loop {
            match channel.recv().await {
                Ok(msg) => {
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
    let client = ProtocolClient::new_default()?;
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

// 送信側: 最小オーバーヘッド、rkyv/zstd をバイパス
audio.send_raw(&pcm_data).await?;

// 受信側
let data = audio.recv_raw().await?;
```

## アーキテクチャ

### Unified Channel

v0.2.0 で RPC を完全に廃止し、全通信をチャネルに統一。

```
Client                              Server
  |                                    |
  |-- open_channel("users") ---------> |  (QUIC bidi stream)
  |                                    |
  |<-- UnisonChannel ----------------->|  UnisonChannel
  |    .request()    -> Protocol frame  |    .recv()
  |    .send_event() -> Protocol frame  |    .send_response()
  |    .send_raw()   -> Raw frame      |    .send_raw()
  |    .recv()       <- Protocol frame  |    .recv_raw()
  |    .recv_raw()   <- Raw frame      |
```

### Typed Frame Format

```
[4 bytes: length][1 byte: type tag][payload]

type tag:
  0x00 = Protocol frame (ProtocolMessage -> rkyv/zstd)
  0x01 = Raw frame (raw bytes, no serialization)
```

### コンポーネント構成

```
unison/
|-- Core Layer
|   |-- parser/          # KDL スキーマパーサー
|   |-- codegen/         # コードジェネレーター（Rust/TypeScript）
|   +-- packet/          # UnisonPacket (rkyv + zstd)
|
|-- Network Layer
|   |-- quic/            # QUIC トランスポート + Typed Frame I/O
|   |-- client/          # ProtocolClient (open_channel -> UnisonChannel)
|   |-- server/          # ProtocolServer (register_channel, spawn_listen)
|   |-- channel/         # UnisonChannel (Request/Response + Event + Raw)
|   |-- identity/        # Identity Handshake (ServerIdentity, ChannelInfo)
|   +-- context/         # ConnectionContext（接続状態管理）
|
+-- 依存ライブラリ
    |-- cgp              # Context-Generic Programming（拡張基盤）
    +-- unison-kdl       # Unison スキーマ用 KDL 拡張
```

> **CGP について**: [Context-Generic Programming](https://crates.io/crates/cgp) (`cgp`, `cgp-component`) は、将来的なトレイトベースのコンポーネントアーキテクチャ構築のためにワークスペース依存に含まれている。現時点ではアプリケーションコードで積極的には使用されていない。

### 主要コンポーネント

#### UnisonChannel — 統一チャネル型

QUIC ストリーム上で動作する統一チャネル。Request/Response、Event push、Raw bytes の3パターンをサポート。内部に recv ループを持ち、受信フレームを type tag で自動振り分けする。

```rust
impl UnisonChannel {
    // Request/Response パターン
    pub async fn request(&self, method: &str, payload: Value) -> Result<Value, NetworkError>;
    pub async fn send_response(&self, id: u64, method: &str, payload: Value) -> Result<(), NetworkError>;

    // Event push（一方向、応答不要）
    pub async fn send_event(&self, method: &str, payload: Value) -> Result<(), NetworkError>;
    pub async fn recv(&self) -> Result<ProtocolMessage, NetworkError>;

    // Raw bytes（シリアライゼーションをバイパス）
    pub async fn send_raw(&self, data: &[u8]) -> Result<(), NetworkError>;
    pub async fn recv_raw(&self) -> Result<Vec<u8>, NetworkError>;

    pub async fn close(&self) -> Result<(), NetworkError>;
}
```

#### ServerHandle — サーバーライフサイクル管理

```rust
let server = ProtocolServer::new();
// チャネルハンドラーの登録...

// バックグラウンドで起動
let handle = server.spawn_listen("[::1]:8080").await?;

// 状態確認
println!("アドレス: {}", handle.local_addr());
println!("終了済み: {}", handle.is_finished());

// グレースフルシャットダウン
handle.shutdown().await?;
```

#### ConnectionEvent — 接続イベント通知

```rust
// subscribe_connection_events() は非同期ではない — broadcast::Receiver を直接返す
let mut events = server.subscribe_connection_events();
tokio::spawn(async move {
    // broadcast::Receiver::recv() は Result を返す（Option ではない）
    while let Ok(event) = events.recv().await {
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

#### UnisonPacket — ゼロコピーパケット型

```rust
use unison::packet::{UnisonPacket, StringPayload};

let payload = StringPayload::from_string("Hello, World!");
let packet = UnisonPacket::builder()
    .with_stream_id(123)
    .with_sequence(1)
    .build(payload)?;

// Bytes に変換（2KB 以上のペイロードは自動 zstd 圧縮）
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

# インテグレーションテストのみ
cargo test --test quic_integration_test

# 詳細ログ付き
RUST_LOG=debug cargo test -- --nocapture
```

## ドキュメント

- [API リファレンス](https://docs.rs/unison)
- **仕様書** (spec/)
  - [コアコンセプト](spec/01-core-concept/SPEC.md) — Everything is a Channel、3層アーキテクチャ
  - [Unified Channel Protocol](spec/02-protocol-rpc/SPEC.md) — KDL スキーマ、request/event 構文、コード生成
  - [Stream Channel](spec/03-stream-channels/SPEC.md) — UnisonChannel、チャネル型仕様
- **設計書** (design/)
  - [アーキテクチャ設計](design/architecture.md)
  - [パケット実装仕様](design/packet.md)
  - [QUIC ランタイム設計](design/quic-runtime.md) — Stream-First API QUIC ランタイム実装
- **実装ガイド** (guides/)
  - [Quinn API ガイド](guides/quinn-stream-api.md)
  - [チャネルガイド](guides/channel-guide.md) — Unified Channel の実践ガイド

## 開発

### ビルド要件

- Rust 1.93+ (MSRV)
- Rust 2024 edition
- Tokio 1.40+

### 開発環境のセットアップ

```bash
git clone https://github.com/chronista-club/unison
cd unison
cargo build

# テスト実行
RUSTFLAGS="-C symbol-mangling-version=v0" cargo test --tests --workspace -- --skip packet
```

> **macOS 開発者への注意**: macOS のデフォルトリンカーには制限があるため、テスト実行に `lld` リンカーが必要な場合がある。`brew install lld` でインストール後、`.cargo/config.toml` に以下を追加:
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
- [CGP](https://github.com/informalsystems/cgp) - Context-Generic Programming
- [Claude Agent SDK](https://crates.io/crates/claude-agent-sdk) - AI エージェント統合

---

**Unison Protocol** - _言語とプラットフォームを超えた通信の調和_

[GitHub](https://github.com/chronista-club/unison) | [Crates.io](https://crates.io/crates/unison)
