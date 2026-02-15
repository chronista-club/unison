# MCP統合ガイド

## SurrealDB MCP Serverの使い方

VantageではSurrealDB MCP Serverが統合されており、Claude経由でSurrealDBを操作できます。

### 接続管理

#### エンドポイント接続

```typescript
// ローカルWebSocket接続
await mcp__surrealdb_mcp__connect_endpoint({
  endpoint: "ws://localhost:8000",
  namespace: "myapp",
  database: "production",
  username: "root",
  password: "root"
});

// メモリモード（テスト用）
await mcp__surrealdb_mcp__connect_endpoint({
  endpoint: "memory"
});

// RocksDB（埋め込み）
await mcp__surrealdb_mcp__connect_endpoint({
  endpoint: "rocksdb://.vantage/database.db",
  namespace: "myapp",
  database: "production"
});

// SurrealDB Cloud
await mcp__surrealdb_mcp__connect_endpoint({
  endpoint: "cloud:instance_id",
  namespace: "myapp",
  database: "production"
});
```

#### 名前空間/データベース切り替え

```typescript
// 名前空間切り替え
await mcp__surrealdb_mcp__use_namespace({
  namespace: "development"
});

// データベース切り替え
await mcp__surrealdb_mcp__use_database({
  database: "testing"
});
```

#### 接続解除

```typescript
await mcp__surrealdb_mcp__disconnect_endpoint();
```

### CRUD操作

#### SELECT

```typescript
// 全件取得
await mcp__surrealdb_mcp__select({
  targets: ["user"]
});

// 特定レコード
await mcp__surrealdb_mcp__select({
  targets: ["user:john"]
});

// WHERE句
await mcp__surrealdb_mcp__select({
  targets: ["user"],
  where_clause: "age > 25"
});

// ORDER BY + LIMIT
await mcp__surrealdb_mcp__select({
  targets: ["user"],
  where_clause: "age > 18",
  order_clause: "name ASC",
  limit_clause: "10"
});

// パラメータ化クエリ
await mcp__surrealdb_mcp__select({
  targets: ["user"],
  where_clause: "age > $min_age AND name CONTAINS $filter",
  parameters: {
    min_age: 25,
    filter: "John"
  }
});
```

#### CREATE

```typescript
// レコード作成
await mcp__surrealdb_mcp__create({
  target: "user:john",
  data: {
    name: "John Doe",
    age: 30,
    email: "john@example.com"
  }
});

// テーブルに作成（IDは自動生成）
await mcp__surrealdb_mcp__create({
  target: "user",
  data: {
    name: "Jane Smith",
    age: 28
  }
});
```

#### UPDATE

```typescript
// CONTENT（上書き）
await mcp__surrealdb_mcp__update({
  targets: ["user:john"],
  content_data: {
    name: "John Doe",
    age: 31,
    email: "john@example.com"
  }
});

// MERGE（部分更新）
await mcp__surrealdb_mcp__update({
  targets: ["user:john"],
  merge_data: {
    age: 31
  }
});

// WHERE句付き
await mcp__surrealdb_mcp__update({
  targets: ["user"],
  merge_data: {
    status: "active"
  },
  where_clause: "age > 25"
});
```

#### UPSERT

```typescript
// 存在しなければ作成、存在すれば更新
await mcp__surrealdb_mcp__upsert({
  targets: ["user:john"],
  content_data: {
    name: "John Doe",
    age: 30
  }
});
```

#### DELETE

```typescript
// 特定レコード削除
await mcp__surrealdb_mcp__delete({
  targets: ["user:john"]
});

// 条件付き削除
await mcp__surrealdb_mcp__delete({
  targets: ["user"],
  where_clause: "age < 18"
});
```

#### INSERT

```typescript
// バッチインサート
await mcp__surrealdb_mcp__insert({
  target: "user",
  values: [
    { name: "Alice", age: 25 },
    { name: "Bob", age: 30 },
    { name: "Charlie", age: 35 }
  ]
});

// IGNORE（重複無視）
await mcp__surrealdb_mcp__insert({
  target: "user",
  values: [{ id: "user:john", name: "John" }],
  ignore: true
});
```

### リレーション操作

#### RELATE

```typescript
// リレーション作成
await mcp__surrealdb_mcp__relate({
  from: ["person:john"],
  table: "wrote",
  with: ["article:surreal_guide"]
});

// エッジデータ付き
await mcp__surrealdb_mcp__relate({
  from: ["person:john"],
  table: "knows",
  with: ["person:jane"],
  content_data: {
    since: "2020-01-01",
    strength: "close"
  }
});

// 複数リレーション
await mcp__surrealdb_mcp__relate({
  from: ["user:alice"],
  table: "likes",
  with: ["post:123", "post:456"],
  content_data: {
    timestamp: "2024-01-15T10:30:00Z"
  }
});
```

### 生SQLクエリ

```typescript
// 複雑なクエリ
await mcp__surrealdb_mcp__query({
  query: `
    SELECT 
      id,
      name,
      ->wrote->article.title AS articles
    FROM person
    WHERE age > $min_age
  `,
  parameters: {
    min_age: 25
  }
});

// トランザクション
await mcp__surrealdb_mcp__query({
  query: `
    BEGIN TRANSACTION;
    
    CREATE user:new SET name = 'New User';
    UPDATE stats:global SET user_count += 1;
    
    COMMIT TRANSACTION;
  `
});
```

### スキーマ管理

```typescript
// テーブル定義
await mcp__surrealdb_mcp__query({
  query: `
    DEFINE TABLE user SCHEMAFULL OVERWRITE;
    DEFINE FIELD name ON TABLE user TYPE string OVERWRITE;
    DEFINE FIELD email ON TABLE user TYPE string OVERWRITE;
    DEFINE FIELD age ON TABLE user TYPE int OVERWRITE;
    DEFINE INDEX email_idx ON TABLE user COLUMNS email UNIQUE OVERWRITE;
  `
});

// 複数ファイルからスキーマ適用（Vantage経由）
// 1. スキーマファイルを読み込み
// 2. MCPツールで順次実行
const schemaFiles = [
  "schema/01_tables/user.surql",
  "schema/02_indexes/user_indexes.surql"
];

for (const file of schemaFiles) {
  const content = await readFile(file);
  await mcp__surrealdb_mcp__query({ query: content });
}
```

### 名前空間/データベース一覧

```typescript
// 名前空間一覧
await mcp__surrealdb_mcp__list_namespaces();

// データベース一覧
await mcp__surrealdb_mcp__list_databases();
```

## Vantage統合パターン

### SurrealDBをVantageプロセスとして管理

```typescript
// 1. SurrealDBサーバーをVantageプロセスとして追加
await mcp__vantage_mcp_df__create_process({
  id: "surrealdb-server",
  command: "surreal",
  args: [
    "start",
    "--user", "vtg-local",
    "--pass", "vtg-local",
    "--bind", "0.0.0.0:30300",
    "rocksdb://.vantage/database.db"
  ],
  auto_start_on_restore: true
});

// 2. 起動
await mcp__vantage_mcp_df__start_process({
  id: "surrealdb-server"
});

// 3. MCP経由で接続
await mcp__surrealdb_mcp__connect_endpoint({
  endpoint: "ws://localhost:30300",
  namespace: "myapp",
  database: "production",
  username: "vtg-local",
  password: "vtg-local"
});

// 4. スナップショット保存（Vantage設定保存）
await mcp__vantage_mcp_df__create_snapshot();
```

### 開発ワークフロー

```typescript
// 開発環境セットアップ
async function setupDevEnvironment() {
  // 1. Vantageプロセス確認
  const processes = await mcp__vantage_mcp_df__list_processes();
  const surrealProcess = processes.find(p => p.id === "surrealdb-server");
  
  if (!surrealProcess) {
    // プロセス作成
    await mcp__vantage_mcp_df__create_process({
      id: "surrealdb-server",
      command: "surreal",
      args: ["start", "--bind", "127.0.0.1:8000", "memory"],
      auto_start_on_restore: false
    });
  }
  
  // 2. 起動
  await mcp__vantage_mcp_df__start_process({ id: "surrealdb-server" });
  
  // 3. 接続
  await mcp__surrealdb_mcp__connect_endpoint({
    endpoint: "ws://localhost:8000",
    namespace: "dev",
    database: "test"
  });
  
  // 4. スキーマ適用
  await applySchema();
  
  // 5. テストデータ投入
  await seedData();
}

async function applySchema() {
  const schemaFiles = [
    "schema/01_tables/user.surql",
    "schema/01_tables/article.surql",
    "schema/02_indexes/user_indexes.surql"
  ];
  
  for (const file of schemaFiles) {
    const content = await readFile(file);
    await mcp__surrealdb_mcp__query({ query: content });
  }
}

async function seedData() {
  await mcp__surrealdb_mcp__insert({
    target: "user",
    values: [
      { id: "user:john", name: "John Doe", age: 30 },
      { id: "user:jane", name: "Jane Smith", age: 28 }
    ]
  });
}
```

### 本番環境デプロイ

```typescript
// 本番環境へのスキーマデプロイ
async function deployToProduction() {
  // 1. 本番サーバーに接続
  await mcp__surrealdb_mcp__connect_endpoint({
    endpoint: "wss://prod.example.com",
    namespace: "myapp",
    database: "production",
    username: process.env.SURREAL_USER,
    password: process.env.SURREAL_PASS
  });
  
  // 2. バックアップ
  await createBackup();
  
  // 3. スキーマ適用（冪等性があるので安全）
  await applySchema();
  
  // 4. 検証
  await verifySchema();
}

async function createBackup() {
  // Vantage経由でバックアップコマンド実行
  await mcp__vantage_mcp_df__create_process({
    id: "backup-job",
    command: "surreal",
    args: [
      "export",
      "--endpoint", "https://prod.example.com",
      "--namespace", "myapp",
      "--database", "production",
      "--username", process.env.SURREAL_USER,
      "--password", process.env.SURREAL_PASS,
      `backups/backup_${new Date().toISOString()}.surql`
    ]
  });
  
  await mcp__vantage_mcp_df__start_process({ id: "backup-job" });
  // 完了を待機
}
```

## エラーハンドリング

```typescript
async function safeQuery(query: string, params?: Record<string, any>) {
  try {
    return await mcp__surrealdb_mcp__query({ query, parameters: params });
  } catch (error) {
    if (error.message.includes("Connection refused")) {
      console.error("SurrealDB server is not running");
      // Vantageプロセスを確認/起動
      await ensureSurrealDBRunning();
      return await mcp__surrealdb_mcp__query({ query, parameters: params });
    }
    throw error;
  }
}

async function ensureSurrealDBRunning() {
  const status = await mcp__vantage_mcp_df__get_process_status({
    id: "surrealdb-server"
  });
  
  if (status.state !== "Running") {
    await mcp__vantage_mcp_df__start_process({ id: "surrealdb-server" });
    // 起動を待つ
    await new Promise(resolve => setTimeout(resolve, 2000));
  }
}
```

## ベストプラクティス

1. **接続管理**: Vantageプロセスとして管理し、自動起動を有効化
2. **パラメータ化**: SQLインジェクション防止のため必ずパラメータを使用
3. **エラー処理**: 接続エラーを捕捉し、適切にリトライ
4. **トランザクション**: 複数操作は BEGIN/COMMIT で囲む
5. **スキーマバージョニング**: OVERWRITE で冪等性を確保
6. **バックアップ**: デプロイ前に必ずバックアップ
