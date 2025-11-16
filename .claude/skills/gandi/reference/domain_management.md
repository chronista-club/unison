# ドメイン管理

Gandi APIを使用したドメインの管理方法。

## TypeScript実装

```typescript
// gandi-domain.ts
export class GandiDomain {
  private apiKey: string;
  private baseUrl = 'https://api.gandi.net/v5/domain';

  constructor(apiKey?: string) {
    this.apiKey = apiKey || process.env.GANDI_API_KEY || '';
  }

  private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const headers = {
      'Authorization': `Bearer ${this.apiKey}`,
      'Content-Type': 'application/json',
      ...options.headers,
    };

    const response = await fetch(url, { ...options, headers });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Domain API Error: ${error.message || response.statusText}`);
    }

    return response.json();
  }

  // ドメイン一覧取得
  async listDomains() {
    return this.request('/domains');
  }

  // ドメイン詳細取得
  async getDomain(domain: string) {
    return this.request(`/domains/${domain}`);
  }

  // ネームサーバー更新
  async updateNameservers(domain: string, nameservers: string[]) {
    return this.request(`/domains/${domain}/nameservers`, {
      method: 'PUT',
      body: JSON.stringify({ nameservers }),
    });
  }

  // 自動更新設定
  async setAutorenew(domain: string, enabled: boolean) {
    return this.request(`/domains/${domain}/autorenew`, {
      method: 'PUT',
      body: JSON.stringify({ enabled }),
    });
  }
}
```

## 使用例

```typescript
// example-domain.ts
import { GandiDomain } from './gandi-domain';

const domain = new GandiDomain();

// ドメイン一覧
const domains = await domain.listDomains();
for (const d of domains) {
  console.log(`${d.fqdn} - Expires: ${d.dates.registry_ends_at}`);
}

// ネームサーバー更新
await domain.updateNameservers('example.com', [
  'ns1.gandi.net',
  'ns2.gandi.net',
]);

// 自動更新有効化
await domain.setAutorenew('example.com', true);
```

## 公式ドキュメント

- Domain API: https://api.gandi.net/docs/domains/

---

次のステップ: [メール管理](./email_management.md)
