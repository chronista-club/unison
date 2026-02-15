# SurrealDB CLIコマンドリファレンス

## インストール

```bash
# macOS/Linux
curl -sSf https://install.surrealdb.com | sh

# Homebrew
brew install surrealdb/tap/surreal

# Cargo
cargo install surrealdb --locked

# バージョン確認
surreal version
```

## サーバー起動

### 基本起動

```bash
# メモリモード（テスト用）
surreal start memory

# RocksDB（本番推奨）
surreal start --user root --pass root rocksdb://data.db

# 特定アドレスでバインド
surreal start --bind 0.0.0.0:8000 rocksdb://data.db

# 認証情報付き
surreal start \
  --user vtg-local \
  --pass vtg-local \
  --bind 0.0.0.0:30300 \
  rocksdb://.vantage/database.db
```

### オプション

| オプション | 説明 | デフォルト |
|-----------|------|-----------|
| `--bind` | バインドアドレス | 127.0.0.1:8000 |
| `--user` | ルートユーザー名 | root |
| `--pass` | ルートパスワード | root |
| `--log` | ログレベル（trace/debug/info/warn/error） | info |
| `--auth` | 認証を有効化 | true |

## SQL実行

### インタラクティブモード

```bash
# デフォルト接続
surreal sql --endpoint http://localhost:8000 --namespace test --database test

# 認証付き
surreal sql \
  --endpoint http://localhost:8000 \
  --namespace myapp \
  --database production \
  --username root \
  --password root

# 省略形
surreal sql -e http://localhost:8000 -n myapp -d prod -u root -p root
```

### スクリプト実行

```bash
# ファイルから実行
surreal sql --endpoint http://localhost:8000 \
  --namespace myapp --database prod \
  --username root --password root \
  schema/01_tables/user.surql

# パイプで実行
cat schema.surql | surreal sql -e http://localhost:8000 -n myapp -d prod -u root -p root
```

## インポート/エクスポート

### エクスポート

```bash
# データベース全体をエクスポート
surreal export \
  --endpoint http://localhost:8000 \
  --namespace myapp \
  --database prod \
  --username root \
  --password root \
  backup.surql

# 特定テーブルのみ
surreal export \
  -e http://localhost:8000 -n myapp -d prod -u root -p root \
  --table user \
  user_backup.surql
```

### インポート

```bash
# SurrealQLファイルをインポート
surreal import \
  --endpoint http://localhost:8000 \
  --namespace myapp \
  --database prod \
  --username root \
  --password root \
  backup.surql

# 複数ファイルを順次インポート
for file in schema/*.surql; do
  surreal import -e http://localhost:8000 -n myapp -d prod -u root -p root "$file"
done
```

## バックアップ/リストア

### バックアップスクリプト例

```bash
#!/bin/bash
# backup.sh

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="backups"
ENDPOINT="http://localhost:8000"
NAMESPACE="myapp"
DATABASE="prod"

mkdir -p "$BACKUP_DIR"

surreal export \
  --endpoint "$ENDPOINT" \
  --namespace "$NAMESPACE" \
  --database "$DATABASE" \
  --username root \
  --password root \
  "$BACKUP_DIR/backup_${TIMESTAMP}.surql"

echo "Backup created: $BACKUP_DIR/backup_${TIMESTAMP}.surql"

# 古いバックアップを削除（7日以上前）
find "$BACKUP_DIR" -name "backup_*.surql" -mtime +7 -delete
```

### リストアスクリプト例

```bash
#!/bin/bash
# restore.sh

if [ $# -eq 0 ]; then
  echo "Usage: $0 <backup_file>"
  exit 1
fi

BACKUP_FILE=$1
ENDPOINT="http://localhost:8000"
NAMESPACE="myapp"
DATABASE="prod"

read -p "本当にリストアしますか？既存データは上書きされます。(yes/no): " confirm

if [ "$confirm" != "yes" ]; then
  echo "リストアをキャンセルしました"
  exit 0
fi

surreal import \
  --endpoint "$ENDPOINT" \
  --namespace "$NAMESPACE" \
  --database "$DATABASE" \
  --username root \
  --password root \
  "$BACKUP_FILE"

echo "Restore completed from: $BACKUP_FILE"
```

## マイグレーション運用

### スキーマ適用スクリプト

```bash
#!/bin/bash
# apply_schema.sh

ENDPOINT="${SURREAL_ENDPOINT:-http://localhost:8000}"
NAMESPACE="${SURREAL_NAMESPACE:-myapp}"
DATABASE="${SURREAL_DATABASE:-prod}"
USERNAME="${SURREAL_USERNAME:-root}"
PASSWORD="${SURREAL_PASSWORD:-root}"
SCHEMA_DIR="schema"

categories=("01_tables" "02_indexes" "03_relations" "04_permissions")

for category in "${categories[@]}"; do
  echo "Applying $category..."
  for file in "$SCHEMA_DIR/$category"/*.surql; do
    if [ -f "$file" ]; then
      echo "  - $(basename "$file")"
      surreal sql \
        --endpoint "$ENDPOINT" \
        --namespace "$NAMESPACE" \
        --database "$DATABASE" \
        --username "$USERNAME" \
        --password "$PASSWORD" \
        "$file"
    fi
  done
done

echo "Schema applied successfully"
```

### CI/CD統合例

```yaml
# .github/workflows/deploy.yml
name: Deploy Schema

on:
  push:
    branches: [main]
    paths:
      - 'schema/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install SurrealDB CLI
        run: curl -sSf https://install.surrealdb.com | sh
      
      - name: Apply Schema
        env:
          SURREAL_ENDPOINT: ${{ secrets.SURREAL_ENDPOINT }}
          SURREAL_NAMESPACE: ${{ secrets.SURREAL_NAMESPACE }}
          SURREAL_DATABASE: ${{ secrets.SURREAL_DATABASE }}
          SURREAL_USERNAME: ${{ secrets.SURREAL_USERNAME }}
          SURREAL_PASSWORD: ${{ secrets.SURREAL_PASSWORD }}
        run: ./scripts/apply_schema.sh
```

## 便利なエイリアス

```bash
# ~/.bashrc or ~/.zshrc に追加

# ローカル開発用
alias surreal-dev='surreal start --user dev --pass dev --bind 127.0.0.1:8000 memory'

# 本番用SQL接続
alias surreal-prod='surreal sql -e https://prod.example.com -n myapp -d prod -u $SURREAL_USER -p $SURREAL_PASS'

# スキーマ適用
alias surreal-schema='./scripts/apply_schema.sh'

# バックアップ
alias surreal-backup='./scripts/backup.sh'
```

## トラブルシューティング

### ポート競合

```bash
# ポート使用状況を確認
lsof -i :8000

# プロセスを停止
kill -9 <PID>

# または別のポートで起動
surreal start --bind 127.0.0.1:9000 rocksdb://data.db
```

### 接続エラー

```bash
# サーバーが起動しているか確認
curl http://localhost:8000/health

# ログを詳細に出力
surreal start --log trace rocksdb://data.db
```

### データベース破損

```bash
# データベースをバックアップ
cp -r data.db data.db.backup

# 新しいデータベースで起動
surreal start --user root --pass root rocksdb://data_new.db

# バックアップからリストア
surreal import -e http://localhost:8000 -n myapp -d prod -u root -p root backup.surql
```

## ベストプラクティス

1. **環境変数を使用**: 認証情報をスクリプトにハードコードしない
2. **定期バックアップ**: cronで自動バックアップを設定
3. **スキーマバージョニング**: スキーマファイルをGitで管理
4. **テスト環境**: 本番適用前にステージング環境でテスト
5. **ログ監視**: `--log info`以上のレベルでログを記録
