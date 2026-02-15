# gh コマンドリファレンス

GitHub CLI (gh) の主要コマンド一覧

## 認証関連

### 認証状態の確認

```bash
# 現在の認証状態を表示
gh auth status

# 出力例:
# github.com
#   ✓ Logged in to github.com account user1 (keyring)
#   - Active account: true
#   - Git operations protocol: https
#   - Token scopes: 'repo', 'read:org', 'workflow'
```

### アカウント切り替え

```bash
# 利用可能なアカウント一覧
gh auth status

# アカウント切り替え（GITHUB_TOKEN環境変数がある場合はクリア）
unset GITHUB_TOKEN && gh auth switch --user [ユーザー名]

# 新しいアカウントでログイン
gh auth login

# 権限スコープを更新
gh auth refresh -h github.com -s read:org,repo,workflow
```

## プルリクエスト (gh pr)

### PR作成

```bash
# 基本的な作成
gh pr create

# タイトルと本文を指定
gh pr create --title "タイトル" --body "本文"

# ドラフトPRとして作成
gh pr create --draft

# ブラウザで作成
gh pr create --web

# ベースブランチを指定
gh pr create --base develop

# レビュアーとラベルを指定
gh pr create --reviewer user1,user2 --label bug,high-priority
```

### PR確認

```bash
# PR詳細を表示
gh pr view [PR番号]

# 本文のみ表示
gh pr view [PR番号] --json body -q .body

# ブラウザで開く
gh pr view [PR番号] --web

# マージ可能状態を確認
gh pr view [PR番号] --json mergeable,mergeStateStatus

# ファイル変更を確認
gh pr diff [PR番号]

# チェック状態を確認
gh pr checks [PR番号]
```

### PR一覧

```bash
# 全PR一覧
gh pr list

# 自分のPR一覧
gh pr list --author @me

# 特定ラベルでフィルタ
gh pr list --label bug

# ステート指定
gh pr list --state open
gh pr list --state closed
gh pr list --state merged
gh pr list --state all

# リミット指定
gh pr list --limit 50

# JSON形式で取得
gh pr list --json number,title,state,author
```

### PR編集

```bash
# タイトルを変更
gh pr edit [PR番号] --title "新しいタイトル"

# 本文を変更
gh pr edit [PR番号] --body "新しい本文"

# ファイルから本文を読み込み
gh pr edit [PR番号] --body "$(cat description.md)"

# ラベルを追加
gh pr edit [PR番号] --add-label enhancement,documentation

# ラベルを削除
gh pr edit [PR番号] --remove-label bug

# レビュアーを追加
gh pr edit [PR番号] --add-reviewer user1,user2

# プロジェクトを追加
gh pr edit [PR番号] --add-project "Project Name"

# マイルストーンを設定
gh pr edit [PR番号] --milestone v1.0
```

### PRマージ

```bash
# マージ
gh pr merge [PR番号]

# マージ方法を指定
gh pr merge [PR番号] --merge      # マージコミット作成
gh pr merge [PR番号] --squash     # スカッシュマージ
gh pr merge [PR番号] --rebase     # リベースマージ

# 自動マージ（CI通過後）
gh pr merge [PR番号] --auto --merge

# ブランチを削除
gh pr merge [PR番号] --delete-branch

# マージコミットメッセージを指定
gh pr merge [PR番号] --merge --subject "カスタムメッセージ"
```

### PRクローズ

```bash
# PRをクローズ
gh pr close [PR番号]

# コメント付きでクローズ
gh pr close [PR番号] --comment "理由を説明"

# PRを再オープン
gh pr reopen [PR番号]
```

### PRレビュー

```bash
# レビューを承認
gh pr review [PR番号] --approve

# 変更要求
gh pr review [PR番号] --request-changes --body "変更内容"

# コメント
gh pr review [PR番号] --comment --body "コメント内容"
```

### PRチェックアウト

```bash
# PRをローカルにチェックアウト
gh pr checkout [PR番号]

# ブランチ名指定
gh pr checkout [PR番号] --branch feature-branch
```

## Issue (gh issue)

### Issue作成

```bash
# 対話形式で作成
gh issue create

# タイトルと本文を指定
gh issue create --title "バグ報告" --body "詳細な説明"

# ラベルとアサイン
gh issue create --label bug --assignee @me

# ブラウザで作成
gh issue create --web
```

### Issue確認

```bash
# Issue詳細
gh issue view [Issue番号]

# ブラウザで開く
gh issue view [Issue番号] --web

# コメント表示
gh issue view [Issue番号] --comments
```

### Issue一覧

```bash
# 全Issue一覧
gh issue list

# 自分にアサインされたIssue
gh issue list --assignee @me

# ラベルでフィルタ
gh issue list --label bug,high-priority

# ステート指定
gh issue list --state open
gh issue list --state closed
gh issue list --state all
```

### Issue編集

```bash
# タイトル変更
gh issue edit [Issue番号] --title "新しいタイトル"

# ラベル追加
gh issue edit [Issue番号] --add-label documentation

# アサイン
gh issue edit [Issue番号] --add-assignee user1
```

### Issueクローズ

```bash
# クローズ
gh issue close [Issue番号]

# コメント付きでクローズ
gh issue close [Issue番号] --comment "修正完了"

# 再オープン
gh issue reopen [Issue番号]
```

## リポジトリ (gh repo)

### リポジトリ確認

```bash
# リポジトリ情報表示
gh repo view

# 特定のリポジトリ
gh repo view owner/repo

# ブラウザで開く
gh repo view --web
```

### リポジトリクローン

```bash
# クローン
gh repo clone owner/repo

# ディレクトリ指定
gh repo clone owner/repo target-directory
```

### リポジトリ作成

```bash
# 新規リポジトリ作成
gh repo create repo-name

# プライベートリポジトリ
gh repo create repo-name --private

# 説明付き
gh repo create repo-name --description "リポジトリの説明"
```

## ワークフロー (gh workflow)

### ワークフロー一覧

```bash
# ワークフロー一覧
gh workflow list

# 実行履歴
gh run list

# 特定ワークフローの実行履歴
gh run list --workflow=ci.yml
```

### ワークフロー実行

```bash
# ワークフローを手動実行
gh workflow run workflow-name

# ブランチ指定
gh workflow run workflow-name --ref branch-name
```

### 実行結果確認

```bash
# 最新の実行結果
gh run view

# 特定の実行
gh run view [Run ID]

# ログを表示
gh run view [Run ID] --log

# ブラウザで開く
gh run view [Run ID] --web
```

## JSON出力とクエリ

### 基本的なJSON出力

```bash
# PR情報をJSON形式で取得
gh pr view [PR番号] --json title,body,state,number

# フィールド一覧確認
gh pr view --help | grep "json"
```

### jqとの組み合わせ

```bash
# タイトルのみ抽出
gh pr view [PR番号] --json title | jq -r '.title'

# 複数フィールドを整形
gh pr list --json number,title,author --jq '.[] | "\(.number): \(.title) by \(.author.login)"'

# マージ可能状態のみ確認
gh pr view [PR番号] --json mergeable -q .mergeable
```

### よく使うJSONフィールド

```bash
# PR関連
--json title              # タイトル
--json body               # 本文
--json state              # 状態 (OPEN, CLOSED, MERGED)
--json number             # PR番号
--json mergeable          # マージ可能か (MERGEABLE, CONFLICTING, UNKNOWN)
--json mergeStateStatus   # マージ状態 (CLEAN, DIRTY, UNSTABLE)
--json author             # 作成者
--json reviewDecision     # レビュー判定 (APPROVED, CHANGES_REQUESTED)
--json files              # 変更ファイル一覧
--json statusCheckRollup  # CI/CDステータス

# Issue関連
--json title,body,state,number,author,labels,assignees
```

## エイリアス

よく使うコマンドのエイリアス設定：

```bash
# エイリアス設定
gh alias set pv 'pr view'
gh alias set pc 'pr create'
gh alias set pm 'pr merge --merge'

# 使用例
gh pv 9
gh pc --title "新機能"
gh pm 9
```

## 設定

```bash
# 設定確認
gh config list

# エディタ設定
gh config set editor vim

# ブラウザ設定
gh config set browser firefox

# プロトコル設定
gh config set git_protocol ssh
```

## チートシート

```bash
# よく使うコマンド
gh pr view 9 --json mergeable,mergeStateStatus  # コンフリクト確認
gh pr list --author @me                         # 自分のPR
gh pr edit 9 --body "新しい説明"                 # PR更新
gh pr merge 9 --merge                           # マージ
gh issue list --label bug                       # バグIssue一覧
gh run view --log                               # CI/CDログ

# 認証関連
unset GITHUB_TOKEN && gh auth status            # 認証確認
unset GITHUB_TOKEN && gh auth switch --user X   # アカウント切替

# JSON活用
gh pr view 9 --json body -q .body              # 本文のみ
gh pr list --json number,title --jq '.[] | "\(.number): \(.title)"'
```
