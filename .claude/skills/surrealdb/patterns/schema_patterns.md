# スキーマパターン集

## 基本的なテーブル設計

### ユーザーテーブル

```sql
-- schema/01_tables/user.surql
DEFINE TABLE user SCHEMAFULL OVERWRITE;

DEFINE FIELD name ON TABLE user TYPE string OVERWRITE
  ASSERT $value != NONE AND string::len($value) >= 2;

DEFINE FIELD email ON TABLE user TYPE string OVERWRITE
  ASSERT string::is::email($value);

DEFINE FIELD password ON TABLE user TYPE string OVERWRITE
  PERMISSIONS FOR select NONE; -- パスワードは選択不可

DEFINE FIELD age ON TABLE user TYPE int OVERWRITE
  ASSERT $value >= 0 AND $value <= 150;

DEFINE FIELD status ON TABLE user TYPE string OVERWRITE
  ASSERT $value IN ["active", "inactive", "suspended"]
  DEFAULT "active";

DEFINE FIELD created_at ON TABLE user TYPE datetime OVERWRITE
  DEFAULT time::now();

DEFINE FIELD updated_at ON TABLE user TYPE datetime OVERWRITE
  DEFAULT time::now()
  VALUE time::now();

DEFINE FIELD deleted_at ON TABLE user TYPE option<datetime> OVERWRITE;

-- インデックス
DEFINE INDEX email_idx ON TABLE user COLUMNS email UNIQUE OVERWRITE;
DEFINE INDEX status_idx ON TABLE user COLUMNS status OVERWRITE;
DEFINE INDEX created_at_idx ON TABLE user COLUMNS created_at OVERWRITE;
```

### 記事テーブル

```sql
-- schema/01_tables/article.surql
DEFINE TABLE article SCHEMAFULL OVERWRITE;

DEFINE FIELD title ON TABLE article TYPE string OVERWRITE
  ASSERT $value != NONE AND string::len($value) >= 1;

DEFINE FIELD slug ON TABLE article TYPE string OVERWRITE
  ASSERT $value != NONE;

DEFINE FIELD content ON TABLE article TYPE string OVERWRITE;

DEFINE FIELD excerpt ON TABLE article TYPE option<string> OVERWRITE;

DEFINE FIELD status ON TABLE article TYPE string OVERWRITE
  ASSERT $value IN ["draft", "published", "archived"]
  DEFAULT "draft";

DEFINE FIELD tags ON TABLE article TYPE array<string> OVERWRITE
  DEFAULT [];

DEFINE FIELD author ON TABLE article TYPE record<user> OVERWRITE;

DEFINE FIELD view_count ON TABLE article TYPE int OVERWRITE
  DEFAULT 0;

DEFINE FIELD created_at ON TABLE article TYPE datetime OVERWRITE
  DEFAULT time::now();

DEFINE FIELD published_at ON TABLE article TYPE option<datetime> OVERWRITE;

-- インデックス
DEFINE INDEX slug_idx ON TABLE article COLUMNS slug UNIQUE OVERWRITE;
DEFINE INDEX status_idx ON TABLE article COLUMNS status OVERWRITE;
DEFINE INDEX author_idx ON TABLE article COLUMNS author OVERWRITE;
DEFINE INDEX published_at_idx ON TABLE article COLUMNS published_at OVERWRITE;
```

## リレーションテーブル

### フォロー関係

```sql
-- schema/03_relations/follows.surql
DEFINE TABLE follows SCHEMAFULL TYPE RELATION OVERWRITE
  IN user OUT user;

DEFINE FIELD created_at ON TABLE follows TYPE datetime OVERWRITE
  DEFAULT time::now();

-- 自分自身をフォローできない
DEFINE FIELD in ON TABLE follows OVERWRITE
  ASSERT $value != $parent.out;

-- ユニークインデックス（重複フォロー防止）
DEFINE INDEX follows_unique_idx ON TABLE follows COLUMNS in, out UNIQUE OVERWRITE;
```

### いいね

```sql
-- schema/03_relations/likes.surql
DEFINE TABLE likes SCHEMAFULL TYPE RELATION OVERWRITE
  IN user OUT article;

DEFINE FIELD created_at ON TABLE likes TYPE datetime OVERWRITE
  DEFAULT time::now();

DEFINE INDEX likes_unique_idx ON TABLE likes COLUMNS in, out UNIQUE OVERWRITE;
```

### コメント（エッジデータ付き）

```sql
-- schema/03_relations/commented.surql
DEFINE TABLE commented SCHEMAFULL TYPE RELATION OVERWRITE
  IN user OUT comment;

DEFINE FIELD created_at ON TABLE commented TYPE datetime OVERWRITE
  DEFAULT time::now();

-- コメントテーブル
DEFINE TABLE comment SCHEMAFULL OVERWRITE;

DEFINE FIELD text ON TABLE comment TYPE string OVERWRITE
  ASSERT $value != NONE AND string::len($value) >= 1;

DEFINE FIELD article ON TABLE comment TYPE record<article> OVERWRITE;

DEFINE FIELD created_at ON TABLE comment TYPE datetime OVERWRITE
  DEFAULT time::now();

DEFINE FIELD updated_at ON TABLE comment TYPE option<datetime> OVERWRITE;
```

## 複雑なデータ型

### JSON/オブジェクト型

```sql
DEFINE TABLE product SCHEMAFULL OVERWRITE;

DEFINE FIELD name ON TABLE product TYPE string OVERWRITE;

DEFINE FIELD metadata ON TABLE product TYPE object OVERWRITE;

-- ネストしたフィールド
DEFINE FIELD metadata.dimensions ON TABLE product TYPE object OVERWRITE;
DEFINE FIELD metadata.dimensions.width ON TABLE product TYPE float OVERWRITE;
DEFINE FIELD metadata.dimensions.height ON TABLE product TYPE float OVERWRITE;
DEFINE FIELD metadata.dimensions.depth ON TABLE product TYPE float OVERWRITE;

DEFINE FIELD metadata.colors ON TABLE product TYPE array<string> OVERWRITE
  DEFAULT [];
```

### 配列型

```sql
DEFINE TABLE project SCHEMAFULL OVERWRITE;

DEFINE FIELD name ON TABLE project TYPE string OVERWRITE;

-- 文字列配列
DEFINE FIELD tags ON TABLE project TYPE array<string> OVERWRITE
  DEFAULT [];

-- レコード配列
DEFINE FIELD members ON TABLE project TYPE array<record<user>> OVERWRITE
  DEFAULT [];

-- オブジェクト配列
DEFINE FIELD tasks ON TABLE project TYPE array<object> OVERWRITE
  DEFAULT [];

DEFINE FIELD tasks[*].title ON TABLE project TYPE string OVERWRITE;
DEFINE FIELD tasks[*].status ON TABLE project TYPE string OVERWRITE
  ASSERT $value IN ["todo", "in_progress", "done"];
DEFINE FIELD tasks[*].assignee ON TABLE project TYPE option<record<user>> OVERWRITE;
```

### Union型（option）

```sql
DEFINE TABLE notification SCHEMAFULL OVERWRITE;

DEFINE FIELD user ON TABLE notification TYPE record<user> OVERWRITE;

DEFINE FIELD type ON TABLE notification TYPE string OVERWRITE
  ASSERT $value IN ["follow", "like", "comment"];

-- 通知元（ユーザーまたはシステム）
DEFINE FIELD source ON TABLE notification TYPE option<record<user>> OVERWRITE;

DEFINE FIELD read_at ON TABLE notification TYPE option<datetime> OVERWRITE;
```

## パーミッション

### 行レベルセキュリティ

```sql
-- schema/04_permissions/article_permissions.surql

-- 全員が公開記事を閲覧可能
DEFINE FIELD status ON TABLE article 
  PERMISSIONS 
    FOR select WHERE status = "published" OR $auth.id = author
    FOR create, update WHERE $auth.id != NONE
    FOR delete WHERE $auth.id = author
  OVERWRITE;

-- 自分の記事のみ編集可能
DEFINE FIELD content ON TABLE article
  PERMISSIONS
    FOR select WHERE status = "published" OR $auth.id = author
    FOR update WHERE $auth.id = author
  OVERWRITE;
```

### ロールベースアクセス制御

```sql
-- schema/01_tables/user.surql
DEFINE FIELD role ON TABLE user TYPE string OVERWRITE
  ASSERT $value IN ["user", "moderator", "admin"]
  DEFAULT "user";

-- schema/04_permissions/admin_permissions.surql
DEFINE TABLE user
  PERMISSIONS
    FOR select WHERE published = true OR $auth.id = id OR $auth.role = "admin"
    FOR create WHERE $auth.role = "admin"
    FOR update WHERE $auth.id = id OR $auth.role = "admin"
    FOR delete WHERE $auth.role = "admin"
  OVERWRITE;
```

### フィールドレベルセキュリティ

```sql
DEFINE FIELD email ON TABLE user TYPE string
  PERMISSIONS
    FOR select WHERE $auth.id = $parent.id OR $auth.role = "admin"
    FOR update WHERE $auth.id = $parent.id
  OVERWRITE;

DEFINE FIELD password ON TABLE user TYPE string
  PERMISSIONS
    FOR select NONE -- パスワードは決して読めない
    FOR update WHERE $auth.id = $parent.id
  OVERWRITE;
```

## インデックス戦略

### 基本インデックス

```sql
-- ユニークインデックス
DEFINE INDEX email_idx ON TABLE user COLUMNS email UNIQUE OVERWRITE;

-- 通常インデックス（検索高速化）
DEFINE INDEX status_idx ON TABLE article COLUMNS status OVERWRITE;

-- 複合インデックス
DEFINE INDEX author_status_idx ON TABLE article COLUMNS author, status OVERWRITE;
```

### 全文検索インデックス

```sql
-- SurrealDB 2.x
DEFINE INDEX article_search_idx ON TABLE article 
  COLUMNS title, content 
  SEARCH ANALYZER simple 
  OVERWRITE;

-- 検索クエリ
SELECT * FROM article WHERE title @@ "SurrealDB";
```

## 計算フィールド（computed fields）

### 仮想フィールド

```sql
DEFINE TABLE user SCHEMAFULL OVERWRITE;

DEFINE FIELD first_name ON TABLE user TYPE string OVERWRITE;
DEFINE FIELD last_name ON TABLE user TYPE string OVERWRITE;

-- 計算フィールド（保存されない）
DEFINE FIELD full_name ON TABLE user 
  VALUE string::concat($this.first_name, " ", $this.last_name)
  OVERWRITE;
```

### 自動更新フィールド

```sql
-- 更新時に自動更新
DEFINE FIELD updated_at ON TABLE article TYPE datetime
  DEFAULT time::now()
  VALUE time::now()
  OVERWRITE;

-- カウンター（手動更新）
DEFINE FIELD comment_count ON TABLE article TYPE int
  DEFAULT 0
  OVERWRITE;

-- トリガーで自動更新（将来のバージョン）
-- DEFINE EVENT comment_added ON TABLE comment
-- WHEN $event = "CREATE"
-- THEN (
--   UPDATE article SET comment_count += 1 WHERE id = $after.article
-- );
```

## ソフトデリート

```sql
DEFINE TABLE user SCHEMAFULL OVERWRITE;

DEFINE FIELD deleted_at ON TABLE user TYPE option<datetime> OVERWRITE;

-- 削除されていないレコードのみを返すビュー的な使い方
-- クエリで WHERE deleted_at IS NONE を使う

-- 論理削除
UPDATE user:john SET deleted_at = time::now();

-- 論理削除されていないユーザー
SELECT * FROM user WHERE deleted_at IS NONE;

-- 完全削除
DELETE user WHERE deleted_at IS NOT NONE AND deleted_at < time::now() - 30d;
```

## バージョニング

```sql
DEFINE TABLE article SCHEMAFULL OVERWRITE;

DEFINE FIELD title ON TABLE article TYPE string OVERWRITE;
DEFINE FIELD content ON TABLE article TYPE string OVERWRITE;
DEFINE FIELD version ON TABLE article TYPE int DEFAULT 1 OVERWRITE;
DEFINE FIELD updated_at ON TABLE article TYPE datetime 
  DEFAULT time::now() 
  VALUE time::now() 
  OVERWRITE;

-- 履歴テーブル
DEFINE TABLE article_history SCHEMAFULL OVERWRITE;

DEFINE FIELD article_id ON TABLE article_history TYPE record<article> OVERWRITE;
DEFINE FIELD version ON TABLE article_history TYPE int OVERWRITE;
DEFINE FIELD title ON TABLE article_history TYPE string OVERWRITE;
DEFINE FIELD content ON TABLE article_history TYPE string OVERWRITE;
DEFINE FIELD created_at ON TABLE article_history TYPE datetime 
  DEFAULT time::now() 
  OVERWRITE;

-- 更新時に履歴を作成（アプリケーションコード）
BEGIN TRANSACTION;

-- 現在のバージョンを履歴に保存
LET $current = SELECT * FROM article:123;
CREATE article_history CONTENT {
  article_id: article:123,
  version: $current.version,
  title: $current.title,
  content: $current.content
};

-- 記事を更新
UPDATE article:123 SET 
  title = $new_title,
  content = $new_content,
  version += 1;

COMMIT TRANSACTION;
```

## タイムスタンプパターン

```sql
DEFINE TABLE base_model SCHEMAFULL OVERWRITE;

-- 作成日時（不変）
DEFINE FIELD created_at ON TABLE base_model TYPE datetime
  DEFAULT time::now()
  PERMISSIONS FOR update NONE
  OVERWRITE;

-- 更新日時（自動更新）
DEFINE FIELD updated_at ON TABLE base_model TYPE datetime
  DEFAULT time::now()
  VALUE time::now()
  OVERWRITE;

-- 削除日時（ソフトデリート）
DEFINE FIELD deleted_at ON TABLE base_model TYPE option<datetime>
  OVERWRITE;
```

## 多対多リレーション

```sql
-- タグ付け（記事とタグの多対多）
DEFINE TABLE article SCHEMAFULL OVERWRITE;
DEFINE TABLE tag SCHEMAFULL OVERWRITE;

DEFINE FIELD name ON TABLE tag TYPE string OVERWRITE;
DEFINE INDEX tag_name_idx ON TABLE tag COLUMNS name UNIQUE OVERWRITE;

-- リレーションテーブル
DEFINE TABLE tagged SCHEMAFULL TYPE RELATION OVERWRITE
  IN article OUT tag;

DEFINE FIELD created_at ON TABLE tagged TYPE datetime
  DEFAULT time::now()
  OVERWRITE;

-- 記事にタグを追加
RELATE article:123->tagged->tag:surrealdb;

-- 記事のタグ一覧
SELECT ->tagged->tag.name AS tags FROM article:123;

-- タグの記事一覧
SELECT <-tagged<-article.{id, title} FROM tag:surrealdb;
```

## 階層構造

```sql
-- カテゴリの階層構造
DEFINE TABLE category SCHEMAFULL OVERWRITE;

DEFINE FIELD name ON TABLE category TYPE string OVERWRITE;

-- 親カテゴリ（自己参照）
DEFINE FIELD parent ON TABLE category TYPE option<record<category>> OVERWRITE;

DEFINE FIELD level ON TABLE category TYPE int DEFAULT 0 OVERWRITE;

-- ルートカテゴリ
SELECT * FROM category WHERE parent IS NONE;

-- 特定カテゴリの子
SELECT * FROM category WHERE parent = category:programming;

-- 再帰的な子孫取得（手動実装が必要）
```

## ベストプラクティス

1. **SCHEMAFULL推奨**: 本番環境では型安全性のため SCHEMAFULL を使用
2. **ASSERT で制約**: データ整合性を保つためのバリデーション
3. **DEFAULT で初期値**: NOT NULL制約の代わりにデフォルト値を設定
4. **OVERWRITE で冪等性**: スキーマ更新時の冪等性を確保
5. **インデックスは必要最小限**: 書き込みパフォーマンスとのトレードオフ
6. **パーミッションは明示的に**: セキュリティは後から追加するのではなく最初から
7. **タイムスタンプは標準化**: created_at, updated_at, deleted_at のパターンを統一
