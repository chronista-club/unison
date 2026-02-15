# GitHub スキル

`gh`コマンドを使用したGitHub操作の実践的なスキルセット

## 概要

このスキルは、GitHub CLIツール（`gh`コマンド）を使用して、プルリクエスト、Issue、マージコンフリクトなどのGitHub関連タスクを効率的に処理する方法を提供します。

## 主な機能

- プルリクエストの作成・編集・確認
- Issueの管理
- マージコンフリクトの検出と解決
- リポジトリ情報の取得
- レビュー管理

## 使用場面

- PRの作成時や更新時
- コンフリクト解決が必要な時
- Issue管理
- リポジトリの状態確認

## ファイル構成

```
github/
├── README.md           # このファイル
├── SKILL.md           # クイックスタート
├── reference/
│   ├── pr-workflow.md      # PRワークフロー
│   ├── conflict-resolution.md  # コンフリクト解決
│   └── gh-commands.md      # ghコマンドリファレンス
└── examples/
    ├── create-pr.md        # PR作成例
    └── resolve-conflict.md # コンフリクト解決例
```

## クイックスタート

```bash
# PR作成
gh pr create --title "タイトル" --body "説明"

# PR確認
gh pr view [番号]

# PR編集
gh pr edit [番号] --body "新しい説明"

# コンフリクト確認
gh pr view [番号] --json mergeable,mergeStateStatus
```

## 関連ドキュメント

- [GitHub CLI公式ドキュメント](https://cli.github.com/manual/)
- [PRワークフロー](reference/pr-workflow.md)
- [コンフリクト解決](reference/conflict-resolution.md)
