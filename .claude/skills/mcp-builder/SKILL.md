---
name: mcp-builder
description: LLMが外部サービスと対話するための適切に設計されたツールを通じて、高品質なMCP (Model Context Protocol) サーバーを作成するためのガイド。Python (FastMCP)、Node/TypeScript (MCP SDK)、Rust (Tokio)で、外部APIやサービスを統合するMCPサーバーを構築する際に使用します。
license: Complete terms in LICENSE.txt
---

# MCPサーバー開発ガイド

## 概要

このスキルを使用して、LLMが外部サービスと効果的に対話できる高品質なMCP（Model Context Protocol）サーバーを作成します。MCPサーバーは、LLMが外部サービスやAPIにアクセスできるようにするツールを提供します。MCPサーバーの品質は、提供されたツールを使用してLLMが実際のタスクをどれだけうまく達成できるかによって測定されます。

---

# プロセス

## 🚀 ハイレベルワークフロー

高品質なMCPサーバーの作成には、4つの主要フェーズがあります：

### フェーズ1：徹底的な調査と計画

#### 1.1 エージェント中心設計原則の理解

実装に入る前に、以下の原則を確認してAIエージェント向けのツール設計方法を理解してください：

**APIエンドポイントではなくワークフローを構築する：**
- 既存のAPIエンドポイントを単純にラップするのではなく、思慮深く影響力の高いワークフローツールを構築する
- 関連する操作を統合する（例：空き状況を確認してイベントを作成する`schedule_event`）
- 個々のAPI呼び出しではなく、完全なタスクを可能にするツールに焦点を当てる
- エージェントが実際に達成する必要があるワークフローを検討する

**限られたコンテキストに最適化する：**
- エージェントのコンテキストウィンドウは制限されている - すべてのトークンを有効活用する
- 網羅的なデータダンプではなく、シグナルの高い情報を返す
- 「簡潔」対「詳細」な応答形式オプションを提供する
- 技術的なコード（ID）よりも人間が読める識別子（名前）をデフォルトにする
- エージェントのコンテキスト予算を希少なリソースとして考慮する

**アクション可能なエラーメッセージを設計する：**
- エラーメッセージはエージェントを正しい使用パターンに導くべき
- 具体的な次のステップを提案する：「結果を減らすには filter='active_only' を試してください」
- エラーを診断的ではなく教育的にする
- 明確なフィードバックを通じてエージェントが適切なツールの使用方法を学習できるように支援する

**自然なタスク分割に従う：**
- ツール名は人間がタスクについて考える方法を反映すべき
- 関連するツールには一貫した接頭辞をつけて発見しやすくする
- API構造だけでなく、自然なワークフローを中心にツールを設計する

**評価駆動開発を使用する：**
- 現実的な評価シナリオを早期に作成する
- エージェントのフィードバックをツール改善の原動力とする
- 素早くプロトタイプを作成し、実際のエージェントのパフォーマンスに基づいて反復する

#### 1.3 MCPプロトコルドキュメントの学習

**最新のMCPプロトコルドキュメントを取得：**

WebFetchを使用してロード：`https://modelcontextprotocol.io/llms-full.txt`

この包括的なドキュメントには、完全なMCP仕様とガイドラインが含まれています。

#### 1.4 フレームワークドキュメントの学習

**以下のリファレンスファイルをロードして読む：**

- **MCPベストプラクティス**：[📋 ベストプラクティスを表示](./reference/mcp_best_practices.md) - すべてのMCPサーバーのためのコアガイドライン

**Python実装の場合は、以下も読み込む：**
- **Python SDKドキュメント**：WebFetchを使用して`https://raw.githubusercontent.com/modelcontextprotocol/python-sdk/main/README.md`をロード
- [🐍 Python実装ガイド](./reference/python_mcp_server.md) - Python固有のベストプラクティスと例

**Node/TypeScript実装の場合は、以下も読み込む：**
- **TypeScript SDKドキュメント**：WebFetchを使用して`https://raw.githubusercontent.com/modelcontextprotocol/typescript-sdk/main/README.md`をロード
- [⚡ TypeScript実装ガイド](./reference/node_mcp_server.md) - Node/TypeScript固有のベストプラクティスと例

**Rust実装の場合は、以下も読み込む：**
- [🦀 Rust実装ガイド](./reference/rust_mcp_server.md) - Rust固有のベストプラクティスとTokio非同期パターン

#### 1.5 APIドキュメントの徹底的な学習

サービスを統合するには、利用可能な**すべての**APIドキュメントを読み込む：
- 公式APIリファレンスドキュメント
- 認証と認可の要件
- レート制限とページネーションのパターン
- エラーレスポンスとステータスコード
- 利用可能なエンドポイントとそのパラメータ
- データモデルとスキーマ

**包括的な情報を収集するには、必要に応じてWeb検索とWebFetchツールを使用します。**

#### 1.6 包括的な実装計画の作成

調査に基づいて、以下を含む詳細な計画を作成する：

**ツール選択：**
- 実装する最も価値のあるエンドポイント/操作をリストアップ
- 最も一般的で重要な使用例を可能にするツールを優先する
- 複雑なワークフローを可能にするために一緒に機能するツールを検討する

**共有ユーティリティとヘルパー：**
- 共通のAPIリクエストパターンを特定
- ページネーションヘルパーを計画
- フィルタリングとフォーマッティングユーティリティを設計
- エラー処理戦略を計画

**入力/出力設計：**
- 入力検証モデルを定義（PythonではPydantic、TypeScriptではZod、Rustではserde）
- 一貫したレスポンス形式（例：JSONまたはMarkdown）と設定可能な詳細レベル（例：DetailedまたはConcise）を設計
- 大規模な使用（数千のユーザー/リソース）を計画
- 文字数制限と切り捨て戦略を実装（例：25,000トークン）

**エラー処理戦略：**
- グレースフルな障害モードを計画
- 明確でアクション可能なLLMフレンドリーな自然言語エラーメッセージを設計し、さらなるアクションを促す
- レート制限とタイムアウトのシナリオを考慮
- 認証と認可のエラーを処理

---

### フェーズ2：実装

包括的な計画ができたら、言語固有のベストプラクティスに従って実装を開始します。

#### 2.1 プロジェクト構造のセットアップ

**Pythonの場合：**
- 単一の`.py`ファイルを作成するか、複雑な場合はモジュールに整理（[🐍 Pythonガイド](./reference/python_mcp_server.md)参照）
- ツール登録にMCP Python SDKを使用
- 入力検証用のPydanticモデルを定義

**Node/TypeScriptの場合：**
- 適切なプロジェクト構造を作成（[⚡ TypeScriptガイド](./reference/node_mcp_server.md)参照）
- `package.json`と`tsconfig.json`をセットアップ
- MCP TypeScript SDKを使用
- 入力検証用のZodスキーマを定義

**Rustの場合：**
- Cargoプロジェクトを作成（[🦀 Rustガイド](./reference/rust_mcp_server.md)参照）
- `Cargo.toml`に必要な依存関係を追加（tokio、serde、axum等）
- 非同期ランタイムと適切なトレイト設計を使用
- 入力検証用のserdeモデルを定義

#### 2.2 最初にコアインフラストラクチャを実装

**実装を開始するには、ツールを実装する前に共有ユーティリティを作成する：**
- APIリクエストヘルパー関数
- エラー処理ユーティリティ
- レスポンスフォーマット関数（JSONとMarkdown）
- ページネーションヘルパー
- 認証/トークン管理

#### 2.3 体系的にツールを実装

計画内の各ツールについて：

**入力スキーマを定義：**
- 検証にPydantic（Python）、Zod（TypeScript）、またはserde（Rust）を使用
- 適切な制約を含める（最小/最大長、正規表現パターン、最小/最大値、範囲）
- 明確で説明的なフィールドの説明を提供
- フィールドの説明に多様な例を含める

**包括的なDocstrings/説明を書く：**
- ツールが何をするかの1行の要約
- 目的と機能の詳細な説明
- 例を含む明示的なパラメータタイプ
- 完全な戻り値タイプスキーマ
- 使用例（いつ使用するか、いつ使用しないか）
- 特定のエラーが発生した場合の進め方を説明するエラー処理ドキュメント

**ツールロジックを実装：**
- コード重複を避けるために共有ユーティリティを使用
- すべてのI/Oに対してasync/awaitパターンに従う
- 適切なエラー処理を実装
- 複数のレスポンス形式（JSONとMarkdown）をサポート
- ページネーションパラメータを尊重
- 文字数制限をチェックし、適切に切り捨て

**ツールアノテーションを追加：**
- `readOnlyHint`: true（読み取り専用操作の場合）
- `destructiveHint`: false（非破壊的操作の場合）
- `idempotentHint`: true（繰り返し呼び出しが同じ効果を持つ場合）
- `openWorldHint`: true（外部システムと対話する場合）

#### 2.4 言語固有のベストプラクティスに従う

**この時点で、適切な言語ガイドをロード：**

**Pythonの場合：[🐍 Python実装ガイド](./reference/python_mcp_server.md)をロードして以下を確認：**
- 適切なツール登録でMCP Python SDKを使用
- `model_config`を持つPydantic v2モデル
- 全体を通じて型ヒント
- すべてのI/O操作に対してAsync/await
- 適切なインポートの整理
- モジュールレベルの定数（CHARACTER_LIMIT、API_BASE_URL）

**Node/TypeScriptの場合：[⚡ TypeScript実装ガイド](./reference/node_mcp_server.md)をロードして以下を確認：**
- `server.registerTool`を適切に使用
- `.strict()`を持つZodスキーマ
- TypeScriptの厳密モードを有効化
- `any`型なし - 適切な型を使用
- 明示的なPromise<T>戻り値型
- ビルドプロセスの設定（`npm run build`）

---

### フェーズ3：レビューと改良

初期実装後：

#### 3.1 MCP Inspectorでのテストとデバッグ

**MCP Inspector**は、MCPサーバーのテストとデバッグのための対話型開発ツールです。Web ベースのインターフェースを提供し、インストール不要で使用できます。

**起動方法：**

```bash
# npm パッケージの場合
npx -y @modelcontextprotocol/inspector npx <package-name> <args>

# Python パッケージ（PyPI）の場合
npx @modelcontextprotocol/inspector uvx <package-name> <args>

# ローカルの TypeScript サーバーの場合
npx @modelcontextprotocol/inspector node path/to/server/index.js args...

# ローカルの Python サーバーの場合
npx @modelcontextprotocol/inspector python path/to/server.py args...

# Rust サーバーの場合
npx @modelcontextprotocol/inspector ./target/release/server_name args...
```

**主な機能：**
- **サーバー接続ペイン**: トランスポート方法の設定とコマンドライン引数のカスタマイズ
- **リソースタブ**: 利用可能なリソースの表示とサブスクリプションテスト
- **プロンプトタブ**: プロンプトテンプレートの表示とカスタム入力でのテスト
- **ツールタブ**: ツールのスキーマ表示と実行テスト
- **通知ペイン**: サーバーメッセージと受信通知のログ表示

**開発ワークフロー：**
1. Inspector を起動して接続を確認
2. 変更を反復的にテスト
3. サーバーを再ビルドして再接続
4. 影響を受ける機能を検証しながらメッセージを監視

#### 3.2 コード品質レビュー

品質を確保するため、以下の点でコードをレビュー：
- **DRY原則**：ツール間でコードの重複なし
- **構成可能性**：共有ロジックを関数に抽出
- **一貫性**：類似の操作は類似の形式を返す
- **エラー処理**：すべての外部呼び出しにエラー処理がある
- **型安全性**：完全な型カバレッジ（Python型ヒント、TypeScript型）
- **ドキュメント**：すべてのツールに包括的なdocstrings/説明がある

#### 3.2 テストとビルド

**重要：** MCPサーバーは、stdio/stdinまたはsse/http経由でリクエストを待つ長時間実行プロセスです。メインプロセスで直接実行する（例：`python server.py`や`node dist/index.js`）と、プロセスが無期限にハングします。

**サーバーをテストする安全な方法：**
- 評価ハーネスを使用（フェーズ4参照）- 推奨アプローチ
- tmuxでサーバーを実行してメインプロセスの外に保つ
- テスト時にタイムアウトを使用：`timeout 5s python server.py`

**Pythonの場合：**
- Python構文を検証：`python -m py_compile your_server.py`
- ファイルを確認してインポートが正しく動作することを確認
- 手動でテストするには：tmuxでサーバーを実行し、メインプロセスで評価ハーネスでテスト
- または直接評価ハーネスを使用（stdioトランスポート用にサーバーを管理）

**Node/TypeScriptの場合：**
- `npm run build`を実行してエラーなしで完了することを確認
- dist/index.jsが作成されたことを確認
- 手動でテストするには：tmuxでサーバーを実行し、メインプロセスで評価ハーネスでテスト
- または直接評価ハーネスを使用（stdioトランスポート用にサーバーを管理）

#### 3.3 品質チェックリストの使用

実装品質を確認するには、言語固有のガイドから適切なチェックリストをロード：
- Python：[🐍 Pythonガイド](./reference/python_mcp_server.md)の「品質チェックリスト」参照
- Node/TypeScript：[⚡ TypeScriptガイド](./reference/node_mcp_server.md)の「品質チェックリスト」参照

---

### フェーズ4：評価の作成

MCPサーバーを実装した後、その有効性をテストするための包括的な評価を作成します。

**完全な評価ガイドラインについては[✅ 評価ガイド](./reference/evaluation.md)をロードしてください。**

#### 4.1 評価目的の理解

評価は、LLMがあなたのMCPサーバーを効果的に使用して、現実的で複雑な質問に答えることができるかをテストします。

#### 4.2 評価スクリプトの使用

**TypeScript版評価スクリプト（Bun対応）：**
評価ハーネスは`scripts/`ディレクトリにTypeScript版が用意されています。高速なBunランタイムを使用して実行できます。

```bash
# 依存関係のインストール
cd scripts
bun install

# 評価の実行
bun run evaluation.ts eval.xml -t stdio -c python -a server.py

# またはBunで直接実行
bun run dev eval.xml -t stdio -c node -a dist/index.js
```

詳細は[scripts/README.md](./scripts/README.md)を参照してください。

#### 4.3 10個の評価質問の作成

効果的な評価を作成するには、評価ガイドで説明されているプロセスに従います：

1. **ツール検査**：利用可能なツールをリストアップし、その機能を理解する
2. **コンテンツ探索**：読み取り専用操作を使用して利用可能なデータを探索
3. **質問生成**：10個の複雑で現実的な質問を作成
4. **回答検証**：答えを確認するために各質問を自分で解く

#### 4.3 評価要件

各質問は以下の条件を満たす必要があります：
- **独立性**：他の質問に依存しない
- **読み取り専用**：非破壊的な操作のみが必要
- **複雑性**：複数のツール呼び出しと深い探索が必要
- **現実的**：人間が実際に気にする実際の使用例に基づく
- **検証可能**：文字列比較で検証できる単一の明確な答え
- **安定性**：時間の経過とともに答えが変わらない

#### 4.4 出力形式

この構造でXMLファイルを作成：

```xml
<evaluation>
  <qa_pair>
    <question>動物のコードネームを持つAIモデルの発表に関する議論を見つけてください。あるモデルには、ASL-Xという形式を使用する特定の安全指定が必要でした。斑点のある野生の猫にちなんで名付けられたモデルに対して、どの数字Xが決定されていましたか？</question>
    <answer>3</answer>
  </qa_pair>
<!-- さらに多くのqa_pairs... -->
</evaluation>
```

---

# リファレンスファイル

## 📚 ドキュメントライブラリ

開発中に必要に応じてこれらのリソースをロードしてください：

### コアMCPドキュメント（最初にロード）
- **MCPプロトコル**：`https://modelcontextprotocol.io/llms-full.txt`から取得 - 完全なMCP仕様
- [📋 MCPベストプラクティス](./reference/mcp_best_practices.md) - 以下を含む普遍的なMCPガイドライン：
  - サーバーとツールの命名規則
  - レスポンス形式ガイドライン（JSON対Markdown）
  - ページネーションのベストプラクティス
  - 文字数制限と切り捨て戦略
  - ツール開発ガイドライン
  - セキュリティとエラー処理の標準

### SDKドキュメント（フェーズ1/2でロード）
- **Python SDK**：`https://raw.githubusercontent.com/modelcontextprotocol/python-sdk/main/README.md`から取得
- **TypeScript SDK**：`https://raw.githubusercontent.com/modelcontextprotocol/typescript-sdk/main/README.md`から取得

### 言語固有の実装ガイド（フェーズ2でロード）
- [🐍 Python実装ガイド](./reference/python_mcp_server.md) - 以下を含む完全なPython/FastMCPガイド：
  - サーバー初期化パターン
  - Pydanticモデルの例
  - `@mcp.tool`によるツール登録
  - 完全な実行例
  - 品質チェックリスト

- [⚡ TypeScript実装ガイド](./reference/node_mcp_server.md) - 以下を含む完全なTypeScriptガイド：
  - プロジェクト構造
  - Zodスキーマパターン
  - `server.registerTool`によるツール登録
  - 完全な実行例
  - 品質チェックリスト

### 評価ガイド（フェーズ4でロード）
- [✅ 評価ガイド](./reference/evaluation.md) - 以下を含む完全な評価作成ガイド：
  - 質問作成ガイドライン
  - 回答検証戦略
  - XML形式の仕様
  - 質問と回答の例
  - 提供されたスクリプトでの評価実行
