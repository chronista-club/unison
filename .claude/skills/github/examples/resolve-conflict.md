# コンフリクト解決の実践例

実際のプロジェクトでのコンフリクト解決手順

## シナリオ

PR #9 でmainブランチとのコンフリクトが発生。以下のファイルでコンフリクト：
- `.gitignore`
- `Cargo.toml`
- `Cargo.lock`
- `.vantage/snapshot.yaml`

## ステップバイステップの解決手順

### 1. コンフリクトの確認

```bash
# PRのマージ可能状態を確認
$ unset GITHUB_TOKEN && gh pr view 9 --json mergeable,mergeStateStatus

{
  "mergeStateStatus": "DIRTY",
  "mergeable": "CONFLICTING"
}
```

コンフリクトが確認されました。

### 2. ローカルブランチの準備

```bash
# 現在のブランチを確認
$ git branch --show-current
feature/memory-implementation

# 最新のリモート情報を取得
$ git fetch origin
remote: Enumerating objects: 45, done.
remote: Counting objects: 100% (45/45), done.
...
```

### 3. mainブランチのマージ

```bash
# mainブランチをマージ
$ git merge origin/main

Auto-merging .gitignore
CONFLICT (content): Merge conflict in .gitignore
Auto-merging .vantage/snapshot.yaml
CONFLICT (add/add): Merge conflict in .vantage/snapshot.yaml
Auto-merging Cargo.lock
CONFLICT (content): Merge conflict in Cargo.lock
Auto-merging Cargo.toml
CONFLICT (content): Merge conflict in Cargo.toml
Automatic merge failed; fix conflicts and then commit the result.
```

4つのファイルでコンフリクトが発生しました。

### 4. コンフリクトファイルの確認

```bash
# コンフリクトしているファイルを一覧表示
$ git status
On branch feature/memory-implementation
You have unmerged paths.
  (fix conflicts and run "git commit")
  (use "git merge --abort" to abort the merge)

Unmerged paths:
  (use "git add <file>..." to mark resolution)
	both modified:   .gitignore
	both modified:   .vantage/snapshot.yaml
	both modified:   Cargo.lock
	both modified:   Cargo.toml

# または
$ git diff --name-only --diff-filter=U
.gitignore
.vantage/snapshot.yaml
Cargo.lock
Cargo.toml
```

### 5. 各ファイルの解決

#### 5-1. .gitignoreの解決

```bash
# ファイルを確認
$ cat .gitignore
...
<<<<<<< HEAD
# Qdrant storage
.qdrant_storage/

# SurrealDB storage
.surrealdb_storage/
=======
# Trunk build artifacts
**/dist/
>>>>>>> origin/main
```

**解決方針**: 両方の変更を保持（どちらも無視するべきファイル）

```bash
# エディタで編集
# Qdrant storage
.qdrant_storage/

# SurrealDB storage
.surrealdb_storage/

# Trunk build artifacts
**/dist/

# 解決をステージング
$ git add .gitignore
```

#### 5-2. Cargo.tomlの解決

```bash
# コンフリクト箇所を確認
$ git diff Cargo.toml
```

**解決方針**: 依存関係やメンバーを統合

```bash
# 両方の変更をマージして編集
# コンフリクトマーカー（<<<<<<, =======, >>>>>>>）を削除
# 重複を除いて両方の変更を含める

# 解決をステージング
$ git add Cargo.toml
```

#### 5-3. Cargo.lockの解決

**重要**: Cargo.lockは自動生成ファイルなので、手動編集は避ける

```bash
# マーカーを削除して仮のファイルとして保存
$ git add Cargo.lock

# cargo buildで正しいCargo.lockを再生成
$ cargo build
   Updating crates.io index
   Compiling...

# 再生成されたファイルをステージング
$ git add Cargo.lock
```

#### 5-4. .vantage/snapshot.yamlの解決

**解決方針**: 通常は最新のmainの状態を優先

```bash
# mainの変更を優先
$ git checkout --theirs .vantage/snapshot.yaml

# ステージング
$ git add .vantage/snapshot.yaml
```

別の選択肢：
```bash
# 自分の変更を優先する場合
$ git checkout --ours .vantage/snapshot.yaml

# または手動でマージ
# エディタで編集して両方の変更を統合
```

### 6. すべての解決を確認

```bash
# ステージング状態を確認
$ git status
On branch feature/memory-implementation
All conflicts fixed but you are still merging.
  (use "git commit" to conclude merge)

Changes to be committed:
	modified:   .gitignore
	modified:   .vantage/snapshot.yaml
	modified:   Cargo.lock
	modified:   Cargo.toml
```

すべてのコンフリクトが解決されました。

### 7. マージコミットの作成

```bash
# マージコミットを作成
$ git commit -m "merge: origin/mainとのコンフリクトを解決

- .gitignore: 両方の無視パターンを保持
- Cargo.toml: 依存関係とメンバーを統合
- Cargo.lock: cargo buildで再生成
- .vantage/snapshot.yaml: mainの最新状態を使用"

[feature/memory-implementation abc1234] merge: origin/mainとのコンフリクトを解決
```

### 8. プッシュ

```bash
# リモートにプッシュ
$ git push origin feature/memory-implementation

Enumerating objects: 15, done.
Counting objects: 100% (15/15), done.
Delta compression using up to 8 threads
Compressing objects: 100% (8/8), done.
Writing objects: 100% (8/8), 1.23 KiB | 1.23 MiB/s, done.
Total 8 (delta 5), reused 0 (delta 0), pack-reused 0
To https://github.com/owner/repo.git
   8e24c6a..abc1234  feature/memory-implementation -> feature/memory-implementation
```

### 9. PRの状態を確認

```bash
# マージ可能状態を再確認
$ unset GITHUB_TOKEN && gh pr view 9 --json mergeable,mergeStateStatus

{
  "mergeStateStatus": "CLEAN",
  "mergeable": "MERGEABLE"
}
```

コンフリクトが解決され、マージ可能になりました！

### 10. テストの実行

```bash
# ビルドとテストを実行して問題がないか確認
$ cargo build
   Compiling vantage-memory v0.1.0
   ...
   Finished dev [unoptimized + debuginfo] target(s) in 12.34s

$ cargo test
   ...
   test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo clippy
   ...
   Checking vantage-memory v0.1.0
   Finished dev [unoptimized + debuginfo] target(s) in 5.67s
```

すべてのテストが通過しました。

## まとめ

### 解決にかかった時間

- コンフリクト確認: 1分
- ファイル解決: 5分
- テスト実行: 2分
- **合計: 約8分**

### ポイント

1. **gh pr view**でコンフリクトを早期発見
2. **両方の変更を保持**するか**どちらかを優先**するか判断
3. **自動生成ファイル**（Cargo.lock）は再生成
4. **設定ファイル**（snapshot.yaml）は最新のmainを優先
5. **テスト実行**で問題ないことを確認

### トラブルがあった場合

```bash
# やり直したい場合
$ git merge --abort

# または
$ git reset --hard HEAD

# 最初からマージを試す
$ git fetch origin
$ git merge origin/main
```

## 次のステップ

```bash
# PRをマージする準備ができた
$ unset GITHUB_TOKEN && gh pr merge 9 --merge

# または自動マージ（CI通過後）
$ unset GITHUB_TOKEN && gh pr merge 9 --auto --merge
```
