# mise リファレンス

## 概要

mise（旧rtx）は、プロジェクトごとに開発ツールのバージョンを管理する統合ツールです。asdf、nvm、rbenv、pyenvなどの代替として機能し、より高速で使いやすい設計となっています。

## インストール

### macOS

```bash
# Homebrew
brew install mise

# または、インストールスクリプト
curl https://mise.run | sh
```

### Linux

```bash
# インストールスクリプト
curl https://mise.run | sh

# または、各パッケージマネージャー
# Ubuntu/Debian
apt update && apt install -y gpg sudo wget curl
sudo install -dm 755 /etc/apt/keyrings
wget -qO - https://mise.jdx.dev/gpg-key.pub | gpg --dearmor | sudo tee /etc/apt/keyrings/mise-archive-keyring.gpg 1> /dev/null
echo "deb [signed-by=/etc/apt/keyrings/mise-archive-keyring.gpg arch=amd64] https://mise.jdx.dev/deb stable main" | sudo tee /etc/apt/sources.list.d/mise.list
sudo apt update
sudo apt install -y mise
```

### シェル統合

```bash
# bash
echo 'eval "$(mise activate bash)"' >> ~/.bashrc

# zsh
echo 'eval "$(mise activate zsh)"' >> ~/.zshrc

# fish
echo 'mise activate fish | source' >> ~/.config/fish/config.fish
```

## 設定ファイル形式

### mise.toml

プロジェクトルートに配置する主設定ファイル。

```toml
# ツールバージョン指定
[tools]
rust = "1.90.0"          # 特定バージョン
node = "20"              # メジャーバージョン指定（20.x.x）
python = "latest"        # 最新安定版
go = "prefix:1.22"       # プレフィックスマッチング

# 環境変数
[env]
DATABASE_URL = "sqlite:./dev.db"
API_PORT = "8080"

# PATH追加
[env]
_.path = ["./bin", "./scripts"]

# タスク定義
[tasks.dev]
description = "開発サーバー起動"
run = "cargo watch -x run"
env = { RUST_LOG = "debug" }

# 条件付き設定
[env]
_.rust = { version = "1.90", if = "[ -f Cargo.toml ]" }
```

### .mise.local.toml

個人設定用（Gitで無視すべきファイル）。

```toml
[env]
ANTHROPIC_API_KEY = "sk-ant-..."
OPENAI_API_KEY = "sk-..."
DATABASE_URL = "postgresql://user:pass@localhost/mydb"
```

### グローバル設定

`~/.config/mise/config.toml`

```toml
[settings]
experimental = true
legacy_version_file = true
always_keep_download = false
plugin_autoupdate_last_check_duration = "1 week"

[tools]
node = "lts"
python = "3.12"
```

## ツール管理コマンド

### インストール

```bash
# プロジェクトの全ツールをインストール
mise install

# 特定ツールのインストール
mise install rust@1.90
mise install node@latest
mise install python@3.12

# インストール可能なバージョン一覧
mise list-remote rust
mise list-remote node --all  # 全バージョン表示
```

### 使用バージョンの設定

```bash
# プロジェクトで使用するバージョンを設定
mise use rust@1.90
mise use node@20 python@3.12

# グローバルデフォルトを設定
mise use -g rust@stable

# 一時的に別バージョンを使用
mise exec rust@nightly -- cargo build
```

### バージョン確認

```bash
# 現在のバージョンを表示
mise current

# インストール済みバージョン一覧
mise list

# 特定ツールの情報
mise where rust
mise which cargo
```

## 環境変数管理

### 設定方法

```toml
[env]
# 単純な値
PORT = "3000"
NODE_ENV = "development"

# テンプレート使用
DATABASE_URL = "postgres://localhost:5432/{{env.USER}}_dev"

# 既存の環境変数を拡張
PATH = ["./node_modules/.bin", "{{env.PATH}}"]

# 条件付き設定
_.production = { NODE_ENV = "production", if = '[ "$DEPLOY_ENV" = "prod" ]' }
```

### 環境変数の確認

```bash
# 設定された環境変数を表示
mise env

# 特定の環境変数を確認
mise env | grep DATABASE_URL

# シェルにエクスポート
eval "$(mise env)"
```

## タスク管理

### タスク定義

```toml
[tasks.test]
description = "Run all tests"
run = "cargo test --all-features"

[tasks.lint]
description = "Run linters"
run = [
  "cargo clippy -- -D warnings",
  "cargo fmt -- --check"
]

[tasks.build]
description = "Build release binary"
run = "cargo build --release"
depends = ["lint", "test"]

[tasks.serve]
description = "Start development server"
run = "cargo run -- --dev"
env = { RUST_LOG = "debug" }
dir = "backend"
hide = false
raw = false
sources = ["src/**/*.rs", "Cargo.toml"]
```

### タスク実行

```bash
# タスク一覧
mise tasks

# タスク実行
mise run test
mise run build

# 省略形
mise test
mise build

# 引数付き実行
mise run test -- --nocapture
```

## プラグイン管理

```bash
# プラグイン一覧
mise plugins

# プラグイン追加
mise plugins add kubectl https://github.com/asdf-community/asdf-kubectl.git

# プラグイン更新
mise plugins update kubectl

# プラグイン削除
mise plugins remove kubectl
```

## 高度な機能

### エイリアス

```toml
[alias.rust]
stable = "1.90"
lts = "1.90"
latest = "1.91"
```

### シェル補完

```bash
# bash
mise completion bash > /etc/bash_completion.d/mise

# zsh
mise completion zsh > /usr/local/share/zsh/site-functions/_mise

# fish
mise completion fish > ~/.config/fish/completions/mise.fish
```

### CI/CD統合

```yaml
# GitHub Actions
- uses: jdx/mise-action@v2
  with:
    version: latest
    install: true
    cache: true

- run: mise run test
```

## トラブルシューティング

### 一般的な問題

```bash
# 診断情報を表示
mise doctor

# キャッシュクリア
mise cache clear

# 設定の検証
mise settings

# デバッグモード
MISE_DEBUG=1 mise install
```

### よくあるエラー

#### ツールが見つからない

```bash
# パスを確認
mise which rust

# 再インストール
mise uninstall rust@1.90
mise install rust@1.90
```

#### バージョンが切り替わらない

```bash
# 設定ファイルの確認
mise config

# シェル統合の再実行
eval "$(mise activate bash)"
```

## ベストプラクティス

1. **mise.tomlをバージョン管理する**
   ```bash
   git add mise.toml
   ```

2. **個人設定はmise.local.tomlに**
   ```bash
   echo "mise.local.toml" >> .gitignore
   ```

3. **CIでの使用**
   ```toml
   # CI用の最小設定
   [tools]
   rust = "1.90"
   
   [tasks.ci]
   run = ["lint", "test", "build"]
   ```

4. **開発環境の標準化**
   ```bash
   # 初回セットアップスクリプト
   #!/bin/bash
   mise install
   mise run setup
   ```

## 移行ガイド

### asdfからの移行

```bash
# .tool-versionsがある場合
mise install  # 自動的に読み込まれる

# または明示的に変換
mise use $(cat .tool-versions)
```

### nvmからの移行

```bash
# .nvmrcがある場合
mise use node@$(cat .nvmrc)
```

## パフォーマンスTips

1. **並列インストール**
   ```bash
   mise install -j 4  # 4並列でインストール
   ```

2. **キャッシュの活用**
   ```toml
   [settings]
   always_keep_download = true
   ```

3. **不要なプラグインの削除**
   ```bash
   mise prune  # 未使用のツールを削除
   ```
