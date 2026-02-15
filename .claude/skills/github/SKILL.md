# GitHub操作スキル - クイックスタート

GitHub CLIを使った実践的なワークフローガイド

## 基本原則

### 1. 認証確認

まず`gh`コマンドの認証状態を確認：

```bash
# 認証状態確認
gh auth status

# アカウント切り替え（必要な場合）
unset GITHUB_TOKEN && gh auth switch --user [ユーザー名]
```

### 2. プルリクエスト操作

#### PR作成

```bash
# 基本的なPR作成
gh pr create --title "タイトル" --body "説明"

# ドラフトPRとして作成
gh pr create --draft --title "WIP: 機能実装中"

# PR作成とブラウザで開く
gh pr create --web
```

#### PR確認

```bash
# PR詳細を表示
gh pr view [番号]

# マージ可能状態を確認
gh pr view [番号] --json mergeable,mergeStateStatus

# PR一覧
gh pr list

# 自分のPR一覧
gh pr list --author @me
```

#### PR編集

```bash
# 説明を更新
gh pr edit [番号] --body "新しい説明"

# タイトルを更新
gh pr edit [番号] --title "新しいタイトル"

# ラベルを追加
gh pr edit [番号] --add-label "enhancement"

# レビュアーを追加
gh pr edit [番号] --add-reviewer ユーザー名
```

### 3. コンフリクト解決ワークフロー

#### ステップ1: コンフリクト確認

```bash
# PRのマージ可能状態を確認
gh pr view [番号] --json mergeable,mergeStateStatus

# 期待される出力:
# {"mergeStateStatus": "DIRTY", "mergeable": "CONFLICTING"}
```

#### ステップ2: 最新のmainをマージ

```bash
# 最新を取得
git fetch origin

# mainブランチをマージ
git merge origin/main

# コンフリクトが表示される
# Auto-merging [ファイル名]
# CONFLICT (content): Merge conflict in [ファイル名]
```

#### ステップ3: コンフリクトファイルを確認

```bash
# コンフリクトしているファイル一覧
git status | grep "both modified"

# または
git diff --name-only --diff-filter=U
```

#### ステップ4: コンフリクトを解決

```bash
# ファイルを編集（マーカーを削除）
# <<<<<<< HEAD
# 自分の変更
# =======
# mainの変更
# >>>>>>> origin/main

# 解決したファイルをステージング
git add [ファイル名]

# すべて解決したらマージコミット
git commit -m "merge: origin/mainとのコンフリクトを解決"

# プッシュ
git push origin [ブランチ名]
```

### 4. Issue操作

```bash
# Issue作成
gh issue create --title "タイトル" --body "説明"

# Issue一覧
gh issue list

# Issue確認
gh issue view [番号]

# Issueを閉じる
gh issue close [番号]
```

### 5. よく使うJSONクエリ

```bash
# PR情報を取得
gh pr view [番号] --json title,body,state,mergeable

# ファイル変更一覧
gh pr view [番号] --json files

# レビューステータス
gh pr view [番号] --json reviewDecision

# CI/CDステータス
gh pr view [番号] --json statusCheckRollup
```

## 実践例

### 例1: PR作成からマージまで

```bash
# 1. ブランチ作成
git checkout -b feature/new-feature

# 2. 変更を加えてコミット
git add .
git commit -m "feat: 新機能追加"

# 3. プッシュ
git push origin feature/new-feature

# 4. PR作成
gh pr create --title "feat: 新機能追加" --body "詳細な説明"

# 5. PR確認
gh pr view

# 6. レビュー後、マージ
gh pr merge --merge
```

### 例2: コンフリクト解決

```bash
# 1. コンフリクト確認
gh pr view 9 --json mergeable,mergeStateStatus

# 2. mainをマージ
git fetch origin
git merge origin/main

# 3. コンフリクトファイルを確認
git status

# 4. 手動で解決してコミット
git add .
git commit -m "merge: コンフリクト解決"
git push

# 5. 解決を確認
gh pr view 9 --json mergeable
```

### 例3: PR説明の更新

```bash
# ファイルから読み込んで更新
gh pr edit 9 --body "$(cat PR_DESCRIPTION.md)"

# または直接指定
gh pr edit 9 --body "## 概要
新機能を追加しました

## 変更内容
- 機能A追加
- バグ修正"
```

## トラブルシューティング

### GITHUB_TOKEN環境変数の問題

```bash
# 環境変数をクリアしてからコマンド実行
unset GITHUB_TOKEN && gh pr view
```

### 権限不足エラー

```bash
# 必要なスコープを追加
gh auth refresh -h github.com -s read:org,repo
```

### アカウント切り替え

```bash
# 利用可能なアカウント確認
gh auth status

# 切り替え
unset GITHUB_TOKEN && gh auth switch --user [ユーザー名]
```

## ベストプラクティス

1. **環境変数のクリア**: `GITHUB_TOKEN`が設定されている場合は`unset`してから操作
2. **JSON出力の活用**: `--json`オプションでプログラマティックな処理が可能
3. **コミットメッセージ規約**: 一貫性のあるプレフィックス（feat, fix, docsなど）を使用
4. **コンフリクト解決**: 早めにmainをマージして大きなコンフリクトを避ける
5. **PRの粒度**: 1つのPRは1つの機能や修正に集中

## 参考リンク

- [GitHub CLI マニュアル](https://cli.github.com/manual/)
- [gh pr コマンドリファレンス](https://cli.github.com/manual/gh_pr)
- [gh issue コマンドリファレンス](https://cli.github.com/manual/gh_issue)
