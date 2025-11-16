# SurrealDB クイックスタート

## インストール

### macOS (Homebrew)
```bash
brew install surrealdb/tap/surreal
```

### Cargo
```bash
cargo install surrealdb --locked
```

### Docker
```bash
docker pull surrealdb/surrealdb:latest
```

## サーバー起動

### メモリモード（開発用）
```bash
surreal start --user root --pass root memory
```

### ファイルモード（永続化）
```bash
surreal start --user root --pass root file://mydb.db
```

### RocksDB（本番推奨）
```bash
surreal start --user root --pass root --bind 0.0.0.0:8000 rocksdb://data/mydb
```

## 基本操作

### CLIで接続
```bash
surreal sql --endpoint http://localhost:8000 --user root --pass root
```

### ネームスペースとデータベース
```sql
USE NS myapp DB production;
```

### テーブル作成
```sql
DEFINE TABLE user SCHEMAFULL;
DEFINE FIELD name ON TABLE user TYPE string;
DEFINE FIELD email ON TABLE user TYPE string;
DEFINE INDEX unique_email ON TABLE user FIELDS email UNIQUE;
```

### CRUD操作
```sql
-- Create
CREATE user:john SET name = 'John Doe', email = 'john@example.com';

-- Read
SELECT * FROM user WHERE email = 'john@example.com';

-- Update
UPDATE user:john SET name = 'John Smith';

-- Delete
DELETE user:john;
```

### リレーション
```sql
-- テーブル定義
DEFINE TABLE post SCHEMAFULL;
DEFINE FIELD title ON TABLE post TYPE string;
DEFINE FIELD author ON TABLE post TYPE record<user>;

-- リレーション作成
CREATE post:1 SET title = 'Hello', author = user:john;

-- フェッチ
SELECT *, author.* FROM post;
```

## 次のステップ

- 詳細な構文: [SurrealQL Essentials](./surrealql_essentials.md)
- Rust SDK: [Rust SDK Guide](./rust_sdk.md)
- TypeScript SDK: [TypeScript SDK Guide](./typescript_sdk.md)
- 公式ドキュメント: https://surrealdb.com/docs
