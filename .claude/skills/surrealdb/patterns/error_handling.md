# エラーハンドリングパターン

## よくあるエラーと対処法

### 接続エラー

```rust
use surrealdb::Error;

async fn connect_with_retry() -> Result<Surreal<Any>, Error> {
    let max_retries = 3;
    let mut retry_count = 0;
    
    loop {
        match Surreal::new::<Ws>("127.0.0.1:8000").await {
            Ok(db) => return Ok(db),
            Err(e) if retry_count < max_retries => {
                eprintln!("Connection failed (attempt {}): {}", retry_count + 1, e);
                retry_count += 1;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

```typescript
// TypeScript
async function connectWithRetry(maxRetries = 3): Promise<Surreal> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const db = new Surreal();
      await db.connect('ws://localhost:8000');
      return db;
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      console.error(`Connection failed (attempt ${i + 1}):`, error);
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
  }
  throw new Error('Failed to connect after retries');
}
```

### 認証エラー

```rust
use surrealdb::opt::auth::Root;

async fn signin_with_error_handling(db: &Surreal<Any>) -> Result<(), Box<dyn std::error::Error>> {
    match db.signin(Root {
        username: "root",
        password: "root",
    }).await {
        Ok(_) => Ok(()),
        Err(e) => {
            match e {
                Error::Api(api_err) if api_err.to_string().contains("credentials") => {
                    Err("Invalid credentials".into())
                }
                Error::Api(api_err) if api_err.to_string().contains("connection") => {
                    Err("Server not responding".into())
                }
                _ => Err(e.into()),
            }
        }
    }
}
```

### レコード未存在エラー

```rust
use surrealdb::sql::Thing;

async fn get_user_safe(
    db: &Surreal<Any>,
    user_id: &str,
) -> Result<Option<User>, Error> {
    let thing = Thing::from(("user", user_id));
    
    match db.select::<Option<User>>(thing).await {
        Ok(user) => Ok(user),
        Err(Error::Api(api_err)) if api_err.to_string().contains("not found") => {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

// 使用例
match get_user_safe(&db, "john").await? {
    Some(user) => println!("User found: {:?}", user),
    None => println!("User not found"),
}
```

```typescript
// TypeScript
async function getUserSafe(db: Surreal, userId: string): Promise<User | null> {
  try {
    const user = await db.select<User>(new RecordId('user', userId));
    return user || null;
  } catch (error) {
    if (error.message?.includes('not found')) {
      return null;
    }
    throw error;
  }
}
```

### クエリエラー

```rust
async fn query_with_error_context(
    db: &Surreal<Any>,
    query: &str,
) -> Result<Vec<Article>, Box<dyn std::error::Error>> {
    db.query(query)
        .await
        .map_err(|e| format!("Query failed: {}\nQuery: {}", e, query))?
        .take(0)
        .map_err(|e| format!("Failed to parse results: {}", e).into())
}
```

### ASSERT制約違反

```sql
-- スキーマ定義
DEFINE FIELD email ON TABLE user TYPE string
  ASSERT string::is::email($value);

DEFINE FIELD age ON TABLE user TYPE int
  ASSERT $value >= 0 AND $value <= 150;
```

```rust
// Rust
async fn create_user_with_validation(
    db: &Surreal<Any>,
    name: String,
    email: String,
    age: i32,
) -> Result<User, String> {
    // クライアント側でも検証
    if !email.contains('@') {
        return Err("Invalid email format".to_string());
    }
    
    if age < 0 || age > 150 {
        return Err("Age must be between 0 and 150".to_string());
    }
    
    // データベース作成
    match db.create::<Option<User>>(("user", name.clone()))
        .content(User { name, email, age })
        .await
    {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err("Failed to create user".to_string()),
        Err(e) => {
            // ASSERT違反のエラーメッセージをパース
            let error_msg = e.to_string();
            if error_msg.contains("ASSERT") {
                Err("Validation failed: Please check your input".to_string())
            } else {
                Err(format!("Database error: {}", e))
            }
        }
    }
}
```

### ユニーク制約違反

```rust
async fn create_user_unique(
    db: &Surreal<Any>,
    email: String,
) -> Result<User, String> {
    match db.create::<Option<User>>("user")
        .content(User { email: email.clone(), ..Default::default() })
        .await
    {
        Ok(Some(user)) => Ok(user),
        Err(e) if e.to_string().contains("unique") => {
            Err(format!("Email {} already exists", email))
        }
        Err(e) => Err(format!("Database error: {}", e)),
        Ok(None) => Err("Unknown error".to_string()),
    }
}
```

```typescript
// TypeScript
async function createUserUnique(db: Surreal, email: string): Promise<User> {
  try {
    const user = await db.create<User>('user', {
      email,
      name: 'New User'
    });
    return user;
  } catch (error) {
    if (error.message?.includes('unique') || error.message?.includes('duplicate')) {
      throw new Error(`Email ${email} already exists`);
    }
    throw new Error(`Database error: ${error.message}`);
  }
}
```

## トランザクションエラーハンドリング

```rust
async fn transfer_with_rollback(
    db: &Surreal<Any>,
    from_id: Thing,
    to_id: Thing,
    amount: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    // トランザクション開始
    db.query("BEGIN TRANSACTION").await?;
    
    match perform_transfer(&db, from_id.clone(), to_id.clone(), amount).await {
        Ok(_) => {
            db.query("COMMIT TRANSACTION").await?;
            Ok(())
        }
        Err(e) => {
            // エラー時はロールバック
            db.query("CANCEL TRANSACTION").await?;
            Err(format!("Transfer failed: {}", e).into())
        }
    }
}

async fn perform_transfer(
    db: &Surreal<Any>,
    from_id: Thing,
    to_id: Thing,
    amount: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    // 残高チェック
    let from_account: Account = db.select(from_id.clone())
        .await?
        .ok_or("From account not found")?;
    
    if from_account.balance < amount {
        return Err("Insufficient balance".into());
    }
    
    // 引き落とし
    db.query("UPDATE $from SET balance -= $amount")
        .bind(("from", from_id))
        .bind(("amount", amount))
        .await?;
    
    // 入金
    db.query("UPDATE $to SET balance += $amount")
        .bind(("to", to_id))
        .bind(("amount", amount))
        .await?;
    
    Ok(())
}
```

```typescript
// TypeScript
async function transferWithRollback(
  db: Surreal,
  fromId: RecordId,
  toId: RecordId,
  amount: number
): Promise<void> {
  try {
    await db.query('BEGIN TRANSACTION');
    
    // 残高チェック
    const fromAccount = await db.select<Account>(fromId);
    if (!fromAccount || fromAccount.balance < amount) {
      throw new Error('Insufficient balance');
    }
    
    // 引き落とし
    await db.query('UPDATE $from SET balance -= $amount', {
      from: fromId,
      amount
    });
    
    // 入金
    await db.query('UPDATE $to SET balance += $amount', {
      to: toId,
      amount
    });
    
    await db.query('COMMIT TRANSACTION');
  } catch (error) {
    await db.query('CANCEL TRANSACTION');
    throw new Error(`Transfer failed: ${error.message}`);
  }
}
```

## パーミッションエラー

```sql
-- スキーマ定義
DEFINE FIELD email ON TABLE user TYPE string
  PERMISSIONS
    FOR select WHERE $auth.id = $parent.id OR $auth.role = "admin"
    FOR update WHERE $auth.id = $parent.id
  OVERWRITE;
```

```rust
async fn update_user_email(
    db: &Surreal<Any>,
    user_id: Thing,
    new_email: String,
) -> Result<(), String> {
    match db.query("UPDATE $user SET email = $email")
        .bind(("user", user_id.clone()))
        .bind(("email", new_email))
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("permission") => {
            Err("You don't have permission to update this user's email".to_string())
        }
        Err(e) => Err(format!("Update failed: {}", e)),
    }
}
```

## リトライパターン

```rust
use tokio::time::{sleep, Duration};

async fn query_with_retry<T: DeserializeOwned>(
    db: &Surreal<Any>,
    query: &str,
    max_retries: u32,
) -> Result<T, Error> {
    let mut retry_count = 0;
    
    loop {
        match db.query(query).await?.take::<T>(0) {
            Ok(result) => return Ok(result),
            Err(e) if retry_count < max_retries && is_retryable(&e) => {
                retry_count += 1;
                let delay = Duration::from_millis(100 * 2_u64.pow(retry_count));
                eprintln!("Query failed (attempt {}), retrying in {:?}", retry_count, delay);
                sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

fn is_retryable(error: &Error) -> bool {
    let error_str = error.to_string();
    error_str.contains("timeout") || error_str.contains("connection")
}
```

## エラーログとデバッグ

```rust
use tracing::{error, warn, debug};

async fn debug_query(db: &Surreal<Any>, query: &str) -> Result<(), Error> {
    debug!("Executing query: {}", query);
    
    match db.query(query).await {
        Ok(response) => {
            debug!("Query successful, response: {:?}", response);
            Ok(())
        }
        Err(e) => {
            error!("Query failed: {}", e);
            error!("Query was: {}", query);
            Err(e)
        }
    }
}
```

```typescript
// TypeScript
async function debugQuery(db: Surreal, query: string): Promise<any> {
  console.debug('Executing query:', query);
  
  try {
    const response = await db.query(query);
    console.debug('Query successful, response:', response);
    return response;
  } catch (error) {
    console.error('Query failed:', error);
    console.error('Query was:', query);
    throw error;
  }
}
```

## カスタムエラー型

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] surrealdb::Error),
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Duplicate entry: {0}")]
    Duplicate(String),
}

async fn get_user_typed(db: &Surreal<Any>, user_id: &str) -> Result<User, AppError> {
    let thing = Thing::from(("user", user_id));
    
    match db.select::<Option<User>>(thing).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err(AppError::UserNotFound(user_id.to_string())),
        Err(e) => Err(AppError::Database(e)),
    }
}
```

## パニック回避

```rust
// 悪い例：パニックする可能性
let user = db.select::<User>(("user", "john")).await.unwrap();

// 良い例：Resultで処理
let user = match db.select::<Option<User>>(("user", "john")).await {
    Ok(Some(user)) => user,
    Ok(None) => return Err("User not found".into()),
    Err(e) => return Err(format!("Database error: {}", e).into()),
};

// より良い例：? 演算子
let user = db.select::<Option<User>>(("user", "john"))
    .await?
    .ok_or("User not found")?;
```

## ベストプラクティス

1. **具体的なエラーメッセージ**: ユーザーが理解できるエラーメッセージを返す
2. **エラーログ**: 詳細なエラー情報をログに記録
3. **リトライ戦略**: 一時的なエラーに対してはリトライを実装
4. **トランザクション**: 複数操作は必ずトランザクションで囲む
5. **クライアント側検証**: データベース検証の前にクライアント側でも検証
6. **カスタムエラー型**: アプリケーション固有のエラー型を定義
7. **パニック回避**: `unwrap()`や`expect()`は避け、Resultで処理
8. **エラーコンテキスト**: エラー発生時のコンテキスト情報を含める
9. **ユーザーフレンドリー**: 技術的なエラーをユーザー向けに変換
10. **型安全**: TypeScriptやRustの型システムを活用してエラーを減らす
