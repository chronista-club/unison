# Gandi API 基礎

## 認証

### Personal Access Token (PAT) の取得

1. Gandiアカウントにログイン
2. セキュリティページにアクセス: https://account.gandi.net/ja/users/security
3. "Personal Access Token" セクションで新規トークンを作成
4. 適切な権限を選択（最小権限の原則）
5. トークンをコピー（一度しか表示されない）

### API キーの使用

```bash
# 環境変数として設定
export GANDI_API_KEY="your-personal-access-token"

# curlでの使用
curl -H "Authorization: Bearer $GANDI_API_KEY" \
     https://api.gandi.net/v5/domain/domains
```

## ベースURL

```
Production: https://api.gandi.net/v5
Sandbox:    https://api.sandbox.gandi.net/v5
```

## 基本的なリクエスト

### GET リクエスト

```bash
# ドメイン一覧
curl -H "Authorization: Bearer $GANDI_API_KEY" \
     https://api.gandi.net/v5/domain/domains

# 特定のドメイン情報
curl -H "Authorization: Bearer $GANDI_API_KEY" \
     https://api.gandi.net/v5/domain/domains/example.com
```

### POST リクエスト

```bash
# DNSレコード作成
curl -X POST \
     -H "Authorization: Bearer $GANDI_API_KEY" \
     -H "Content-Type: application/json" \
     -d '{"rrset_name": "www", "rrset_type": "A", "rrset_values": ["192.0.2.1"], "rrset_ttl": 300}' \
     https://api.gandi.net/v5/livedns/domains/example.com/records
```

### PUT リクエスト

```bash
# DNSレコード更新
curl -X PUT \
     -H "Authorization: Bearer $GANDI_API_KEY" \
     -H "Content-Type: application/json" \
     -d '{"rrset_values": ["192.0.2.2"], "rrset_ttl": 600}' \
     https://api.gandi.net/v5/livedns/domains/example.com/records/www/A
```

### DELETE リクエスト

```bash
# DNSレコード削除
curl -X DELETE \
     -H "Authorization: Bearer $GANDI_API_KEY" \
     https://api.gandi.net/v5/livedns/domains/example.com/records/www/A
```

## レスポンス形式

### 成功レスポンス

```json
{
  "fqdn": "example.com",
  "domain_href": "https://api.gandi.net/v5/domain/domains/example.com",
  "status": ["clientTransferProhibited"],
  "dates": {
    "created_at": "2020-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
  }
}
```

### エラーレスポンス

```json
{
  "code": 400,
  "message": "Invalid request",
  "errors": [
    {
      "location": "body",
      "name": "rrset_type",
      "description": "Invalid record type"
    }
  ]
}
```

## HTTPステータスコード

| コード | 意味 | 説明 |
|--------|------|------|
| 200 | OK | リクエスト成功 |
| 201 | Created | リソース作成成功 |
| 204 | No Content | 削除成功（レスポンスボディなし） |
| 400 | Bad Request | リクエストが不正 |
| 401 | Unauthorized | 認証失敗 |
| 403 | Forbidden | 権限不足 |
| 404 | Not Found | リソースが存在しない |
| 429 | Too Many Requests | レート制限超過 |
| 500 | Internal Server Error | サーバーエラー |

## レート制限

- APIリクエストには制限があります
- 具体的な制限値は公式ドキュメントを参照
- `429 Too Many Requests` を受け取ったら、リクエスト間隔を空ける

### レート制限の確認

```bash
# レスポンスヘッダーで確認
curl -I -H "Authorization: Bearer $GANDI_API_KEY" \
     https://api.gandi.net/v5/domain/domains

# X-RateLimit-* ヘッダーに注目
# X-RateLimit-Limit: 100
# X-RateLimit-Remaining: 95
# X-RateLimit-Reset: 1699999999
```

## TypeScript + Bun での使用

### プロジェクトセットアップ

```bash
# プロジェクト初期化
mkdir gandi-api-client
cd gandi-api-client
bun init -y

# 型定義ファイル作成
touch gandi-types.ts
touch gandi-client.ts
```

### 型定義

```typescript
// gandi-types.ts
export interface GandiDomain {
  fqdn: string;
  status: string[];
  dates: {
    created_at: string;
    updated_at: string;
    registry_created_at: string;
    registry_ends_at: string;
  };
  nameservers?: string[];
}

export interface GandiDNSRecord {
  rrset_name: string;
  rrset_type: string;
  rrset_values: string[];
  rrset_ttl: number;
}

export interface GandiError {
  code: number;
  message: string;
  errors?: Array<{
    location: string;
    name: string;
    description: string;
  }>;
}
```

### クライアント実装

```typescript
// gandi-client.ts
import type { GandiDomain, GandiDNSRecord } from './gandi-types';

export class GandiClient {
  private apiKey: string;
  private baseUrl: string;

  constructor(apiKey?: string, sandbox = false) {
    this.apiKey = apiKey || process.env.GANDI_API_KEY || '';
    this.baseUrl = sandbox
      ? 'https://api.sandbox.gandi.net/v5'
      : 'https://api.gandi.net/v5';
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const headers = {
      'Authorization': `Bearer ${this.apiKey}`,
      'Content-Type': 'application/json',
      ...options.headers,
    };

    const response = await fetch(url, { ...options, headers });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`API Error: ${error.message || response.statusText}`);
    }

    // 204 No Content の場合
    if (response.status === 204) {
      return {} as T;
    }

    return response.json();
  }

  // ドメイン一覧取得
  async listDomains(): Promise<GandiDomain[]> {
    return this.request<GandiDomain[]>('/domain/domains');
  }

  // ドメイン詳細取得
  async getDomain(domain: string): Promise<GandiDomain> {
    return this.request<GandiDomain>(`/domain/domains/${domain}`);
  }

  // DNSレコード一覧取得
  async listDNSRecords(domain: string): Promise<GandiDNSRecord[]> {
    return this.request<GandiDNSRecord[]>(`/livedns/domains/${domain}/records`);
  }

  // DNSレコード作成
  async createDNSRecord(
    domain: string,
    record: Omit<GandiDNSRecord, 'rrset_name'> & { rrset_name: string }
  ): Promise<{ message: string }> {
    return this.request(`/livedns/domains/${domain}/records`, {
      method: 'POST',
      body: JSON.stringify(record),
    });
  }

  // DNSレコード更新
  async updateDNSRecord(
    domain: string,
    name: string,
    type: string,
    values: string[],
    ttl = 300
  ): Promise<{ message: string }> {
    return this.request(
      `/livedns/domains/${domain}/records/${name}/${type}`,
      {
        method: 'PUT',
        body: JSON.stringify({ rrset_values: values, rrset_ttl: ttl }),
      }
    );
  }

  // DNSレコード削除
  async deleteDNSRecord(
    domain: string,
    name: string,
    type: string
  ): Promise<void> {
    return this.request(`/livedns/domains/${domain}/records/${name}/${type}`, {
      method: 'DELETE',
    });
  }
}
```

### 使用例

```typescript
// example.ts
import { GandiClient } from './gandi-client';

const client = new GandiClient();

// ドメイン一覧取得
const domains = await client.listDomains();
console.log('Domains:', domains);

// 特定のドメイン情報
const domain = await client.getDomain('example.com');
console.log('Domain:', domain);

// DNSレコード取得
const records = await client.listDNSRecords('example.com');
console.log('DNS Records:', records);

// DNSレコード作成
await client.createDNSRecord('example.com', {
  rrset_name: 'www',
  rrset_type: 'A',
  rrset_values: ['192.0.2.1'],
  rrset_ttl: 300,
});
console.log('✅ DNS record created');

// DNSレコード更新
await client.updateDNSRecord('example.com', 'www', 'A', ['192.0.2.2'], 600);
console.log('✅ DNS record updated');
```

### 実行

```bash
# スクリプト実行
bun run example.ts

# または、監視モード
bun --watch example.ts
```

## エラーハンドリング

```typescript
// error-handling.ts
import { GandiClient } from './gandi-client';

const client = new GandiClient();

try {
  const domains = await client.listDomains();
  console.log(domains);
} catch (error) {
  if (error instanceof Error) {
    console.error('Error:', error.message);
    
    // レート制限の場合
    if (error.message.includes('429')) {
      console.log('⏳ Rate limit exceeded. Waiting...');
      await Bun.sleep(5000);
      // リトライ処理
    }
  }
}
```

## セキュリティのベストプラクティス

### 環境変数の管理

```bash
# .env
GANDI_API_KEY=your-api-key-here
```

```typescript
// .envファイルを読み込み（Bunは自動で読み込む）
const apiKey = process.env.GANDI_API_KEY;

if (!apiKey) {
  throw new Error('GANDI_API_KEY is not set');
}

const client = new GandiClient(apiKey);
```

### .gitignore

```bash
# .gitignore
.env
node_modules/
bun.lockb
```

## トラブルシューティング

### 認証エラー (401)

```typescript
// API キーのテスト
const testAuth = async () => {
  try {
    const client = new GandiClient();
    const domains = await client.listDomains();
    console.log('✅ Authentication successful');
  } catch (error) {
    console.error('❌ Authentication failed:', error);
  }
};

await testAuth();
```

### レート制限 (429)

```typescript
// リトライ付きリクエスト
async function retryRequest<T>(
  fn: () => Promise<T>,
  maxRetries = 3,
  delay = 1000
): Promise<T> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      
      console.log(`Retry ${i + 1}/${maxRetries} after ${delay}ms`);
      await Bun.sleep(delay * Math.pow(2, i)); // 指数バックオフ
    }
  }
  throw new Error('Max retries exceeded');
}

// 使用例
const domains = await retryRequest(() => client.listDomains());
```

## 公式ドキュメント

- API ドキュメント: https://api.gandi.net/docs/
- 認証ガイド: https://api.gandi.net/docs/authentication/
- レート制限: https://api.gandi.net/docs/reference/#rate-limiting

---

次のステップ: [DNS レコード管理](./dns_records.md)
