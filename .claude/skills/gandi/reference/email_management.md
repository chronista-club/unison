# メール管理

Gandi APIを使用したメールボックスとエイリアスの管理方法。

## TypeScript実装

```typescript
// gandi-email.ts
export class GandiEmail {
  private apiKey: string;
  private baseUrl = 'https://api.gandi.net/v5/email';

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
      throw new Error(`Email API Error: ${error.message || response.statusText}`);
    }

    if (response.status === 204) {
      return {} as T;
    }

    return response.json();
  }

  // メールボックス一覧
  async listMailboxes(domain: string) {
    return this.request(`/mailboxes/${domain}`);
  }

  // メールボックス作成
  async createMailbox(domain: string, login: string, password: string) {
    return this.request(`/mailboxes/${domain}`, {
      method: 'POST',
      body: JSON.stringify({
        login,
        mailbox_type: 'standard',
        password,
      }),
    });
  }

  // メールボックス削除
  async deleteMailbox(domain: string, login: string) {
    return this.request(`/mailboxes/${domain}/${login}`, {
      method: 'DELETE',
    });
  }

  // 転送作成
  async createForward(domain: string, source: string, destinations: string[]) {
    return this.request(`/forwards/${domain}`, {
      method: 'POST',
      body: JSON.stringify({
        source,
        destinations: Array.isArray(destinations) ? destinations : [destinations],
      }),
    });
  }
}
```

## 使用例

```typescript
// example-email.ts
import { GandiEmail } from './gandi-email';

const email = new GandiEmail();
const domain = 'example.com';

// メールボックス作成
await email.createMailbox(domain, 'contact', 'SecurePassword123!');

// エイリアス作成
await email.createForward(domain, 'info@example.com', ['contact@example.com']);

// メールボックス一覧
const mailboxes = await email.listMailboxes(domain);
for (const mb of mailboxes) {
  console.log(`${mb.login}@${domain}`);
}
```

## 公式ドキュメント

- Email API: https://api.gandi.net/docs/email/

---

次のステップ: [よく使うパターン](./common_patterns.md)
