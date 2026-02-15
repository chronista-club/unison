# SurrealDB 型システム完全ガイド

SurrealDBの型システムは独特です。特にRecordId（Thing）、Reference、レコードリンクの理解が重要です。

## レコードID（Thing / RecordId）

### 基本形式
```
table:id
```

### SurrealQLでの使用
```sql
-- 文字列ID
CREATE user:john SET name = 'John';

-- 数値ID
CREATE user:123 SET name = 'Jane';

-- UUID（自動生成）
CREATE user SET name = 'Alice';
-- 結果例: user:⟨ulid()⟩

-- ULID指定
CREATE user:⟨01HQKM9YN7⟩ SET name = 'Bob';
```

### Rust: RecordId型（edition="2024"）
```rust
use surrealdb::RecordId;

// RecordId作成
let user_id = RecordId::from(("user", "john"));

// 文字列からパース
let record_id: RecordId = "user:john".parse()?;

// 構造体で使用
#[derive(Serialize, Deserialize)]
struct User {
    id: Option<RecordId>,
    name: String,
}

// CRUD
let user: Option<User> = db.select(("user", "john")).await?;
let user: Option<User> = db.select(user_id).await?;
```

### TypeScript: RecordId型
```typescript
import { RecordId } from 'surrealdb';

// RecordId作成
const userId = new RecordId('user', 'john');

// 文字列でも可
const user = await db.select<User>('user:john');

// 型定義
interface User {
  id?: RecordId<'user'> | string;
  name: string;
}
```

## Record型（レコードリンク）

### SurrealQLでの定義
```sql
-- Record型フィールド
DEFINE FIELD author ON TABLE post TYPE record<user>;
DEFINE FIELD category ON TABLE post TYPE record<category>;

-- 単一レコードへの参照
CREATE post:1 SET 
    title = 'Hello',
    author = user:john;

-- フェッチ
SELECT *, author.* FROM post;
SELECT *, author.(id, name, email) FROM post;
```

### 複数レコード（配列）
```sql
-- 配列型
DEFINE FIELD tags ON TABLE post TYPE array<record<tag>>;

CREATE post:1 SET
    title = 'Hello',
    tags = [tag:tech, tag:news];

SELECT *, tags.* FROM post;
```

### Rust実装
```rust
use surrealdb::RecordId;

#[derive(Serialize, Deserialize)]
struct Post {
    id: Option<RecordId>,
    title: String,
    author: RecordId,  // レコードリンク
}

// フェッチ後の型
#[derive(Serialize, Deserialize)]
struct PostWithAuthor {
    id: Option<RecordId>,
    title: String,
    author: User,  // 完全なUserオブジェクト
}

let posts: Vec<PostWithAuthor> = db
    .query("SELECT *, author.* FROM post")
    .await?
    .take(0)?;
```

### TypeScript実装
```typescript
interface Post {
  id?: string;
  title: string;
  author: string;  // "user:john"
}

interface PostWithAuthor {
  id?: string;
  title: string;
  author: User;  // 完全なUserオブジェクト
}

const posts = await db.query<[PostWithAuthor[]]>(
  'SELECT *, author.* FROM post'
);
```

## Reference型（弱参照）

Referenceは存在しないレコードを指せる弱参照です（SurrealDB 2.0+）。

### 定義と使用
```sql
-- Reference型（削除されても参照が残る）
DEFINE FIELD author ON TABLE post TYPE option<reference<user>>;

-- レコードが削除されても参照は残る
CREATE post:1 SET author = user:deleted;  -- user:deletedが存在しなくてもOK

-- Recordとの違い
DEFINE FIELD author ON TABLE post TYPE record<user>;  -- 存在チェックあり
DEFINE FIELD author ON TABLE post TYPE reference<user>;  -- 存在チェックなし
```

### ユースケース
```sql
-- 削除されたユーザーの投稿を保持
DEFINE FIELD author ON TABLE post TYPE option<reference<user>>;

-- ユーザー削除
DELETE user:john;

-- 投稿は残る（author = NULLまたはuser:john参照を保持）
SELECT * FROM post WHERE author = user:john;
```

## GRAPHリレーション（エッジテーブル）

多対多関係用のリレーションテーブル。

### 定義
```sql
-- エッジテーブル
DEFINE TABLE likes SCHEMAFULL;
DEFINE FIELD in ON TABLE likes TYPE record<user>;
DEFINE FIELD out ON TABLE likes TYPE record<post>;
DEFINE FIELD created_at ON TABLE likes TYPE datetime DEFAULT time::now();

-- RELATE文で作成
RELATE user:john->likes->post:1;
RELATE user:john->likes->post:2 SET created_at = time::now();
```

### グラフクエリ
```sql
-- 外向きトラバース（johnがlikeした投稿）
SELECT ->likes->post FROM user:john;

-- 内向きトラバース（post:1をlikeしたユーザー）
SELECT <-likes<-user FROM post:1;

-- 双方向
SELECT <-likes<-user, ->likes->post FROM user:john;

-- フェッチ
SELECT *, ->likes->post.* FROM user:john;
```

### Rust実装
```rust
#[derive(Serialize, Deserialize)]
struct Likes {
    id: Option<Thing>,
    r#in: Thing,   // user
    out: Thing,    // post
    created_at: String,
}

// RELATE実行
db.query("RELATE $user->likes->$post")
    .bind(("user", Thing::from(("user", "john"))))
    .bind(("post", Thing::from(("post", "1"))))
    .await?;

// グラフクエリ
let result = db
    .query("SELECT ->likes->post.* FROM $user")
    .bind(("user", Thing::from(("user", "john"))))
    .await?;
```

### TypeScript実装
```typescript
interface Likes {
  id?: string;
  in: string;   // "user:john"
  out: string;  // "post:1"
  created_at: string;
}

// RELATE実行
await db.query(
  'RELATE $user->likes->$post',
  { user: 'user:john', post: 'post:1' }
);

// グラフクエリ
const result = await db.query<[any[]]>(
  'SELECT ->likes->post.* FROM $user',
  { user: 'user:john' }
);
```

## Option型とNULL

### Option型
```sql
-- Nullable（オプショナル）
DEFINE FIELD email ON TABLE user TYPE option<string>;

-- 値あり
CREATE user:john SET email = 'john@example.com';

-- NULL
CREATE user:jane SET email = NONE;
UPDATE user:jane SET email = NULL;

-- チェック
SELECT * FROM user WHERE email IS NOT NULL;
SELECT * FROM user WHERE email IS NONE;
```

### Rust: Option<T>
```rust
#[derive(Serialize, Deserialize)]
struct User {
    id: Option<Thing>,
    name: String,
    email: Option<String>,  // NULL許可
}
```

### TypeScript: T | null | undefined
```typescript
interface User {
  id?: string;
  name: string;
  email?: string | null;
}
```

## 配列型

### 定義と使用
```sql
-- プリミティブ配列
DEFINE FIELD tags ON TABLE post TYPE array<string>;

-- レコード配列
DEFINE FIELD authors ON TABLE book TYPE array<record<user>>;

-- 作成
CREATE post:1 SET tags = ['tech', 'news'];
CREATE book:1 SET authors = [user:john, user:jane];

-- クエリ
SELECT * FROM post WHERE 'tech' IN tags;
SELECT * FROM book WHERE user:john IN authors;
```

### Rust
```rust
#[derive(Serialize, Deserialize)]
struct Post {
    id: Option<Thing>,
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Book {
    id: Option<Thing>,
    authors: Vec<Thing>,
}
```

## 型変換とキャスト

```sql
-- 文字列から数値
SELECT <int>"123";
SELECT <float>"3.14";

-- レコードIDから文字列
SELECT <string>user:john;  -- "user:john"

-- 時刻型
SELECT <datetime>"2024-01-01T00:00:00Z";
SELECT <duration>"1h30m";
```

## ベストプラクティス

### 1. Record vs Reference
```sql
-- 強参照（削除時エラー）
DEFINE FIELD author ON TABLE post TYPE record<user>;

-- 弱参照（削除後もOK）
DEFINE FIELD author ON TABLE post TYPE option<reference<user>>;

-- 使い分け
-- ✅ Record: 必須の整合性が必要（注文→商品）
-- ✅ Reference: 削除後も保持（投稿→削除済みユーザー）
```

### 2. レコードリンク vs GRAPHリレーション
```sql
-- ✅ レコードリンク: 1対多、多対1
DEFINE FIELD author ON TABLE post TYPE record<user>;

-- ✅ GRAPHリレーション: 多対多
DEFINE TABLE likes SCHEMAFULL;
RELATE user:john->likes->post:1;
```

### 3. ID生成戦略
```sql
-- ✅ 自動UUID（推奨）
CREATE user SET name = 'John';

-- ✅ 文字列ID（可読性）
CREATE user:john SET name = 'John';

-- ✅ ULID（時系列ソート可能）
CREATE user:⟨ulid()⟩ SET name = 'John';

-- ⚠️ 数値ID（既存システム統合のみ）
CREATE user:123 SET name = 'John';
```

### 4. 型安全性
```rust
// ✅ 型を明示
#[derive(Serialize, Deserialize)]
struct User {
    id: Option<Thing>,      // レコードID
    email: Option<String>,  // nullable
    tags: Vec<String>,      // 配列
    profile: Profile,       // ネスト
}

// ❌ 汎用的すぎる
#[derive(Serialize, Deserialize)]
struct User {
    #[serde(flatten)]
    data: HashMap<String, serde_json::Value>,
}
```

## よくあるエラー

### エラー1: 存在しないレコードへの参照
```sql
-- ❌ Recordタイプ: エラー
DEFINE FIELD author ON TABLE post TYPE record<user>;
CREATE post:1 SET author = user:nonexistent;  -- エラー

-- ✅ Reference: OK
DEFINE FIELD author ON TABLE post TYPE reference<user>;
CREATE post:1 SET author = user:nonexistent;  -- OK
```

### エラー2: 循環フェッチ
```sql
-- ❌ 無限ループの可能性
SELECT *, author.* FROM post;

-- ✅ フィールド指定
SELECT *, author.(id, name) FROM post;
```

### エラー3: 型不一致
```rust
// ❌ Thing vs 文字列
let id: String = "user:john".to_string();
db.select::<Option<User>>(id).await?;  // エラー

// ✅ 正しい型
let id = ("user", "john");
db.select::<Option<User>>(id).await?;
```

## 公式リソース

- データモデル: https://surrealdb.com/docs/surrealql/datamodel
- Record IDs: https://surrealdb.com/docs/surrealql/datamodel/ids
- グラフリレーション: https://surrealdb.com/docs/surrealql/statements/relate
- 型システム: https://surrealdb.com/docs/surrealql/datamodel/casting
