# UnisonPacket - バイナリパケット層

## 概要

UnisonPacketは、Unison Protocolの低レベルバイナリパケットフォーマットです。rkyvによるゼロコピーデシリアライゼーションと、効率的な自動圧縮により、高速かつ帯域効率の良い通信を実現します。

## 主要な特徴

### 🚀 パフォーマンス
- **ゼロコピーデシリアライゼーション**: rkyvを使用し、メモリアロケーションなしで直接データを読み取り
- **自動圧縮**: 2KB以上のペイロードをzstd Level 1で自動的に圧縮
- **固定長ヘッダー**: 48バイトの固定長ヘッダーで高速なパース

### 🔒 型安全性
- **ジェネリクス**: `UnisonPacket<T: Payloadable>`による型安全なペイロード
- **ビルダーパターン**: 型安全なパケット構築
- **トレイト境界**: `Payloadable`トレイトによる明確なインターフェース

### 🎛️ 柔軟性
- **拡張可能なフラグ**: 16ビットのフラグフィールドで将来の拡張に対応
- **オプションフィールド**: チェックサム、タイムスタンプなどの選択的な機能
- **カスタムペイロード**: `Payloadable`トレイトを実装することで任意の型をサポート

## アーキテクチャ

### パケット構造

```
┌─────────────────────────────────────┐
│         UnisonPacket                │
├─────────────────────────────────────┤
│  Header (48 bytes, 非圧縮)          │
│  ├─ version: u8                    │
│  ├─ packet_type: u8                │
│  ├─ flags: u16                     │
│  ├─ payload_length: u32            │
│  ├─ compressed_length: u32         │
│  ├─ checksum: u32                  │
│  ├─ sequence_number: u64           │
│  ├─ timestamp: u64                 │
│  ├─ stream_id: u64                 │
│  └─ _padding: [u8; 8]              │
├─────────────────────────────────────┤
│  Payload (可変長)                   │
│  └─ 圧縮 or 非圧縮 (rkyv形式)      │
└─────────────────────────────────────┘
```

### モジュール構成

```
src/packet/
├── mod.rs          # メインモジュール、UnisonPacket実装
├── flags.rs        # PacketFlagsビットフラグ
├── header.rs       # UnisonPacketHeader構造体
├── payload.rs      # Payloadableトレイトと基本実装
└── serialization.rs # シリアライゼーション/圧縮ロジック
```

## 使用方法

### 基本的な使用例

```rust
use unison::packet::{UnisonPacket, StringPayload};

// ペイロードの作成
let payload = StringPayload::from_str("Hello, Unison!");

// パケットの構築
let packet = UnisonPacket::builder()
    .packet_type(PacketType::Data)
    .with_stream_id(12345)
    .with_sequence(1)
    .with_checksum()
    .build(payload)?;

// Bytesに変換（ネットワーク送信用）
let bytes = packet.to_bytes();
println!("パケットサイズ: {} bytes", bytes.len());

// パケットの復元
let restored = UnisonPacket::<StringPayload>::from_bytes(&bytes)?;
let header = restored.header()?;
println!("ストリームID: {}", header.stream_id);

// ペイロードの取得
let restored_payload = restored.payload()?;
println!("メッセージ: {}", restored_payload.data);
```

### ゼロコピー読み取り

```rust
use unison::packet::{UnisonPacketView, BytesPayload};

// パケットビューの作成（コピーなし）
let view = UnisonPacketView::from_bytes(&bytes)?;

// ヘッダー情報の取得
println!("圧縮: {}", view.is_compressed());
println!("ペイロードサイズ: {} bytes", view.payload_size());

// ゼロコピーでペイロードを参照
let mut buffer = Vec::new();
let archived = packet.payload_zero_copy(&mut buffer)?;
// archivedは元のデータを直接参照（コピーなし）
```

### カスタムペイロードの実装

```rust
use rkyv::{Archive, Deserialize, Serialize};
use unison::packet::Payloadable;

#[derive(Archive, Deserialize, Serialize, Debug)]
#[archive(check_bytes)]
pub struct MyCustomPayload {
    pub id: u64,
    pub name: String,
    pub data: Vec<u8>,
}

// Payloadableは自動的に実装される
impl Payloadable for MyCustomPayload {}

// 使用例
let custom = MyCustomPayload {
    id: 42,
    name: "test".to_string(),
    data: vec![1, 2, 3, 4, 5],
};

let packet = UnisonPacket::new(custom)?;
```

## パケットフラグ

PacketFlagsは16ビットのビットフィールドで、パケットの状態や処理方法を示します。

```rust
pub struct PacketFlags {
    pub const COMPRESSED: u16      = 0x0001; // ペイロード圧縮
    pub const ENCRYPTED: u16       = 0x0002; // 暗号化（将来）
    pub const FRAGMENTED: u16      = 0x0004; // 分割パケット
    pub const LAST_FRAGMENT: u16   = 0x0008; // 最後の分割
    pub const PRIORITY_HIGH: u16   = 0x0010; // 高優先度
    pub const REQUIRES_ACK: u16    = 0x0020; // ACK要求
    pub const IS_ACK: u16          = 0x0040; // ACKパケット
    pub const KEEPALIVE: u16       = 0x0080; // キープアライブ
    pub const ERROR: u16           = 0x0100; // エラー含む
    pub const METADATA: u16        = 0x0200; // メタデータ付き
    // 0x0400 - 0x8000: 将来の拡張用
}
```

### フラグの使用例

```rust
let packet = UnisonPacket::builder()
    .with_high_priority()
    .requires_ack()
    .build(payload)?;

// カスタムフラグの設定
let mut flags = PacketFlags::new();
flags.set(PacketFlags::PRIORITY_HIGH | PacketFlags::REQUIRES_ACK);

let packet = UnisonPacket::builder()
    .with_flags(flags)
    .build(payload)?;
```

## 圧縮機能

### 自動圧縮の仕組み

1. **閾値判定**: ペイロードサイズが2048バイト以上の場合に圧縮を検討
2. **圧縮実行**: zstd Level 1（最速設定）で圧縮
3. **効果判定**: 圧縮後のサイズが元のサイズより小さい場合のみ採用
4. **フラグ設定**: `PacketFlags::COMPRESSED`を自動的に設定

### 圧縮パラメータ

```rust
pub const COMPRESSION_THRESHOLD: usize = 2048;  // 2KB
pub const COMPRESSION_LEVEL: i32 = 1;          // zstd Level 1
pub const MAX_PACKET_SIZE: usize = 16 * 1024 * 1024; // 16MB
```

### パフォーマンス特性

| ペイロードサイズ | 圧縮 | レイテンシー | 帯域削減 |
|--------------|------|-----------|---------|
| < 2KB | なし | < 1μs | 0% |
| 2-10KB | 自動 | ~5μs | 30-50% |
| 10KB-1MB | 自動 | ~50μs | 50-70% |
| > 1MB | 自動 | ~500μs | 60-80% |

*テキストデータの場合の目安値

## エラーハンドリング

### SerializationError

```rust
pub enum SerializationError {
    Payload(PayloadError),           // ペイロードエラー
    CompressionFailed(String),       // 圧縮失敗
    DecompressionFailed(String),     // 解凍失敗
    PacketTooLarge { size, max_size }, // サイズ超過
    InvalidHeader,                   // 不正なヘッダー
    ChecksumMismatch { expected, actual }, // チェックサム不一致
    IncompatibleVersion { version }, // バージョン非互換
}
```

### エラー処理の例

```rust
match UnisonPacket::<MyPayload>::from_bytes(&bytes) {
    Ok(packet) => {
        // 正常処理
    }
    Err(SerializationError::ChecksumMismatch { expected, actual }) => {
        eprintln!("チェックサムエラー: 期待値 {:#x}, 実際 {:#x}", expected, actual);
        // 再送要求など
    }
    Err(SerializationError::IncompatibleVersion { version }) => {
        eprintln!("非互換バージョン: {}", version);
        // プロトコルネゴシエーション
    }
    Err(e) => {
        eprintln!("パケットエラー: {}", e);
    }
}
```

## パフォーマンス最適化

### メモリ効率

- **ゼロコピー**: 非圧縮パケットの読み取りは完全にゼロコピー
- **遅延デシリアライゼーション**: ペイロードは必要時まで処理されない
- **固定長ヘッダー**: ヘッダーの高速パース

### ネットワーク効率

- **自動圧縮**: 大きなペイロードは自動的に圧縮
- **バッチ処理**: 複数のパケットを効率的に処理可能
- **ストリーミング**: QUICストリームとの統合に最適化

### 最適化のヒント

1. **バッファの再利用**
```rust
let mut buffer = Vec::with_capacity(4096);
for packet_bytes in packets {
    buffer.clear();
    let archived = packet.payload_zero_copy(&mut buffer)?;
    // bufferを再利用してアロケーションを削減
}
```

2. **ビルダーの再利用**
```rust
let builder = UnisonPacket::builder()
    .with_stream_id(stream_id)
    .with_checksum();

for (seq, payload) in payloads.enumerate() {
    let packet = builder.clone()
        .with_sequence(seq as u64)
        .build(payload)?;
    // ...
}
```

## テスト

### ユニットテスト

```bash
# パケットモジュールのテスト実行
cargo test packet

# 詳細出力付き
cargo test packet -- --nocapture
```

### 統合テスト

```rust
#[test]
fn test_large_payload_compression() {
    let large_text = "x".repeat(3000);
    let payload = StringPayload::new(large_text.clone());
    let packet = UnisonPacket::new(payload).unwrap();
    
    let header = packet.header().unwrap();
    assert!(header.is_compressed());
    assert!(header.compressed_length < header.payload_length);
    
    // ラウンドトリップ
    let bytes = packet.to_bytes();
    let restored = UnisonPacket::<StringPayload>::from_bytes(&bytes).unwrap();
    assert_eq!(restored.payload().unwrap().data, large_text);
}
```

## 今後の拡張計画

### 短期計画
- [ ] フラグメンテーション/リアセンブリ機能
- [ ] ベンチマークスイートの追加
- [ ] 暗号化サポート（AES-GCM）

### 長期計画
- [ ] カスタムシリアライザのサポート
- [ ] 辞書ベースの圧縮最適化
- [ ] ハードウェアアクセラレーション（AES-NI、CRC32C）

## 関連ドキュメント

- [アーキテクチャガイド](./architecture.md)
- [PROTOCOL_SPEC](../spec/PROTOCOL_SPEC.md)
- [API Reference](https://docs.rs/unison)