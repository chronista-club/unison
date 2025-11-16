# マージコンフリクト解決ガイド

GitHubでのマージコンフリクトを効率的に解決する方法

## コンフリクトとは

同じファイルの同じ箇所に異なる変更が加えられた時に発生します。Gitは自動的にマージできないため、手動での解決が必要です。

## コンフリクト検出

### ghコマンドでPRのコンフリクトを確認

```bash
# PRのマージ可能状態を確認
gh pr view [PR番号] --json mergeable,mergeStateStatus

# 出力例（コンフリクトあり）:
# {
#   "mergeStateStatus": "DIRTY",
#   "mergeable": "CONFLICTING"
# }

# 出力例（コンフリクトなし）:
# {
#   "mergeStateStatus": "CLEAN",
#   "mergeable": "MERGEABLE"
# }
```

### Gitコマンドでコンフリクトを確認

```bash
# コンフリクトしているファイルを一覧表示
git status

# コンフリクトファイルのみ表示
git diff --name-only --diff-filter=U

# 詳細な差分を表示
git diff
```

## コンフリクト解決の基本フロー

### ステップ1: 最新のmainブランチを取得

```bash
# リモートの最新を取得
git fetch origin

# 現在のブランチを確認
git branch --show-current

# mainブランチの最新をマージ
git merge origin/main
```

### ステップ2: コンフリクトの確認

マージ時に以下のようなメッセージが表示されます：

```
Auto-merging Cargo.toml
CONFLICT (content): Merge conflict in Cargo.toml
Auto-merging .gitignore
CONFLICT (content): Merge conflict in .gitignore
Automatic merge failed; fix conflicts and then commit the result.
```

### ステップ3: コンフリクトマーカーの理解

コンフリクトしているファイルを開くと、以下のマーカーが見つかります：

```
<<<<<<< HEAD
// 自分のブランチの変更
=======
// mainブランチの変更
>>>>>>> origin/main
```

**マーカーの意味:**
- `<<<<<<< HEAD`: 自分のブランチの変更の開始
- `=======`: 自分の変更とmainの変更の境界
- `>>>>>>> origin/main`: mainブランチの変更の終了

### ステップ4: コンフリクトの解決方法

#### パターン1: 両方の変更を保持

```bash
# 例: .gitignore での追加項目
<<<<<<< HEAD
# Qdrant storage
.qdrant_storage/

# SurrealDB storage
.surrealdb_storage/
=======
# Trunk build artifacts
**/dist/
>>>>>>> origin/main

# 解決後（両方を残す）:
# Qdrant storage
.qdrant_storage/

# SurrealDB storage
.surrealdb_storage/

# Trunk build artifacts
**/dist/
```

#### パターン2: 自分の変更を優先

```bash
# マーカーとmainの変更を削除
<<<<<<< HEAD
自分の変更
=======
mainの変更  # この行を削除
>>>>>>> origin/main  # マーカーも削除

# 解決後:
自分の変更
```

#### パターン3: mainの変更を優先

```bash
# マーカーと自分の変更を削除
<<<<<<< HEAD  # マーカーを削除
自分の変更  # この行を削除
=======
mainの変更
>>>>>>> origin/main  # マーカーも削除

# 解決後:
mainの変更
```

#### パターン4: 両方をマージして新しい内容に

```bash
# 例: Cargo.toml の依存関係
<<<<<<< HEAD
dependencies = ["dep-a", "dep-b"]
=======
dependencies = ["dep-c", "dep-d"]
>>>>>>> origin/main

# 解決後（すべてを含む）:
dependencies = ["dep-a", "dep-b", "dep-c", "dep-d"]
```

### ステップ5: 解決をステージング

```bash
# 解決したファイルをステージング
git add [ファイル名]

# または全ファイルを一度にステージング
git add .

# ステージング状態を確認
git status
```

### ステップ6: マージコミット

```bash
# マージコミットを作成
git commit -m "merge: origin/mainとのコンフリクトを解決"

# または詳細なメッセージで
git commit -m "merge: origin/mainとのコンフリクトを解決

- .gitignore: 両方の変更を保持
- Cargo.toml: 依存関係をマージ
- Cargo.lock: cargo buildで自動更新"
```

### ステップ7: プッシュと確認

```bash
# リモートにプッシュ
git push origin [ブランチ名]

# PRの状態を確認
gh pr view [PR番号] --json mergeable,mergeStateStatus

# 期待される結果:
# {
#   "mergeStateStatus": "CLEAN",
#   "mergeable": "MERGEABLE"
# }
```

## 特殊なケースの解決

### Cargo.lockのコンフリクト

Cargo.lockは自動生成ファイルなので、手動編集は避けるべきです：

```bash
# 1. マーカーを削除して両方の変更を一旦受け入れる
git add Cargo.lock

# 2. cargo buildで再生成
cargo build

# 3. 再生成されたファイルをコミット
git add Cargo.lock
git commit --amend --no-edit
```

### .vantage/snapshot.yamlのコンフリクト

スナップショットファイルは通常、最新の状態を保持します：

```bash
# mainの変更を優先（最新の状態）
git checkout --theirs .vantage/snapshot.yaml
git add .vantage/snapshot.yaml

# または自分の変更を優先
git checkout --ours .vantage/snapshot.yaml
git add .vantage/snapshot.yaml
```

### 大規模なコンフリクト

多数のファイルでコンフリクトが発生した場合：

```bash
# コンフリクトファイルの数を確認
git diff --name-only --diff-filter=U | wc -l

# マージを中止して戦略を変更
git merge --abort

# リベースを試す（履歴が線形になる）
git rebase origin/main

# コンフリクトを順番に解決
git add [解決したファイル]
git rebase --continue
```

## ghコマンドを使った効率的なワークフロー

```bash
# 1. コンフリクトを検出
gh pr view 9 --json mergeable,mergeStateStatus

# 2. PR情報を確認
gh pr view 9

# 3. ブラウザで開いてファイル差分を確認
gh pr view 9 --web

# 4. 解決後、PRのチェックを確認
gh pr checks 9

# 5. 自動でマージ（CI/CDが通った後）
gh pr merge 9 --auto --merge
```

## トラブルシューティング

### コンフリクトマーカーが残っている

```bash
# コンフリクトマーカーを検索
git grep -n "<<<<<<< HEAD"
git grep -n "======="
git grep -n ">>>>>>> origin/main"

# 見つかった場合は手動で削除
```

### マージを途中でやり直したい

```bash
# マージを中止
git merge --abort

# または
git reset --hard HEAD

# 最初からやり直す
git fetch origin
git merge origin/main
```

### コミット前に間違いに気づいた

```bash
# ステージングを解除
git reset HEAD [ファイル名]

# 作業ディレクトリの変更も戻す
git checkout -- [ファイル名]
```

## ベストプラクティス

1. **頻繁にmainをマージ**: 大きなコンフリクトを避けるため、定期的にmainの変更を取り込む

2. **小さな変更でPR**: 大きなPRはコンフリクトが起きやすく、解決も困難

3. **自動生成ファイル**: Cargo.lockなどは手動編集せず、ツールで再生成

4. **チーム連携**: 同じファイルを編集する場合は事前に調整

5. **テストの実行**: コンフリクト解決後は必ずテストを実行

```bash
# 解決後のテスト
cargo test
cargo build
cargo clippy
```

6. **レビュー依頼**: 複雑なコンフリクト解決の場合は、マージ前にレビューを依頼

```bash
# レビュアーを追加
gh pr edit [PR番号] --add-reviewer [ユーザー名]
```

## チートシート

```bash
# コンフリクト確認
gh pr view [PR番号] --json mergeable
git status

# 解決フロー
git fetch origin
git merge origin/main
# ファイルを手動編集
git add .
git commit -m "merge: コンフリクト解決"
git push

# 確認
gh pr view [PR番号] --json mergeable

# やり直し
git merge --abort
git reset --hard HEAD

# ショートカット
git checkout --ours [ファイル]   # 自分の変更を優先
git checkout --theirs [ファイル] # mainの変更を優先
```
