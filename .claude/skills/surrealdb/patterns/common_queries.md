# よく使うクエリパターン集

## ユーザー管理

### ユーザーCRUD

```sql
-- ユーザー作成
CREATE user:john CONTENT {
  name: "John Doe",
  email: "john@example.com",
  age: 30,
  created_at: time::now()
};

-- メール検索
SELECT * FROM user WHERE email = "john@example.com";

-- 複数条件検索
SELECT * FROM user 
WHERE age > 25 AND email CONTAINS "@example.com"
ORDER BY created_at DESC
LIMIT 10;

-- ユーザー更新（部分更新）
UPDATE user:john MERGE {
  age: 31,
  updated_at: time::now()
};

-- パスワード変更
UPDATE user:john SET password = crypto::argon2::generate($password);

-- ユーザー削除（ソフトデリート）
UPDATE user:john SET deleted_at = time::now();

-- 論理削除されていないユーザーのみ取得
SELECT * FROM user WHERE deleted_at IS NONE;
```

### ユーザープロフィール

```sql
-- プロフィール詳細（リレーション含む）
SELECT 
  *,
  ->posted->article.* AS articles,
  ->follows->user.name AS following,
  <-follows<-user.name AS followers
FROM user:john;

-- フォロワー数カウント
SELECT 
  name,
  count(<-follows) AS follower_count
FROM user:john;

-- 記事投稿数
SELECT 
  name,
  count(->posted->article) AS article_count
FROM user:john;
```

## 記事管理

### 記事CRUD

```sql
-- 記事作成
LET $article = CREATE article CONTENT {
  title: "SurrealDB入門",
  content: "SurrealDBは...",
  status: "draft",
  created_at: time::now()
};

-- 著者をリレーション
RELATE $user->posted->$article;

-- 公開記事一覧
SELECT * FROM article 
WHERE status = "published"
ORDER BY created_at DESC
LIMIT 20;

-- タグで検索
SELECT * FROM article 
WHERE tags CONTAINS "surrealdb"
AND status = "published";

-- 全文検索
SELECT * FROM article 
WHERE title ~ "SurrealDB" OR content ~ "SurrealDB"
AND status = "published";

-- 記事公開
UPDATE article:123 SET 
  status = "published",
  published_at = time::now();

-- 記事削除
DELETE article:123;
```

### 記事とコメント

```sql
-- コメント追加
LET $comment = CREATE comment CONTENT {
  text: "とても参考になりました！",
  created_at: time::now()
};

RELATE $user->commented->$comment;
RELATE $article->has->$comment;

-- 記事のコメント一覧
SELECT 
  *,
  ->has->comment.{
    text,
    created_at,
    <-commented<-user.name AS author
  } AS comments
FROM article:123;

-- コメント数
SELECT 
  title,
  count(->has->comment) AS comment_count
FROM article:123;
```

## リレーション操作

### フォロー機能

```sql
-- フォロー
RELATE user:alice->follows->user:bob CONTENT {
  created_at: time::now()
};

-- フォロー解除
DELETE user:alice->follows WHERE out = user:bob;

-- フォロー中ユーザー
SELECT ->follows->user.* FROM user:alice;

-- フォロワー
SELECT <-follows<-user.* FROM user:bob;

-- 相互フォロー
SELECT * FROM user:alice->follows WHERE out IN (
  SELECT VALUE id FROM user:alice<-follows
);

-- フォロー推奨（フォロワーのフォロワー）
SELECT 
  id,
  name,
  count() AS mutual_followers
FROM user:alice->follows->user->follows->user
WHERE id != user:alice
AND id NOT IN (SELECT VALUE out FROM user:alice->follows)
GROUP BY id, name
ORDER BY mutual_followers DESC
LIMIT 10;
```

### いいね機能

```sql
-- いいね
RELATE user:alice->likes->article:123 CONTENT {
  created_at: time::now()
};

-- いいね解除
DELETE user:alice->likes WHERE out = article:123;

-- いいね済みかチェック
SELECT * FROM user:alice->likes WHERE out = article:123;

-- いいね数
SELECT 
  title,
  count(<-likes) AS like_count
FROM article:123;

-- いいねしたユーザー一覧
SELECT <-likes<-user.{id, name} FROM article:123;

-- ユーザーがいいねした記事
SELECT ->likes->article.{id, title} FROM user:alice;
```

## 集計・統計

### 基本統計

```sql
-- ユーザー数
SELECT count() FROM user;

-- 年齢別ユーザー数
SELECT age, count() AS user_count 
FROM user 
GROUP BY age 
ORDER BY age;

-- 年齢範囲別
SELECT 
  math::floor(age / 10) * 10 AS age_range,
  count() AS user_count
FROM user
GROUP BY age_range
ORDER BY age_range;

-- 月別記事数
SELECT 
  time::format(created_at, "%Y-%m") AS month,
  count() AS article_count
FROM article
GROUP BY month
ORDER BY month DESC;
```

### ランキング

```sql
-- 記事投稿数ランキング
SELECT 
  id,
  name,
  count(->posted->article) AS article_count
FROM user
ORDER BY article_count DESC
LIMIT 10;

-- いいね数ランキング
SELECT 
  id,
  title,
  count(<-likes) AS like_count
FROM article
WHERE status = "published"
ORDER BY like_count DESC
LIMIT 10;

-- トレンド記事（直近7日間のいいね）
SELECT 
  id,
  title,
  count(<-likes WHERE created_at > time::now() - 7d) AS recent_likes
FROM article
WHERE status = "published"
ORDER BY recent_likes DESC
LIMIT 10;
```

## 検索・フィルタリング

### 複雑な検索

```sql
-- 複数条件でOR検索
SELECT * FROM article
WHERE title CONTAINS "SurrealDB" 
   OR tags CONTAINSANY ["database", "rust"]
   OR author->user.name = "John Doe";

-- 範囲検索
SELECT * FROM article
WHERE created_at >= "2024-01-01" 
  AND created_at < "2024-02-01";

-- NULL/NONEチェック
SELECT * FROM user WHERE email IS NOT NONE;
SELECT * FROM user WHERE deleted_at IS NONE;

-- IN句
SELECT * FROM article 
WHERE status IN ["published", "featured"];

-- サブクエリ
SELECT * FROM article 
WHERE author IN (
  SELECT VALUE id FROM user WHERE age > 30
);
```

### ページネーション

```sql
-- オフセットベース
SELECT * FROM article 
WHERE status = "published"
ORDER BY created_at DESC
LIMIT 20 START 40; -- 3ページ目（0始まり）

-- カーソルベース（推奨）
SELECT * FROM article 
WHERE status = "published" 
  AND created_at < $cursor
ORDER BY created_at DESC
LIMIT 20;

-- 次ページのカーソル取得
LET $results = SELECT * FROM article 
  WHERE status = "published"
  ORDER BY created_at DESC
  LIMIT 21; -- 1件多く取得

-- has_next を判定
RETURN {
  items: array::slice($results, 0, 20),
  has_next: array::len($results) > 20,
  next_cursor: IF array::len($results) > 20 THEN $results[19].created_at ELSE NONE END
};
```

## トランザクション

```sql
-- ユーザー作成とプロフィール初期化
BEGIN TRANSACTION;

LET $user = CREATE user CONTENT {
  name: $name,
  email: $email,
  created_at: time::now()
};

CREATE profile CONTENT {
  user: $user.id,
  bio: "",
  avatar_url: NONE
};

UPDATE stats:global SET user_count += 1;

COMMIT TRANSACTION;
```

```sql
-- 記事公開とアクティビティ記録
BEGIN TRANSACTION;

UPDATE article:123 SET 
  status = "published",
  published_at = time::now();

CREATE activity CONTENT {
  type: "article_published",
  user: $user_id,
  article: article:123,
  created_at: time::now()
};

UPDATE user:$user_id SET article_count += 1;

COMMIT TRANSACTION;
```

## パフォーマンス最適化

### インデックス活用

```sql
-- インデックス定義
DEFINE INDEX email_idx ON TABLE user COLUMNS email UNIQUE;
DEFINE INDEX title_idx ON TABLE article COLUMNS title;
DEFINE INDEX created_at_idx ON TABLE article COLUMNS created_at;

-- インデックスを使った検索（高速）
SELECT * FROM user WHERE email = "john@example.com"; -- email_idx使用
SELECT * FROM article ORDER BY created_at DESC LIMIT 10; -- created_at_idx使用
```

### クエリ最適化

```sql
-- 悪い例：全件取得してカウント
LET $all = SELECT * FROM user;
RETURN count($all);

-- 良い例：count()関数を使う
SELECT count() FROM user;

-- 悪い例：複数クエリ
LET $user = SELECT * FROM user:john;
LET $articles = SELECT * FROM article WHERE author = user:john;

-- 良い例：リレーションで一度に取得
SELECT 
  *,
  ->posted->article.* AS articles
FROM user:john;
```

### バッチ操作

```sql
-- 複数レコード一括作成
INSERT INTO user [
  { name: "Alice", email: "alice@example.com" },
  { name: "Bob", email: "bob@example.com" },
  { name: "Charlie", email: "charlie@example.com" }
];

-- 条件付き一括更新
UPDATE user SET status = "verified"
WHERE email_verified = true
AND status = "pending";

-- 一括削除
DELETE article WHERE status = "draft" AND created_at < time::now() - 30d;
```

## 便利な関数

```sql
-- 日時操作
SELECT 
  created_at,
  time::format(created_at, "%Y-%m-%d") AS date,
  time::day(created_at) AS day_of_week,
  time::now() - created_at AS age
FROM article;

-- 文字列操作
SELECT 
  string::lowercase(name) AS name_lower,
  string::uppercase(name) AS name_upper,
  string::trim(name) AS name_trimmed,
  string::len(name) AS name_length
FROM user;

-- 配列操作
SELECT 
  array::len(tags) AS tag_count,
  array::first(tags) AS first_tag,
  array::contains(tags, "surrealdb") AS has_surreal_tag
FROM article;

-- 数学関数
SELECT 
  math::ceil(age / 10) * 10 AS age_bucket,
  math::round(score, 2) AS rounded_score
FROM user;
```
