---
title: DevTools Skills
description: 開発ツールの効果的な使用方法をまとめたスキルセット
version: 1.0.0
author: Vantage Hub Contributors
created_at: 2025-11-01
updated_at: 2025-11-02
tags:
  - devtools
  - development-tools
  - mise
  - chrome-devtools
  - mcp
  - automation
  - testing
  - e2e-testing
categories:
  - skill
  - development-tools
  - automation
---

# DevTools スキル

開発ツールの効果的な使用方法をまとめたスキルセットです。

## 含まれるツール

### 1. mise - 開発環境管理ツール
プロジェクトごとに開発ツールのバージョンを管理し、自動的に切り替えを行います。

**クイックスタート:**
```bash
# ツールをインストール
mise install

# 開発サーバーを起動
mise run dev

# テストを実行
mise run test
```

→ 詳細は [mise リファレンス](reference/mise-reference.md) を参照

### 2. Chrome DevTools MCP サーバー
ブラウザの自動操作とE2Eテストを可能にするMCPサーバーです。

**主なツール:**
- `new_page` - ページを開く
- `take_snapshot` - DOM構造を取得
- `click` - 要素をクリック
- `take_screenshot` - 画面キャプチャ
- `navigate_page` - ページ遷移

**基本的な使い方:**
```json
// ページを開く
mcp__chrome-devtools__new_page
{ "url": "http://localhost:8080" }

// 要素を確認
mcp__chrome-devtools__take_snapshot

// クリック
mcp__chrome-devtools__click
{ "uid": "element_uid" }
```

→ 詳細は [Chrome DevTools MCP リファレンス](reference/chrome-devtools-mcp-reference.md) を参照

## 実践例

- [Webダッシュボードのテスト例](examples/chrome-mcp-dashboard-test.md)
- [mise設定ファイルの例](examples/mise-config.toml)

## いつ使うか

### mise
- ✅ 新しいプロジェクトをセットアップするとき
- ✅ チームで同じツールバージョンを使いたいとき
- ✅ CI/CD環境を構築するとき
- ✅ 複数プロジェクトで異なるバージョンを使うとき

### Chrome DevTools MCP
- ✅ WebUIの動作を確認したいとき
- ✅ E2Eテストを実施したいとき
- ✅ ブラウザでの問題を調査したいとき
- ✅ リリース前の手動テストを効率化したいとき

## トラブルシューティング

### よくある問題

**mise:** ツールバージョンが切り替わらない
```bash
mise doctor  # 診断を実行
eval "$(mise activate bash)"  # シェル統合を再実行
```

**Chrome MCP:** 要素が見つからない
- ページの読み込み完了を待つ
- 複数回スナップショットを取る
- 動的に生成される要素に注意

## 参考資料

- [mise公式ドキュメント](https://mise.jdx.dev/)
- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
- [MCP仕様](https://modelcontextprotocol.io/)