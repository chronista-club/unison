# ドキュメントフルリライト設計

最終更新日: 2026-02-16

## 背景

PR #12 (Stream-First API) と PR #13 (QUIC Runtime 統合) のマージにより、Unison Protocol の実装は大きく進化した。しかし、既存ドキュメントは初期構想（3層ネットワーク、mDNS Discovery等）を記述しており、実装との乖離が深刻。

## 原則

**コードが正。ドキュメントをコードに合わせる。**

- 実装済み機能のみを仕様として記述
- 未実装の構想はロードマップセクションに集約
- Living Documentation: コードと常に同期

## ギャップ分析

### 実装済みだがドキュメント未記載

| 機能 | PR | 現状 |
|------|-----|------|
| Stream-First API (StreamSender/Receiver/BidirectionalChannel) | #12 | spec/design/guide すべて未記載 |
| Identity Channel (ServerIdentity, 認証フロー) | #12 | 未記載 |
| Channel型 KDL構文 (`channel` キーワード) | #12 | 未記載 |
| QUIC Runtime (QuicClient/Server/ConnectionContext) | #13 | 未記載 |
| QuicBackedChannel<S, R> | #13 | 未記載 |
| チャネルルーティング (`__channel:{name}`) | #13 | 未記載 |
| Length-prefixed framing (read_frame/write_frame) | #13 | 未記載 |

### ドキュメント記載済みだが未実装

| 機能 | 記載箇所 |
|------|---------|
| 3層アーキテクチャ (Agent/Hub/Root) | spec/01 全体 |
| mDNS / Bootstrap Discovery | spec/01 §5 |
| IPv6 ULA アドレッシング | spec/01 §6 |
| Hub/Root 障害復旧 | spec/01 §9 |
| WebSocketTransport 構造体 | design/architecture.md |

## 新しいドキュメント構成

```
spec/
├── 01-core-concept/SPEC.md    # 全面書き直し: Stream-First哲学、Channel型、Identity
├── 02-protocol-rpc/SPEC.md    # 部分更新: channel構文追加
└── 03-stream-channels/SPEC.md # 新規: チャネルAPI仕様

design/
├── architecture.md            # 全面書き直し: 実装ベースのモジュール構成
├── packet.md                  # 維持（実装と一致）
└── quic-runtime.md            # 新規: QUIC統合設計

guides/
├── quinn-stream-api.md        # 維持
└── channel-guide.md           # 新規: チャネル使用ガイド

README.md                      # 全面書き直し: MSRV更新、Stream-First、WASM展望
```

## 各ファイルの詳細方針

### spec/01-core-concept/SPEC.md（全面書き直し）

削除する内容:
- 3層アーキテクチャ図と説明（§3全体）
- 起動シーケンス（§4: Agent/Hub/Root起動フロー）
- ディスカバリー機構（§5: mDNS, Bootstrap）
- Network IDとアドレッシング（§6: IPv6 ULA構造）
- 障害時の動作（§9: Hub/Root障害フロー）

新規記述:
- Everything is a Stream 哲学
- 通信モデル: 1 Channel = 1 QUIC Stream
- HoL Blocking分析とStream分離の根拠
- Channel型定義: Bidirectional / Receive / Request
- Identity: ServerIdentity によるノード認証
- 接続ライフサイクル
- Length-prefixed framing

ロードマップに移動:
- 3層ネットワーク構想 → 「今後の拡張」に凝縮（5行程度）

### spec/02-protocol-rpc/SPEC.md（部分更新）

維持: §1-§9（RPC仕様は実装と概ね一致）

追加:
- §4に `channel` キーワード構文
- creo_sync.kdl を公式スキーマ例として追加
- §6に Channel型の codegen 説明

### spec/03-stream-channels/SPEC.md（新規）

- StreamSender<T> / StreamReceiver<T> API仕様
- BidirectionalChannel<T> API仕様
- QuicBackedChannel<S, R> API仕様
- `__channel:{name}` ルーティングプロトコル
- チャネルライフサイクル（open → send/recv → close）

### design/architecture.md（全面書き直し）

実際のモジュール構成:
```
unison-protocol/
├── core/        # プロトコル定義
├── parser/      # KDL解析
├── codegen/     # Rust/TS コード生成
├── packet/      # UnisonPacket (rkyv + zstd)
├── network/
│   ├── quic.rs      # QUIC transport (quinn)
│   ├── channel.rs   # Channel プリミティブ
│   ├── identity.rs  # Identity 認証
│   ├── context.rs   # ConnectionContext
│   ├── client.rs    # ProtocolClient
│   └── server.rs    # ProtocolServer
└── context/     # CGP
```

### design/quic-runtime.md（新規）

- ConnectionContext のライフサイクル
- Identity Handshake シーケンス図
- チャネルルーティングフロー
- read_frame / write_frame framing仕様
- 後方互換性（read_to_end fallback）

### guides/channel-guide.md（新規）

- チャネルの作り方（KDLスキーマ定義）
- サーバー側: チャネルハンドラ登録
- クライアント側: open_channel() の使い方
- E2Eサンプルコード

### README.md（全面書き直し）

- MSRV: 1.70 → 1.93
- Rust 2024 edition 明記
- Stream-First API の紹介文
- チャネル通信コード例の追加
- WASM/SDK 展望セクション
- 既存の良い構造（クイックスタート等）は活かしつつ更新

## WASM/SDK 調査結果（参考）

技術的に実現可能。packet/, parser/, core/ はトランスポート非依存で WASM 互換。
詳細は creo-memories に記録済み。README に展望として記載。

## 実装順序

1. spec/01 全面書き直し（最重要：現在のコアコンセプトの正確な記述）
2. design/architecture.md 全面書き直し
3. design/quic-runtime.md 新規作成
4. spec/03 新規作成
5. spec/02 部分更新
6. guides/channel-guide.md 新規作成
7. README.md 全面書き直し
