# ドキュメントフルリライト実装計画

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Unison Protocol の全ドキュメントを現在の実装に合わせてフルリライトする

**Architecture:** 7つのドキュメントを実装順に更新。spec → design → guides → README の順で、下流が上流を参照できるようにする。各タスクは1ファイル = 1コミット。

**Tech Stack:** Markdown, Mermaid diagrams, KDL schema examples

---

### Task 1: spec/01-core-concept/SPEC.md 全面書き直し

**Files:**
- Rewrite: `spec/01-core-concept/SPEC.md`

**概要:** 未実装の3層ネットワーク構想を削除し、実装済みの Stream-First 哲学、Channel型、Identity、QUIC通信を記述する。

**Step 1: SPEC.md を書き直す**

以下の構成で全面書き直し:

```
1. 概要 - Unison Protocol とは
2. 設計思想 - Everything is a Stream
3. 通信モデル - 1 Channel = 1 QUIC Stream
4. Channel型 - Bidirectional / Receive / Request
5. Identity - ServerIdentity によるノード認証
6. QUIC通信 - トランスポート層
7. パケットフォーマット - UnisonPacket（既存§8を維持・更新）
8. セキュリティ - TLS 1.3（既存§10を簡潔化）
9. 今後の拡張 - 3層ネットワーク構想をここに凝縮
10. 関連ドキュメント
```

記述のポイント:
- §2「設計思想」: HoL Blocking 分析図（Mermaid）で Stream 分離の根拠を示す
- §3「通信モデル」: Channel ⇔ QUIC Stream の対応図、length-prefixed framing の図解
- §4「Channel型」: creo_sync.kdl の5チャネルを例に、各型の用途と特性を表で整理
- §5「Identity」: ServerIdentity のシーケンス図（接続→Identity送信→チャネル広告→通信開始）
- §6「QUIC通信」: 既存の QUIC 比較表とストリーム図は良質なので流用。ストリーム予約マップは `__channel:{name}` ルーティングに更新
- §9「今後の拡張」: 旧 spec/01 の3層ネットワーク・Discovery・IPv6 ULA を5-10行に凝縮

ソースから参照すべき型情報:
- `ServerIdentity` (identity.rs): name, version, namespace, channels, metadata
- `ChannelInfo` (identity.rs): name, direction, lifetime, status
- `ChannelDirection`: ServerToClient, ClientToServer, Bidirectional
- `ConnectionContext` (context.rs): connection_id, identity, channels
- `read_frame`/`write_frame` (quic.rs): 4バイト BE length + data, max 8MB

**Step 2: 検証**

Run: `mise x rust@1.93 -- cargo test --tests --manifest-path Cargo.toml -- --skip packet 2>&1 | tail -5`
Expected: テストが引き続きパスすること（ドキュメント変更のみ）

**Step 3: コミット**

```bash
git add spec/01-core-concept/SPEC.md
git commit -m "docs: spec/01 を実装ベースに全面書き直し

Stream-First 哲学、Channel型、Identity、QUIC通信を記述。
未実装の3層ネットワーク構想はロードマップに凝縮。"
```

---

### Task 2: design/architecture.md 全面書き直し

**Files:**
- Rewrite: `design/architecture.md`

**概要:** 汎用的な5層アーキテクチャを、実際のモジュール構成に合わせて書き直す。

**Step 1: architecture.md を書き直す**

以下の構成:

```
1. 概要
2. ワークスペース構成 - crates/ の全体図
3. unison-protocol モジュール構成
   - core/ (プロトコル定義)
   - parser/ (KDL解析)
   - codegen/ (Rust/TS コード生成)
   - packet/ (UnisonPacket: rkyv + zstd)
   - network/ (QUIC transport, Channel, Identity, Context, Client, Server)
   - context/ (CGP)
4. データフロー
   - RPC フロー: Client → ProtocolMessage → QUIC Stream → Server → Handler
   - Channel フロー: open_channel → __channel:{name} → QuicBackedChannel<S,R>
   - Identity フロー: connect → ServerIdentity → channel一覧 → open_channel
5. エラーハンドリング - NetworkError enum の全バリアント
6. 拡張ポイント - トレイト一覧 (UnisonClient, UnisonServer, SystemStream 等)
```

記述のポイント:
- §3: 実際の `src/network/` 配下のファイル一覧と各ファイルの責務を表で整理
- §4: Mermaid シーケンス図で3つのフロー（RPC, Channel, Identity）を図解
- §6: 各トレイトのシグネチャを Rust コードブロックで記載

ソースから参照すべき情報:
- `mod.rs` の public exports と trait 定義
- `NetworkError` の全バリアント: Connection, Protocol, Serialization, FrameSerialization, Quic, Timeout, HandlerNotFound, NotConnected, UnsupportedTransport
- `ProtocolMessage`: id, method, msg_type, payload
- `MessageType` enum: Request, Response, Stream, StreamData, StreamEnd, StreamError, BidirectionalStream, StreamSend, StreamReceive, Error

**Step 2: コミット**

```bash
git add design/architecture.md
git commit -m "docs: design/architecture.md を実装ベースに全面書き直し

実際のモジュール構成、データフロー、トレイト一覧を記述。"
```

---

### Task 3: design/quic-runtime.md 新規作成

**Files:**
- Create: `design/quic-runtime.md`

**概要:** PR #13 で実装した QUIC Runtime 統合の設計を記録する。

**Step 1: quic-runtime.md を作成する**

以下の構成:

```
1. 概要
2. ConnectionContext
   - ライフサイクル図（Mermaid statechart）
   - connection_id (UUID)、identity、channels の管理
3. Identity Handshake
   - シーケンス図: サーバーが接続直後に ServerIdentity を送信
   - ServerIdentity の構造 (name, version, namespace, channels)
4. チャネルルーティング
   - `__channel:{name}` プレフィックスによるRPCとの分離
   - フロー図: accept_bi → read_frame → prefix判定 → channel handler or RPC handler
5. Length-Prefixed Framing
   - read_frame: 4バイト BE length → data 読み取り (max 8MB)
   - write_frame: 4バイト BE length → data 書き込み
   - 後方互換: read_frame 失敗時は read_to_end にフォールバック
6. QuicBackedChannel<S, R>
   - UnisonStream をラップした型安全チャネル
   - PhantomData + Serialize/DeserializeOwned で型パラメータ
   - send/recv/close/is_active メソッド
7. コード生成統合
   - {Protocol}QuicConnection 構造体の自動生成
   - {Protocol}ConnectionBuilder トレイトの自動生成
   - channel_quic_field_type() のマッピング表
```

ソースから参照すべき情報:
- `handle_connection()` のフロー (quic.rs)
- `QuicBackedChannel<S, R>` の実装 (channel.rs)
- `codegen/rust.rs` の `generate_connection_struct()` と `channel_quic_field_type()`

**Step 2: コミット**

```bash
git add design/quic-runtime.md
git commit -m "docs: QUIC Runtime 統合の設計ドキュメントを新規作成

ConnectionContext, Identity Handshake, チャネルルーティング,
QuicBackedChannel の設計を記述。"
```

---

### Task 4: spec/03-stream-channels/SPEC.md 新規作成

**Files:**
- Create: `spec/03-stream-channels/SPEC.md`

**概要:** Stream-First API のチャネル仕様を記述する。

**Step 1: ディレクトリ作成 & SPEC.md を作成する**

```bash
mkdir -p spec/03-stream-channels
```

以下の構成:

```
1. 概要
2. チャネル型一覧
   - BidirectionalChannel<S, R> - 双方向、persistent
   - ReceiveChannel<T> - サーバー→クライアント push
   - RequestChannel<Req, Res> - transient RPC (oneshot)
   - StreamSender<T> / StreamReceiver<T> - インメモリチャネル
3. KDL スキーマ構文
   - `channel "name" direction="..." lifetime="..." { ... }` の完全構文
   - direction: "client_to_server" | "server_to_client" | "either"
   - lifetime: "persistent" | "transient"
   - send / recv / error ブロック
4. スキーマ例: creo_sync.kdl
   - 5つのチャネル定義を解説（control, events, query, messaging, urgent）
5. QuicBackedChannel<S, R>
   - QUIC stream 上での型安全チャネル
   - open → send/recv → close のライフサイクル
6. チャネルルーティングプロトコル
   - `__channel:{name}` フレーム送信でチャネルオープン
   - サーバー側: ChannelHandler への dispatch
```

ソースから参照すべき情報:
- channel.rs の全 public 型
- schemas/creo_sync.kdl のスキーマ定義
- identity.rs の ChannelInfo, ChannelDirection

**Step 2: コミット**

```bash
git add spec/03-stream-channels/
git commit -m "docs: Stream-First API チャネル仕様を新規作成

BidirectionalChannel, ReceiveChannel, RequestChannel の仕様、
KDL構文、creo_sync.kdl 解説、ルーティングプロトコルを記述。"
```

---

### Task 5: spec/02-protocol-rpc/SPEC.md 部分更新

**Files:**
- Modify: `spec/02-protocol-rpc/SPEC.md`

**概要:** 既存の RPC 仕様に Channel 関連の構文を追加する。

**Step 1: 以下のセクションを追加・更新**

追加箇所:
- §4.4「Channel 定義構文」を新設 — `channel` キーワードの構文、direction, lifetime 属性
- §4.3「スキーマ例」に creo_sync.kdl の抜粋を追加（channel 定義の実例）
- §6「コード生成」に §6.3「Channel型 コード生成」を追加
  - Rust: `QuicBackedChannel<SendType, RecvType>` への変換
  - `{Protocol}QuicConnection` / `{Protocol}ConnectionBuilder` の生成

更新箇所:
- §10.1「計画中の機能」から「ストリーミングサポート」を削除（実装済みのため）
- 最終更新日を更新

**Step 2: コミット**

```bash
git add spec/02-protocol-rpc/SPEC.md
git commit -m "docs: spec/02 に channel 構文と codegen 説明を追加

KDL channel キーワードの構文仕様、creo_sync.kdl 例、
Channel型コード生成の説明を追加。"
```

---

### Task 6: guides/channel-guide.md 新規作成

**Files:**
- Create: `guides/channel-guide.md`

**概要:** チャネル機能の使い方ガイド。開発者が最初に読むドキュメント。

**Step 1: channel-guide.md を作成する**

以下の構成:

```
1. はじめに - チャネルとは何か（1段落）
2. クイックスタート
   2.1 KDL スキーマでチャネルを定義する（5行のKDLコード）
   2.2 サーバー側: チャネルハンドラを登録する
       - server.register_channel("events", handler) のコード例
   2.3 クライアント側: チャネルを開く
       - client.open_channel::<SendType, RecvType>("events") のコード例
   2.4 データの送受信
       - channel.send(msg) / channel.recv() のコード例
3. チャネル型の選び方
   - ユースケース→チャネル型の対応表
   - 判断フローチャート（Mermaid）
4. 高度な使用法
   - ConnectionBuilder トレイトによる一括チャネルオープン
   - Identity Handshake でサーバーが広告するチャネル一覧の取得
```

ソースから参照すべき情報:
- client.rs の `open_channel()` シグネチャ
- server.rs の `register_channel()` シグネチャ
- channel.rs の `QuicBackedChannel` メソッド
- schemas/creo_sync.kdl のスキーマ例

**Step 2: コミット**

```bash
git add guides/channel-guide.md
git commit -m "docs: チャネル使用ガイドを新規作成

KDLスキーマ定義、サーバー/クライアント実装例、
チャネル型選択フローチャートを記述。"
```

---

### Task 7: README.md 全面書き直し

**Files:**
- Rewrite: `README.md`

**概要:** README を現在の実装に合わせて全面更新する。

**Step 1: README.md を書き直す**

更新ポイント:
- MSRV: 1.70 → **1.93**
- Rust edition: **2024** を明記
- 「主要機能」に **Stream-First API** と **Identity Handshake** を追加
- 「クイックスタート」に **チャネル通信例** を追加（KDLスキーマ → サーバー → クライアント）
- 「アーキテクチャ」のコンポーネント構造を実際のモジュールに合わせる
- 「コアコンポーネント」に `QuicBackedChannel<S, R>` を追加
- 「ドキュメント」セクションに spec/03, design/quic-runtime.md, guides/channel-guide.md を追加
- 「開発環境のセットアップ」の Rust バージョン要件を更新
- 「今後の展望」セクション新設: WASM/SDK 化の可能性（WebSocket transport, wasm-bindgen, tsify）

既存で維持するもの:
- バッジ、ライセンス、コントリビューションガイドライン
- UnisonPacket / CGP の説明
- macOS lld の注意書き

**Step 2: コミット**

```bash
git add README.md
git commit -m "docs: README.md を実装ベースに全面書き直し

MSRV 1.93, Stream-First API, チャネル通信例,
WASM/SDK 展望を追加。"
```

---

### Task 8: PR 作成

**Step 1: ブランチ作成 & push**

```bash
git checkout -b docs/full-rewrite
git push -u origin docs/full-rewrite
```

**Step 2: PR 作成**

```bash
gh pr create --title "docs: ドキュメントフルリライト（実装ベース）" --body "$(cat <<'EOF'
## Summary
- spec/01 を全面書き直し: Stream-First哲学、Channel型、Identity
- design/architecture.md を実装ベースに全面書き直し
- design/quic-runtime.md を新規作成
- spec/03-stream-channels/SPEC.md を新規作成
- spec/02 に channel 構文を追加
- guides/channel-guide.md を新規作成
- README.md を全面書き直し (MSRV 1.93, WASM展望)

## Test plan
- [ ] Mermaid ダイアグラムが GitHub 上で正しくレンダリングされること
- [ ] 全ドキュメントのリンクが有効であること
- [ ] コード例がコンパイル可能であること（将来的に doctest 化）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## 注意事項

- **ドキュメント変更のみ**のため、Rust のテストは変更前後で同じ結果になるはず
- spec/01 の旧内容（3層ネットワーク等）は git 履歴に残るので、完全削除で問題ない
- Mermaid ダイアグラムは GitHub のネイティブレンダリングに依存
- 日本語が第一言語。技術用語は英語のまま使用
- コード内の型名・関数名は正確にソースから引用すること
