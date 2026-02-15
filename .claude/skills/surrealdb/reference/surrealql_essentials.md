# SurrealQL 必須パターン集

## 基本構文

### データ型
```sql
-- プリミティブ型
SET @bool = true;
SET @int = 42;
SET @float = 3.14;
SET @string = 'Hello';
SET @datetime = time::now();
SET @duration = 1h30m;

-- 複合型
SET @array = [1, 2, 3];
SET @object = { name: 'John', age: 30 };
SET @record = user:john;
```

### CRUD操作

#### CREATE/INSERT
```sql
-- レコードID指定
CREATE user:john SET name = 'John', age = 30;

-- 自動ID
CREATE user CONTENT { name: 'Jane', age: 25 };

-- バッチ挿入
INSERT INTO user [
    { name: 'Alice', age: 28 },
    { name: 'Bob', age: 32 }
];
```

#### SELECT
```sql
-- 基本
SELECT * FROM user;
SELECT name, age FROM user WHERE age > 25;

-- ソート・制限
SELECT * FROM user ORDER BY age DESC LIMIT 10;

-- グループ化
SELECT count() AS total GROUP BY age FROM user;

-- リレーションフェッチ
SELECT *, author.* FROM post;
```

#### UPDATE
```sql
-- 条件付き更新
UPDATE user SET age = 31 WHERE name = 'John';

-- MERGE（部分更新）
UPDATE user:john MERGE { age: 31 };

-- PATCH（JSONパッチ）
UPDATE user:john PATCH [
    { op: 'replace', path: '/age', value: 31 }
];
```

#### DELETE
```sql
DELETE user:john;
DELETE user WHERE age < 18;
```

## リレーション

### グラフリレーション
```sql
-- リレーションテーブル定義
DEFINE TABLE likes SCHEMAFULL;
DEFINE FIELD in ON TABLE likes TYPE record<user>;
DEFINE FIELD out ON TABLE likes TYPE record<post>;

-- RELATE文でリレーション作成
RELATE user:john->likes->post:1;

-- リレーションクエリ
SELECT * FROM likes WHERE in = user:john;
SELECT ->likes->post FROM user:john;
```

### レコードリンク
```sql
-- レコードリンク型
DEFINE FIELD author ON TABLE post TYPE record<user>;

-- 作成
CREATE post SET title = 'Hello', author = user:john;

-- フェッチ
SELECT *, author.name FROM post;
```

## トランザクション

```sql
BEGIN TRANSACTION;

CREATE user:alice SET balance = 100;
CREATE user:bob SET balance = 50;

UPDATE user:alice SET balance -= 50;
UPDATE user:bob SET balance += 50;

COMMIT TRANSACTION;
-- または CANCEL TRANSACTION;
```

## 関数

### 文字列関数
```sql
SELECT string::lowercase('HELLO');
SELECT string::uppercase('hello');
SELECT string::trim('  hello  ');
SELECT string::concat('Hello', ' ', 'World');
```

### 配列関数
```sql
SELECT array::len([1, 2, 3]);
SELECT array::first([1, 2, 3]);
SELECT array::last([1, 2, 3]);
SELECT array::sort([3, 1, 2]);
```

### 数学関数
```sql
SELECT math::abs(-5);
SELECT math::ceil(3.2);
SELECT math::floor(3.8);
SELECT math::round(3.5);
SELECT math::max([1, 5, 3]);
SELECT math::sum([1, 2, 3]);
```

### 時間関数
```sql
SELECT time::now();
SELECT time::unix();
SELECT time::format(time::now(), '%Y-%m-%d');
SELECT time::hour(time::now());
```

## スキーマ定義

```sql
-- スキーマフルテーブル
DEFINE TABLE user SCHEMAFULL;

-- フィールド定義
DEFINE FIELD name ON TABLE user TYPE string;
DEFINE FIELD email ON TABLE user TYPE string ASSERT string::is::email($value);
DEFINE FIELD age ON TABLE user TYPE int VALUE $value OR 0;

-- インデックス
DEFINE INDEX unique_email ON TABLE user FIELDS email UNIQUE;
DEFINE INDEX idx_name ON TABLE user FIELDS name SEARCH ANALYZER ascii;

-- デフォルト値
DEFINE FIELD created_at ON TABLE user TYPE datetime DEFAULT time::now();
```

## 高度なクエリ

### サブクエリ
```sql
SELECT * FROM user WHERE id IN (
    SELECT author FROM post WHERE published = true
);
```

### 条件式
```sql
SELECT name, IF age >= 18 THEN 'adult' ELSE 'minor' END AS status FROM user;
```

### LET変数
```sql
LET $min_age = 18;
SELECT * FROM user WHERE age >= $min_age;
```

## パラメータ化クエリ

```sql
-- CLI
SELECT * FROM user WHERE age > $age;
-- パラメータ: { "age": 25 }

-- SDK使用時も同様
```

## 公式ドキュメント

- SurrealQL: https://surrealdb.com/docs/surrealql
- 関数リファレンス: https://surrealdb.com/docs/surrealql/functions
- データ型: https://surrealdb.com/docs/surrealql/datamodel
