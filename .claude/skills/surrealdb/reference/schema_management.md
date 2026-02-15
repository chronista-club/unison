# スキーマ管理 - OVERWRITE戦略

SurrealDBのスキーマ管理には`OVERWRITE`キーワードを使った冪等な宣言的アプローチを推奨します。

## 基本概念

### OVERWRITE の利点

1. **冪等性**: 何度実行しても同じ結果
2. **順序不要**: マイグレーションバージョン管理不要
3. **宣言的**: 現在のあるべき状態を定義
4. **シンプル**: 複雑なマイグレーション管理が不要

### 基本構文

```sql
-- テーブル定義
DEFINE TABLE user SCHEMAFULL OVERWRITE;

-- フィールド定義
DEFINE FIELD name ON TABLE user TYPE string OVERWRITE;

-- インデックス定義
DEFINE INDEX idx_email ON TABLE user FIELDS email UNIQUE OVERWRITE;
```

## ディレクトリ構造

### 推奨構造

```
schema/
├── 01_tables/
│   ├── user.surql
│   ├── post.surql
│   └── comment.surql
├── 02_indexes/
│   ├── user_indexes.surql
│   └── post_indexes.surql
├── 03_relations/
│   └── likes.surql
├── 04_permissions/
│   └── user_permissions.surql
└── removed/
    └── old_definitions.surql  # 削除が必要な定義
```

### 番号プレフィックスの意味

- `01_`: 基本テーブル定義（依存なし）
- `02_`: インデックス（テーブル依存）
- `03_`: リレーションテーブル（複数テーブル依存）
- `04_`: 権限設定（全体依存）

## スキーマファイルの作成

### tables/user.surql

```sql
-- ユーザーテーブル
DEFINE TABLE user SCHEMAFULL OVERWRITE
    PERMISSIONS
        FOR select WHERE published = true OR $auth.id = id
        FOR create, update, delete WHERE $auth.id = id;

-- フィールド定義
DEFINE FIELD name ON TABLE user 
    TYPE string 
    ASSERT string::len($value) > 0
    OVERWRITE;

DEFINE FIELD email ON TABLE user 
    TYPE string 
    ASSERT string::is::email($value)
    OVERWRITE;

DEFINE FIELD password_hash ON TABLE user 
    TYPE string 
    PERMISSIONS FOR select NONE
    OVERWRITE;

DEFINE FIELD age ON TABLE user 
    TYPE option<int>
    OVERWRITE;

DEFINE FIELD created_at ON TABLE user 
    TYPE datetime 
    DEFAULT time::now()
    OVERWRITE;

DEFINE FIELD updated_at ON TABLE user 
    TYPE datetime 
    DEFAULT time::now()
    VALUE time::now()
    OVERWRITE;
```

### tables/post.surql

```sql
DEFINE TABLE post SCHEMAFULL OVERWRITE;

DEFINE FIELD title ON TABLE post 
    TYPE string 
    ASSERT string::len($value) > 0
    OVERWRITE;

DEFINE FIELD content ON TABLE post 
    TYPE string 
    OVERWRITE;

DEFINE FIELD author ON TABLE post 
    TYPE record<user>
    OVERWRITE;

DEFINE FIELD published ON TABLE post 
    TYPE bool 
    DEFAULT false
    OVERWRITE;

DEFINE FIELD created_at ON TABLE post 
    TYPE datetime 
    DEFAULT time::now()
    OVERWRITE;
```

### indexes/user_indexes.surql

```sql
-- ユニークインデックス
DEFINE INDEX unique_email ON TABLE user 
    FIELDS email 
    UNIQUE 
    OVERWRITE;

-- 全文検索インデックス
DEFINE INDEX idx_name ON TABLE user 
    FIELDS name 
    SEARCH ANALYZER ascii 
    OVERWRITE;
```

### relations/likes.surql

```sql
-- 多対多リレーション
DEFINE TABLE likes SCHEMAFULL OVERWRITE;

DEFINE FIELD in ON TABLE likes 
    TYPE record<user>
    OVERWRITE;

DEFINE FIELD out ON TABLE likes 
    TYPE record<post>
    OVERWRITE;

DEFINE FIELD created_at ON TABLE likes 
    TYPE datetime 
    DEFAULT time::now()
    OVERWRITE;

-- リレーション用インデックス
DEFINE INDEX idx_likes_in ON TABLE likes 
    FIELDS in 
    OVERWRITE;

DEFINE INDEX idx_likes_out ON TABLE likes 
    FIELDS out 
    OVERWRITE;
```

## Rust実装

### SchemaManager

```rust
use std::fs;
use std::path::PathBuf;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

pub struct SchemaManager {
    schema_dir: PathBuf,
}

impl SchemaManager {
    pub fn new(schema_dir: impl Into<PathBuf>) -> Self {
        Self {
            schema_dir: schema_dir.into(),
        }
    }
    
    /// 全スキーマを適用
    pub async fn apply_all(&self, db: &Surreal<Any>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Applying schema from: {}", self.schema_dir.display());
        
        // カテゴリ順に適用
        let categories = ["01_tables", "02_indexes", "03_relations", "04_permissions"];
        
        for category in categories {
            self.apply_category(db, category).await?;
        }
        
        println!("Schema applied successfully!");
        Ok(())
    }
    
    /// カテゴリ配下の全.surqlファイルを適用
    async fn apply_category(
        &self,
        db: &Surreal<Any>,
        category: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let category_path = self.schema_dir.join(category);
        
        if !category_path.exists() {
            return Ok(());
        }
        
        println!("\nApplying category: {}", category);
        
        let mut entries: Vec<_> = fs::read_dir(&category_path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    == Some("surql")
            })
            .collect();
        
        // ファイル名でソート
        entries.sort_by_key(|e| e.file_name());
        
        for entry in entries {
            let path = entry.path();
            let filename = path.file_name().unwrap().to_string_lossy();
            
            println!("  ✓ {}", filename);
            
            let sql = fs::read_to_string(&path)?;
            db.query(sql).await?;
        }
        
        Ok(())
    }
}

// 使用例
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Surreal::new::<Ws>("localhost:8000").await?;
    
    db.signin(Root {
        username: "root",
        password: "root",
    }).await?;
    
    db.use_ns("myapp").use_db("production").await?;
    
    // スキーマ適用
    let schema_manager = SchemaManager::new("schema");
    schema_manager.apply_all(&db).await?;
    
    Ok(())
}
```

### 起動時自動適用

```rust
// main.rs
async fn initialize_database() -> Result<Surreal<Any>> {
    let db = Surreal::new::<Ws>("localhost:8000").await?;
    db.signin(Root { username: "root", password: "root" }).await?;
    db.use_ns("myapp").use_db("production").await?;
    
    // スキーマ適用（冪等なので毎回実行してOK）
    let schema_manager = SchemaManager::new("schema");
    schema_manager.apply_all(&db).await?;
    
    Ok(db)
}
```

## TypeScript実装

### SchemaManager

```typescript
import { Surreal } from 'surrealdb';
import { readdir, readFile } from 'fs/promises';
import { join } from 'path';

export class SchemaManager {
  constructor(
    private db: Surreal,
    private schemaDir: string = 'schema'
  ) {}
  
  async applyAll(): Promise<void> {
    console.log(`Applying schema from: ${this.schemaDir}`);
    
    const categories = [
      '01_tables',
      '02_indexes',
      '03_relations',
      '04_permissions',
    ];
    
    for (const category of categories) {
      await this.applyCategory(category);
    }
    
    console.log('Schema applied successfully!');
  }
  
  private async applyCategory(category: string): Promise<void> {
    const categoryPath = join(this.schemaDir, category);
    
    try {
      const files = await readdir(categoryPath);
      const surqlFiles = files
        .filter(f => f.endsWith('.surql'))
        .sort();
      
      if (surqlFiles.length === 0) return;
      
      console.log(`\nApplying category: ${category}`);
      
      for (const file of surqlFiles) {
        console.log(`  ✓ ${file}`);
        const sql = await readFile(join(categoryPath, file), 'utf-8');
        await this.db.query(sql);
      }
    } catch (error: any) {
      if (error.code !== 'ENOENT') throw error;
    }
  }
}

// 使用例
async function initializeDatabase(): Promise<Surreal> {
  const db = new Surreal();
  await db.connect('ws://localhost:8000/rpc');
  await db.signin({ username: 'root', password: 'root' });
  await db.use({ ns: 'myapp', db: 'production' });
  
  // スキーマ適用
  const schemaManager = new SchemaManager(db);
  await schemaManager.applyAll();
  
  return db;
}
```

## CLIスクリプト

### apply.sh

```bash
#!/bin/bash
set -e

# 設定
ENDPOINT=${SURREAL_ENDPOINT:-http://localhost:8000}
USER=${SURREAL_USER:-root}
PASS=${SURREAL_PASS:-root}
NS=${SURREAL_NS:-myapp}
DB=${SURREAL_DB:-production}

echo "================================================"
echo "Applying schema to $ENDPOINT/$NS/$DB"
echo "================================================"

# カテゴリ順に適用
for category_dir in schema/*/; do
  category=$(basename "$category_dir")
  
  echo ""
  echo "Applying category: $category"
  echo "--------------------------------"
  
  for file in "$category_dir"*.surql; do
    if [ -f "$file" ]; then
      filename=$(basename "$file")
      echo "  ✓ $filename"
      
      surreal sql \
        --endpoint "$ENDPOINT" \
        --user "$USER" \
        --pass "$PASS" \
        --ns "$NS" \
        --db "$DB" \
        < "$file"
    fi
  done
done

echo ""
echo "================================================"
echo "Schema applied successfully!"
echo "================================================"
```

### 実行

```bash
chmod +x schema/apply.sh
./schema/apply.sh
```

## 削除戦略

### removed/配下で管理

削除が必要な定義は`removed/`に移動し、手動で実行します。

```sql
-- schema/removed/2024-01-15_remove_old_field.surql
REMOVE FIELD old_field ON TABLE user;
REMOVE INDEX old_index ON TABLE user;
REMOVE TABLE deprecated_table;
```

### 削除スクリプト

```bash
#!/bin/bash
# schema/remove.sh

echo "WARNING: This will remove schema definitions!"
read -p "Continue? (y/N): " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
  for file in schema/removed/*.surql; do
    echo "Removing: $(basename $file)"
    surreal sql --endpoint "$ENDPOINT" \
      --user "$USER" --pass "$PASS" \
      --ns "$NS" --db "$DB" < "$file"
  done
fi
```

## CI/CD統合

### GitHub Actions

```yaml
name: Apply Schema

on:
  push:
    branches: [main]
    paths:
      - 'schema/**'

jobs:
  apply-schema:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install SurrealDB
        run: |
          curl -sSf https://install.surrealdb.com | sh
      
      - name: Apply Schema
        env:
          SURREAL_ENDPOINT: ${{ secrets.SURREAL_ENDPOINT }}
          SURREAL_USER: ${{ secrets.SURREAL_USER }}
          SURREAL_PASS: ${{ secrets.SURREAL_PASS }}
          SURREAL_NS: production
          SURREAL_DB: app
        run: |
          chmod +x schema/apply.sh
          ./schema/apply.sh
```

## ベストプラクティス

### 1. 常にOVERWRITEを使用

```sql
-- ✅ 冪等
DEFINE TABLE user SCHEMAFULL OVERWRITE;
DEFINE FIELD name ON TABLE user TYPE string OVERWRITE;

-- ❌ 2回目の実行でエラー
DEFINE TABLE user SCHEMAFULL;
DEFINE FIELD name ON TABLE user TYPE string;
```

### 2. ファイルは論理単位で分割

```
01_tables/
├── user.surql          # ユーザー関連
├── post.surql          # 投稿関連
└── comment.surql       # コメント関連
```

### 3. カテゴリ順序を守る

```
01_tables/      # 最初（依存なし）
02_indexes/     # テーブル後
03_relations/   # リレーションは最後
04_permissions/ # 全体設定は最後
```

### 4. 環境別スキーマ

```
schema/
├── base/           # 全環境共通
├── development/    # 開発専用
└── production/     # 本番専用
```

```bash
# 開発環境
./schema/apply.sh schema/base
./schema/apply.sh schema/development

# 本番環境
./schema/apply.sh schema/base
./schema/apply.sh schema/production
```

## トラブルシューティング

### 問題1: フィールド型変更

```sql
-- ❌ 型変更はエラーになる場合がある
DEFINE FIELD age ON TABLE user TYPE int OVERWRITE;
-- 既存データがstring型の場合エラー

-- ✅ データ移行を先に実行
UPDATE user SET age = <int>age WHERE age != NONE;
DEFINE FIELD age ON TABLE user TYPE int OVERWRITE;
```

### 問題2: インデックス再構築

```sql
-- OVERWRITE時に自動再構築される
DEFINE INDEX idx_email ON TABLE user FIELDS email UNIQUE OVERWRITE;
```

### 問題3: 依存関係エラー

```sql
-- ❌ postテーブルがまだ存在しない
DEFINE FIELD post ON TABLE comment TYPE record<post> OVERWRITE;

-- ✅ カテゴリ順序で解決
-- 01_tables/post.surql で post を先に定義
-- 01_tables/comment.surql で comment を定義
```

## 公式リソース

- DEFINE文: https://surrealdb.com/docs/surrealql/statements/define
- スキーマ: https://surrealdb.com/docs/surrealql/datamodel
