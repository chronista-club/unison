# Rust SDK 実践ガイド

## セットアップ

### Cargo.toml
```toml
[dependencies]
surrealdb = "2"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

## 接続

### リモート接続
```rust
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;

#[tokio::main]
async fn main() -> surrealdb::Result<()> {
    // 接続
    let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;
    
    // 認証
    db.signin(Root {
        username: "root",
        password: "root",
    }).await?;
    
    // USE NS DB
    db.use_ns("myapp").use_db("production").await?;
    
    Ok(())
}
```

### 埋め込みモード（RocksDB）
```rust
use surrealdb::Surreal;
use surrealdb::engine::local::RocksDb;

#[tokio::main]
async fn main() -> surrealdb::Result<()> {
    let db = Surreal::new::<RocksDb>("path/to/db").await?;
    db.use_ns("myapp").use_db("production").await?;
    Ok(())
}
```

### メモリモード
```rust
use surrealdb::Surreal;
use surrealdb::engine::local::Mem;

let db = Surreal::new::<Mem>(()).await?;
```

## CRUD操作

### 型定義
```rust
use serde::{Deserialize, Serialize};
use surrealdb::RecordId; // edition="2024"ではRecordIdを使用

#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<RecordId>,
    name: String,
    email: String,
    age: u32,
}
```

### CREATE
```rust
// 自動ID
let user: Option<User> = db.create("user")
    .content(User {
        id: None,
        name: "John".to_string(),
        email: "john@example.com".to_string(),
        age: 30,
    })
    .await?;

// ID指定
let user: Option<User> = db.create(("user", "john"))
    .content(User {
        id: None,
        name: "John".to_string(),
        email: "john@example.com".to_string(),
        age: 30,
    })
    .await?;
```

### SELECT
```rust
// 全件取得
let users: Vec<User> = db.select("user").await?;

// ID指定
let user: Option<User> = db.select(("user", "john")).await?;
```

### UPDATE
```rust
// 全体更新
let user: Option<User> = db.update(("user", "john"))
    .content(User {
        id: None,
        name: "John Smith".to_string(),
        email: "john@example.com".to_string(),
        age: 31,
    })
    .await?;

// MERGE（部分更新）
let user: Option<User> = db.update(("user", "john"))
    .merge(serde_json::json!({ "age": 31 }))
    .await?;
```

### DELETE
```rust
let user: Option<User> = db.delete(("user", "john")).await?;
```

## クエリ実行

### 基本クエリ
```rust
let mut result = db.query("SELECT * FROM user WHERE age > $age")
    .bind(("age", 25))
    .await?;

let users: Vec<User> = result.take(0)?;
```

### 複数ステートメント
```rust
let mut result = db.query("
    CREATE user:alice SET name = 'Alice', age = 28;
    CREATE user:bob SET name = 'Bob', age = 32;
    SELECT * FROM user;
")
.await?;

let alice: Option<User> = result.take(0)?;
let bob: Option<User> = result.take(1)?;
let users: Vec<User> = result.take(2)?;
```

## トランザクション

```rust
// BEGIN
db.query("BEGIN TRANSACTION").await?;

// 操作
db.create("user")
    .content(User { /* ... */ })
    .await?;

// COMMIT
db.query("COMMIT TRANSACTION").await?;

// またはCANCEL
// db.query("CANCEL TRANSACTION").await?;
```

## エラーハンドリング

```rust
use surrealdb::Error;

match db.select::<Option<User>>(("user", "john")).await {
    Ok(Some(user)) => println!("Found: {:?}", user),
    Ok(None) => println!("Not found"),
    Err(Error::Api(e)) => eprintln!("API error: {}", e),
    Err(Error::Db(e)) => eprintln!("DB error: {}", e),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Arc<RwLock>パターン（並行アクセス）

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    db: Arc<RwLock<Surreal<Ws>>>,
}

impl AppState {
    async fn new() -> surrealdb::Result<Self> {
        let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;
        db.signin(Root {
            username: "root",
            password: "root",
        }).await?;
        db.use_ns("myapp").use_db("production").await?;
        
        Ok(Self {
            db: Arc::new(RwLock::new(db)),
        })
    }
    
    async fn get_user(&self, id: &str) -> surrealdb::Result<Option<User>> {
        let db = self.db.read().await;
        db.select(("user", id)).await
    }
}
```

## 実用パターン

### 接続プール（シングルトン）
```rust
use once_cell::sync::OnceCell;

static DB: OnceCell<Surreal<Ws>> = OnceCell::new();

async fn init_db() -> surrealdb::Result<()> {
    let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;
    db.signin(Root {
        username: "root",
        password: "root",
    }).await?;
    db.use_ns("myapp").use_db("production").await?;
    
    DB.set(db).expect("DB already initialized");
    Ok(())
}

fn get_db() -> &'static Surreal<Ws> {
    DB.get().expect("DB not initialized")
}
```

## 公式ドキュメント

- Rust SDK: https://surrealdb.com/docs/sdk/rust
- API Docs: https://docs.rs/surrealdb
- Examples: https://github.com/surrealdb/surrealdb/tree/main/examples
