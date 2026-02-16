# Unison Protocol

_次世代型の型安全通信プロトコルフレームワーク_

[![Crates.io](https://img.shields.io/crates/v/unison.svg)](https://crates.io/crates/unison)
[![Documentation](https://docs.rs/unison/badge.svg)](https://docs.rs/unison)
[![Build Status](https://github.com/chronista-club/unison/workflows/CI/badge.svg)](https://github.com/chronista-club/unison/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 概要

**Unison Protocol** は、KDL (KDL Document Language) ベースの型安全な通信プロトコルフレームワークである。QUIC トランスポートを活用し、高速・安全・拡張可能な分散システムとリアルタイムアプリケーションの構築を支援する。

### 主要機能

- **Stream-First API**: 各チャネルが独立した QUIC ストリームにマッピングされ、Head-of-Line Blocking を排除した型安全な通信
- **Identity Handshake**: 接続時にサーバーが自己紹介情報（利用可能チャネル、状態）を自動通知し、動的なチャネル管理を実現
- **型安全な通信**: KDL スキーマベースの自動コード生成により、コンパイル時の型チェックを保証
- **超低レイテンシー**: QUIC (HTTP/3) トランスポートによる次世代の高速通信
- **IPv6 専用設計**: 最新のネットワーク標準である IPv6 のみをサポート（設計のシンプル化）
- **組み込みセキュリティ**: TLS 1.3 完全暗号化と開発用証明書の自動生成
- **CGP (Context-Generic Programming)**: 拡張可能なコンポーネントベースアーキテクチャ
- **完全非同期**: Rust 2024 エディション + Tokio による最新の非同期実装
- **双方向ストリーミング**: QUIC ベースの全二重通信によるリアルタイムデータ転送
- **スキーマファースト**: プロトコル定義駆動開発（KDL スキーマから型・チャネル・サービスを自動生成）
- **ゼロコピー通信**: rkyv ベースの効率的なパケットシリアライゼーション
- **自動圧縮**: 2KB 以上のペイロードを zstd Level 1 で自動圧縮

## クイックスタート

### インストール

```toml
[dependencies]
unison = "^0.1"
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

    // チャネルベースの Request/Response + Event 定義
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ProtocolServer::with_identity(
        "my-server", "1.0.0", "com.example.myservice",
    );

    // チャネルハンドラーの登録
    server.register_channel("users", |ctx, stream| async move {
        let mut channel = UnisonChannel::new(stream);
        loop {
            match channel.recv().await {
                Ok(msg) => { /* request を処理 */ }
                Err(_) => break,
            }
        }
        Ok(())
    }).await;

    server.register_channel("events", |ctx, stream| async move {
        let channel = UnisonChannel::new(stream);
        // イベント配信ロジック
        channel.send_event("UserEvent", json!({"event_type": "created", "user_id": "123"})).await?;
        Ok(())
    }).await;

    server.listen("[::1]:8080").await?;
    Ok(())
}
```

#### 3. クライアント実装

```rust
use unison::ProtocolClient;
use unison::network::UnisonChannel;
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
    let mut events = client.open_channel("events").await?;
    loop {
        match events.recv().await {
            Ok(event) => println!("イベント: {:?}", event),
            Err(_) => break,
        }
    }

    Ok(())
}
```

## アーキテクチャ

### コンポーネント構造

```
unison/
|-- コア層
|   |-- parser/          # KDL スキーマパーサー
|   |-- codegen/         # コードジェネレーター (Rust/TypeScript)
|   |-- types/           # 基本型定義
|   +-- packet/          # UnisonPacket 型定義
|
|-- ネットワーク層
|   |-- quic/            # QUIC トランスポート実装
|   |-- client/          # ProtocolClient (open_channel → UnisonChannel)
|   |-- server/          # ProtocolServer (register_channel)
|   |-- channel/         # UnisonChannel (統合チャネル型)
|   |-- identity/        # Identity Handshake (ServerIdentity, ChannelInfo)
|   |-- context/         # ConnectionContext (接続状態管理)
|   +-- service/         # サービス抽象化層
|
+-- コンテキスト層 (CGP)
    |-- adapter/         # 既存システム統合
    |-- handlers/        # 拡張可能ハンドラー
    +-- traits/          # ジェネリックトレイト定義
```

### コアコンポーネント

#### UnisonStream -- 低レベル双方向ストリーミング

```rust
pub trait SystemStream: Send + Sync {
    async fn send(&mut self, data: Value) -> Result<(), NetworkError>;
    async fn receive(&mut self) -> Result<Value, NetworkError>;
    async fn close(&mut self) -> Result<(), NetworkError>;
    fn is_active(&self) -> bool;
}
```

#### UnisonChannel -- 統合チャネル型

QUIC ストリーム上で動作する統合チャネル。Request/Response パターンと Event push の両方をサポートする。内部に recv ループを持ち、受信メッセージを自動的に振り分ける。

```rust
pub struct UnisonChannel {
    stream: Arc<Mutex<UnisonStream>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<ProtocolMessage>>>>,
    event_rx: mpsc::Receiver<ProtocolMessage>,
}

impl UnisonChannel {
    pub async fn request(&self, method: &str, payload: Value) -> Result<Value, NetworkError>;
    pub async fn send_event(&self, method: &str, payload: Value) -> Result<(), NetworkError>;
    pub async fn recv(&mut self) -> Result<ProtocolMessage, NetworkError>;
    pub async fn close(&mut self) -> Result<(), NetworkError>;
}
```

#### Service -- 高レベルサービス抽象化

```rust
pub trait Service: Send + Sync {
    fn service_type(&self) -> &str;
    fn version(&self) -> &str;
    async fn handle_request(&mut self, method: &str, payload: Value)
        -> Result<Value, NetworkError>;
}
```

#### UnisonPacket -- ゼロコピー効率的パケット型

```rust
pub struct UnisonPacket<T: Payloadable> {
    // rkyv + zstd による効率的なシリアライゼーション
    // 2KB 以上のペイロードは自動圧縮
    // CRC32 チェックサム付き
}

impl<T: Payloadable> UnisonPacket<T> {
    pub fn builder() -> UnisonPacketBuilder<T>;
    pub fn from_bytes(data: Bytes) -> Result<Self, PacketError>;
    pub fn extract_payload(&self) -> Result<T, PayloadError>;
}
```

#### CGP Context -- 拡張可能なコンテキスト

```rust
pub struct CgpProtocolContext<T, R, H> {
    transport: T,      // トランスポート層
    registry: R,       // サービスレジストリ
    handlers: H,       // メッセージハンドラー
}
```

## パフォーマンス

### 特徴

- **超低レイテンシ**: QUIC による高速通信
- **高スループット**: マルチストリーム並列処理（チャネルごとに独立）
- **効率的**: ゼロコピーデシリアライゼーション
- **省リソース**: 最適化された CPU/メモリ使用率

_ベンチマーク結果は実測後に掲載予定_

## テスト

### テストの実行

```bash
# 全テストの実行
cargo test

# 統合テストのみ
cargo test --test quic_integration_test

# 詳細ログ付き
RUST_LOG=debug cargo test -- --nocapture
```

### テストカバレッジ

- QUIC 接続/切断
- メッセージシリアライゼーション
- ハンドラー登録/呼び出し
- エラーハンドリング
- SystemStream ライフサイクル
- サービスメタデータ管理
- 証明書自動生成
- Identity Handshake
- チャネル開設/送受信

## 高度な使用方法

### UnisonPacket による効率的な通信

```rust
use unison::packet::{UnisonPacket, Payloadable};

// カスタムペイロード定義
#[derive(Archive, Serialize, Deserialize, Debug)]
struct MyPayload {
    message: String,
    timestamp: i64,
    data: Vec<u8>,
}

impl Payloadable for MyPayload {}

// パケットの送信
let payload = MyPayload {
    message: "Hello".to_string(),
    timestamp: 1234567890,
    data: vec![1, 2, 3, 4, 5],
};

let packet = UnisonPacket::builder()
    .payload(payload)
    .priority(5)
    .build()?;

// バイト配列への変換（自動圧縮付き）
let bytes = packet.to_bytes()?;
stream.send_bytes(bytes).await?;

// パケットの受信（ゼロコピーデシリアライゼーション）
let received_bytes = stream.receive_bytes().await?;
let received_packet = UnisonPacket::<MyPayload>::from_bytes(received_bytes)?;
let received_payload = received_packet.extract_payload()?;
```

### チャネルベースの双方向通信

```rust
use unison::ProtocolClient;
use unison::network::UnisonChannel;
use serde_json::json;

let mut client = ProtocolClient::new_default()?;
client.connect("[::1]:8080").await?;

// チャネルを開く
let mut channel = client.open_channel("messaging").await?;

// Request/Response パターン
let response = channel.request("send_message", json!({
    "to": "user_123",
    "body": "Hello!"
})).await?;

// イベント受信（非同期ループ）
tokio::spawn(async move {
    loop {
        match channel.recv().await {
            Ok(event) => println!("イベント: {:?}", event),
            Err(_) => break,
        }
    }
});
```

### サービスメトリクス

```rust
let stats = service.get_performance_stats().await?;
println!("レイテンシ: {:?}", stats.avg_latency);
println!("スループット: {} msg/s", stats.messages_per_second);
println!("アクティブストリーム: {}", stats.active_streams);
```

## ドキュメント

- [API リファレンス](https://docs.rs/unison)
- **仕様書** (spec/) -- プロジェクトの正式な要求仕様
  - [コアコンセプト](spec/01-core-concept/SPEC.md) -- Everything is a Channel、3 層アーキテクチャ
  - [Unified Channel プロトコル](spec/02-protocol-rpc/SPEC.md) -- KDL スキーマ、request/event 構文、コード生成
  - [Stream Channel](spec/03-stream-channels/SPEC.md) -- UnisonChannel、チャネル型仕様
- **設計ドキュメント** (design/) -- 実装方法の詳細
  - [アーキテクチャ設計](design/architecture.md)
  - [パケット実装仕様](design/packet.md)
  - [QUIC ランタイム設計](design/quic-runtime.md) -- Stream-First API の QUIC ランタイム実装
- **実装ガイド** (guides/) -- 実装時の参考資料
  - [Quinn API ガイド](guides/quinn-stream-api.md)
  - [チャネルガイド](guides/channel-guide.md) -- Stream Channel の実践ガイド
- [コントリビューションガイド](CONTRIBUTING.md)

## 開発

### ビルド要件

- Rust 1.93 以上（MSRV）
- Rust 2024 エディション
- Tokio 1.40 以上
- OpenSSL または BoringSSL (QUIC 用)

### 開発環境のセットアップ

```bash
# リポジトリのクローン
git clone https://github.com/chronista-club/unison
cd unison

# macOS の場合: LLD リンカーをインストール（テスト実行に必要）
brew install lld

# 依存関係のインストール
cargo build

# 開発サーバーの起動
cargo run --example unison_ping_server

# テストの実行
cargo test
```

> **macOS 開発者向けの注意**: macOS の標準リンカーには制限があるため、テストを実行するには `lld` リンカーが必要である。`brew install lld` でインストール後、プロジェクトルートに `.cargo/config.toml` ファイルを作成して以下の設定を追加する:
>
> ```toml
> [target.aarch64-apple-darwin]
> linker = "clang"
> rustflags = ["-C", "link-arg=-fuse-ld=/opt/homebrew/bin/ld64.lld"]
> ```
>
> **注**: `.cargo/config.toml` はローカル開発環境専用の設定ファイルである（`.gitignore` に含まれている）。CI 環境では不要。

### コード生成

```bash
# KDL スキーマからコード生成
cargo build --features codegen

# TypeScript 定義の生成
cargo run --bin generate-ts
```

## 今後の展望

### WASM/SDK 化

Unison Protocol をブラウザ環境や他言語から利用可能にするための SDK 化を計画している。

- **WebSocket トランスポート**: QUIC が利用できない環境でのフォールバック（将来的に WebTransport へ移行）
- **wasm-bindgen**: Rust 実装を WebAssembly にコンパイルし、JavaScript/TypeScript から直接利用
- **tsify**: Rust の型定義から TypeScript の型を自動生成し、型安全性をフロントエンドまで維持
- **WebTransport**: ブラウザネイティブの QUIC 対応プロトコルとして、WebSocket の後継トランスポートに位置づけ

## コントリビューション

プルリクエストを歓迎します。以下のガイドラインに従ってください:

1. フォークしてフィーチャーブランチを作成
2. テストを追加（カバレッジ 80% 以上）
3. `cargo fmt` と `cargo clippy` を実行
4. [Conventional Commits](https://www.conventionalcommits.org/) に従ったコミットメッセージ
5. プルリクエストを提出

## ライセンス

MIT License - 詳細は [LICENSE](LICENSE) ファイルを参照。

## 謝辞

- [Quinn](https://github.com/quinn-rs/quinn) - QUIC 実装
- [KDL](https://kdl.dev/) - 設定言語
- [Tokio](https://tokio.rs/) - 非同期ランタイム

---

**Unison Protocol** - _言語とプラットフォームを越えた通信の調和_

[GitHub](https://github.com/chronista-club/unison) | [Crates.io](https://crates.io/crates/unison) | [Discord](https://discord.gg/unison)
