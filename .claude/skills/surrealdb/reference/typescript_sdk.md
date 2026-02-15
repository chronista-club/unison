# TypeScript SDK 実践ガイド

## セットアップ

### インストール
```bash
npm install surrealdb
# または
bun add surrealdb
```

## 接続

### WebSocket接続
```typescript
import { Surreal } from 'surrealdb';

const db = new Surreal();

async function connect() {
  await db.connect('ws://localhost:8000/rpc');
  
  await db.signin({
    username: 'root',
    password: 'root',
  });
  
  await db.use({ ns: 'myapp', db: 'production' });
}
```

### HTTP接続
```typescript
await db.connect('http://localhost:8000/rpc');
```

## 型定義

```typescript
interface User {
  id?: string;
  name: string;
  email: string;
  age: number;
}

interface Post {
  id?: string;
  title: string;
  content: string;
  author: string; // レコードID: "user:john"
}
```

## CRUD操作

### CREATE
```typescript
// 自動ID
const [user] = await db.create<User>('user', {
  name: 'John',
  email: 'john@example.com',
  age: 30,
});

// ID指定
const user = await db.create<User>('user:john', {
  name: 'John',
  email: 'john@example.com',
  age: 30,
});
```

### SELECT
```typescript
// 全件取得
const users = await db.select<User>('user');

// ID指定
const user = await db.select<User>('user:john');
```

### UPDATE
```typescript
// 全体更新
const user = await db.update<User>('user:john', {
  name: 'John Smith',
  email: 'john@example.com',
  age: 31,
});

// MERGE（部分更新）
const user = await db.merge<User>('user:john', {
  age: 31,
});
```

### DELETE
```typescript
await db.delete('user:john');
```

## クエリ実行

### 基本クエリ
```typescript
const result = await db.query<[User[]]>(
  'SELECT * FROM user WHERE age > $age',
  { age: 25 }
);

const users = result[0];
```

### 複数ステートメント
```typescript
const result = await db.query<[User, User, User[]]>(`
  CREATE user:alice SET name = 'Alice', age = 28;
  CREATE user:bob SET name = 'Bob', age = 32;
  SELECT * FROM user;
`);

const alice = result[0];
const bob = result[1];
const users = result[2];
```

### パラメータ化クエリ
```typescript
const result = await db.query<[User[]]>(
  `SELECT * FROM user WHERE name = $name AND age > $age`,
  {
    name: 'John',
    age: 25,
  }
);
```

## トランザクション

```typescript
try {
  await db.query('BEGIN TRANSACTION');
  
  await db.create('user:alice', { balance: 100 });
  await db.create('user:bob', { balance: 50 });
  
  await db.query('UPDATE user:alice SET balance -= 50');
  await db.query('UPDATE user:bob SET balance += 50');
  
  await db.query('COMMIT TRANSACTION');
} catch (error) {
  await db.query('CANCEL TRANSACTION');
  throw error;
}
```

## リアルタイムクエリ（Live Query）

```typescript
// ライブクエリの開始
const queryUuid = await db.live<User>(
  'user',
  (action, result) => {
    console.log('Action:', action); // 'CREATE' | 'UPDATE' | 'DELETE'
    console.log('Data:', result);
  }
);

// 購読解除
await db.kill(queryUuid);
```

### 条件付きライブクエリ
```typescript
const queryUuid = await db.live<User>(
  'SELECT * FROM user WHERE age > 25',
  (action, result) => {
    console.log('Adult user changed:', result);
  }
);
```

## エラーハンドリング

```typescript
try {
  const user = await db.select<User>('user:john');
  if (!user) {
    console.log('User not found');
  }
} catch (error) {
  if (error instanceof Error) {
    console.error('Error:', error.message);
  }
}
```

## 実用パターン

### シングルトンパターン
```typescript
// db.ts
export class Database {
  private static instance: Surreal;
  
  static async getInstance(): Promise<Surreal> {
    if (!this.instance) {
      this.instance = new Surreal();
      await this.instance.connect('ws://localhost:8000/rpc');
      await this.instance.signin({
        username: 'root',
        password: 'root',
      });
      await this.instance.use({ ns: 'myapp', db: 'production' });
    }
    return this.instance;
  }
}

// 使用
const db = await Database.getInstance();
```

### リポジトリパターン
```typescript
export class UserRepository {
  constructor(private db: Surreal) {}
  
  async findById(id: string): Promise<User | null> {
    return await this.db.select<User>(`user:${id}`);
  }
  
  async findAll(): Promise<User[]> {
    return await this.db.select<User>('user');
  }
  
  async create(user: Omit<User, 'id'>): Promise<User> {
    const [created] = await this.db.create<User>('user', user);
    return created;
  }
  
  async update(id: string, data: Partial<User>): Promise<User> {
    return await this.db.merge<User>(`user:${id}`, data);
  }
  
  async delete(id: string): Promise<void> {
    await this.db.delete(`user:${id}`);
  }
}
```

### Vue/React統合
```typescript
// Vue Composition API
import { ref, onMounted, onUnmounted } from 'vue';

export function useUsers() {
  const users = ref<User[]>([]);
  let queryUuid: string;
  
  onMounted(async () => {
    const db = await Database.getInstance();
    
    // 初期データ取得
    users.value = await db.select<User>('user');
    
    // ライブクエリ
    queryUuid = await db.live<User>('user', (action, result) => {
      if (action === 'CREATE') {
        users.value.push(result);
      } else if (action === 'UPDATE') {
        const index = users.value.findIndex(u => u.id === result.id);
        if (index !== -1) users.value[index] = result;
      } else if (action === 'DELETE') {
        users.value = users.value.filter(u => u.id !== result.id);
      }
    });
  });
  
  onUnmounted(async () => {
    const db = await Database.getInstance();
    await db.kill(queryUuid);
  });
  
  return { users };
}
```

## 公式ドキュメント

- JavaScript SDK: https://surrealdb.com/docs/sdk/javascript
- API Reference: https://surrealdb.com/docs/sdk/javascript/core
- Examples: https://github.com/surrealdb/surrealdb.js/tree/main/examples
