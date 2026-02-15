# DevTools スキル

開発ツールの効果的な使用方法とベストプラクティスをまとめたスキルセットです。

## 概要

このスキルは、Vantage Hubプロジェクトで使用する主要な開発ツールについて、実践的な使い方とベストプラクティスを提供します。

## 含まれるツール

### 1. mise - 開発環境管理ツール
- プロジェクトごとのツールバージョン管理
- 自動的なバージョン切り替え
- タスクランナー機能
- 環境変数管理

### 2. Chrome DevTools MCP サーバー
- ブラウザ自動操作
- E2Eテストの実施
- インタラクティブなデバッグ
- スクリーンショット/スナップショット取得

## ディレクトリ構造

```
devtools/
├── README.md                          # このファイル
├── SKILL.md                          # スキルの概要とクイックスタート
├── reference/                        # 詳細なリファレンスドキュメント
│   ├── mise-reference.md            # mise完全ガイド
│   └── chrome-devtools-mcp-reference.md  # Chrome MCP完全ガイド
└── examples/                         # 実践例とサンプル
    ├── chrome-mcp-dashboard-test.md # E2Eテストの実例
    └── mise-config.toml            # mise設定ファイルの例
```

## クイックスタート

### mise

```bash
# プロジェクトのツールをインストール
mise install

# 開発サーバーを起動
mise run dev

# テストを実行
mise run test
```

### Chrome DevTools MCP

```json
// 1. ページを開く
mcp__chrome-devtools__new_page
{
  "url": "http://localhost:8080"
}

// 2. スナップショットを取得
mcp__chrome-devtools__take_snapshot

// 3. 要素をクリック
mcp__chrome-devtools__click
{
  "uid": "element_uid"
}
```

## ドキュメントナビゲーション

### 初めての方
1. [SKILL.md](SKILL.md) - 基本的な使い方とクイックスタート
2. [examples/](examples/) - 実際の使用例を確認

### 詳しく学びたい方
1. [reference/mise-reference.md](reference/mise-reference.md) - miseの完全ガイド
2. [reference/chrome-devtools-mcp-reference.md](reference/chrome-devtools-mcp-reference.md) - Chrome MCPの完全ガイド

### 実装例を探している方
1. [examples/mise-config.toml](examples/mise-config.toml) - プロジェクト設定の実例
2. [examples/chrome-mcp-dashboard-test.md](examples/chrome-mcp-dashboard-test.md) - E2Eテストシナリオ

## いつ使うか

### mise
- 新しい開発者がプロジェクトに参加するとき
- CI/CD環境をセットアップするとき
- 複数のプロジェクトで異なるツールバージョンを使うとき

### Chrome DevTools MCP
- WebUIの動作確認をするとき
- E2Eテストを実施するとき
- UIのバグを調査するとき
- リリース前の手動テストを行うとき

## 関連リソース

### 公式ドキュメント
- [mise公式ドキュメント](https://mise.jdx.dev/)
- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
- [MCP仕様](https://modelcontextprotocol.io/)

### Vantage Hub関連
- [プロジェクトREADME](../../../../README.md)
- [開発ガイド](../../../../docs/development.md)

## 更新履歴

- 2024-03-XX: 初版作成
- 2024-03-XX: referenceディレクトリ構造に再編成
- 2024-03-XX: Chrome DevTools MCP詳細リファレンスを追加