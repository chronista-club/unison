# MCP Server Evaluation Scripts

MCPサーバーの評価とテストを行うためのTypeScriptスクリプト集です。Bunランタイムを使用して高速実行を実現しています。

## 🚀 セットアップ

### 前提条件

- [Bun](https://bun.sh/) がインストールされていること
- `ANTHROPIC_API_KEY` 環境変数が設定されていること

### インストール

```bash
# 依存関係のインストール
bun install
```

## 📁 ファイル構成

- `evaluation.ts` - MCPサーバー評価ハーネスのメインスクリプト
- `connections.ts` - MCPサーバーへの接続管理（stdio/SSE/HTTP）
- `package.json` - プロジェクト設定と依存関係
- `tsconfig.json` - TypeScript設定（Bun用に最適化）

## 🎯 使用方法

### 評価スクリプトの実行

```bash
# 開発モード（直接実行）
bun run evaluation.ts <eval-file> [options]

# または npm スクリプト経由
bun run dev <eval-file> [options]

# ビルドして実行
bun run build
bun run evaluate <eval-file> [options]
```

### コマンドラインオプション

```bash
評価スクリプトのオプション:
  -t, --transport <type>      トランスポートタイプ (stdio/sse/http)
  -m, --model <model>         使用するClaude モデル (デフォルト: claude-3-5-sonnet-20241022)
  -c, --command <command>     MCPサーバーを起動するコマンド (stdioのみ)
  -a, --args <args...>        コマンドの引数 (stdioのみ)
  -e, --env <env...>          環境変数 KEY=VALUE形式 (stdioのみ)
  -u, --url <url>             MCPサーバーURL (sse/httpのみ)
  -H, --header <headers...>   HTTPヘッダー "Key: Value"形式 (sse/httpのみ)
  -o, --output <file>         評価レポートの出力ファイル (デフォルト: stdout)
```

### 実行例

```bash
# ローカルのstdio MCPサーバーを評価
bun run evaluation.ts eval.xml -t stdio -c python -a my_server.py

# SSE MCPサーバーを評価
bun run evaluation.ts eval.xml -t sse -u https://example.com/mcp -H "Authorization: Bearer token"

# カスタムモデルで評価
bun run evaluation.ts eval.xml -t stdio -c node -a server.js -m claude-3-opus-20240229

# レポートをファイルに保存
bun run evaluation.ts eval.xml -t stdio -c bun -a server.ts -o report.md
```

## 📝 評価ファイル形式

評価ファイルはXML形式で、質問と期待される回答のペアを定義します：

```xml
<evaluation>
  <qa_pair>
    <question>2 + 2 を計算してください</question>
    <answer>4</answer>
  </qa_pair>
  <qa_pair>
    <question>現在の時刻を教えてください</question>
    <answer>NOT_FOUND</answer>
  </qa_pair>
</evaluation>
```

## 📊 出力形式

評価レポートには以下の情報が含まれます：

- **精度**: 正解率とスコア
- **パフォーマンス**: 各タスクの実行時間
- **ツール使用状況**: 呼び出されたツールと回数
- **詳細なフィードバック**: 各ツールの使いやすさに関するAIの評価

## 🔧 開発

### TypeScriptのコンパイル

```bash
# TypeScriptファイルをコンパイル（型チェックのみ）
bun run tsc --noEmit

# ビルド（Bunのビルダーを使用）
bun run build
```

### テスト実行

```bash
bun test
```

## 🛠️ トラブルシューティング

### Bunタイプが見つからない場合

```bash
# bun-typesを再インストール
bun add -d bun-types
```

### MCPサーバーへの接続エラー

- stdioの場合: コマンドとパスが正しいか確認
- SSE/HTTPの場合: URLとヘッダーが正しいか確認
- 環境変数が正しく設定されているか確認

## 📚 関連ドキュメント

- [MCP仕様](https://github.com/modelcontextprotocol/specification)
- [Bun ドキュメント](https://bun.sh/docs)
- [TypeScript ハンドブック](https://www.typescriptlang.org/docs/)

## 🎯 Bunを使用するメリット

1. **高速起動**: Node.jsと比較して起動時間が大幅に短縮
2. **ネイティブTypeScript**: トランスパイル不要で直接実行可能
3. **統合ツール**: ビルド、テスト、パッケージ管理が一体化
4. **メモリ効率**: より少ないメモリ使用量で実行可能

## 🔄 Python版からの移行

このTypeScript実装は、元のPython実装（`evaluation.py`, `connections.py`）と完全な互換性を保ちながら、以下の改善を提供します：

- 型安全性の向上
- より高速な実行
- モダンなasync/await構文
- 公式MCP SDKの活用