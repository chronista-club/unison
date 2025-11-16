# DNS レコード管理

Gandi LiveDNS APIを使用したDNSレコードの管理方法。

## エンドポイント

```
ベースURL: https://api.gandi.net/v5/livedns
```

## DNSレコードの種類

| タイプ | 説明 | 例 |
|--------|------|-----|
| A | IPv4アドレス | 192.0.2.1 |
| AAAA | IPv6アドレス | 2001:db8::1 |
| CNAME | 別名 | www.example.com |
| MX | メールサーバー | 10 mail.example.com |
| TXT | テキスト | "v=spf1 include:_spf.google.com ~all" |
| SRV | サービス | 10 5 5060 sipserver.example.com |
| NS | ネームサーバー | ns1.gandi.net |
| CAA | 証明書認証局 | 0 issue "letsencrypt.org" |

## TypeScript型定義

```typescript
// types/dns.ts
export interface DNSRecord {
  rrset_name: string;
  rrset_type: 'A' | 'AAAA' | 'CNAME' | 'MX' | 'TXT' | 'SRV' | 'NS' | 'CAA';
  rrset_values: string[];
  rrset_ttl: number;
}

export interface CreateDNSRecordRequest {
  rrset_name: string;
  rrset_type: string;
  rrset_values: string[];
  rrset_ttl?: number;
}

export interface UpdateDNSRecordRequest {
  rrset_values: string[];
  rrset_ttl?: number;
}
```

## DNS管理クラス

```typescript
// gandi-dns.ts
import type { DNSRecord, CreateDNSRecordRequest, UpdateDNSRecordRequest } from './types/dns';

export class GandiDNS {
  private apiKey: string;
  private baseUrl = 'https://api.gandi.net/v5/livedns';

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
      throw new Error(`DNS API Error: ${error.message || response.statusText}`);
    }

    if (response.status === 204) {
      return {} as T;
    }

    return response.json();
  }

  // ドメインのDNSレコード一覧取得
  async listRecords(domain: string): Promise<DNSRecord[]> {
    return this.request<DNSRecord[]>(`/domains/${domain}/records`);
  }

  // 特定のレコード取得
  async getRecord(domain: string, name: string, type: string): Promise<DNSRecord> {
    return this.request<DNSRecord>(`/domains/${domain}/records/${name}/${type}`);
  }

  // DNSレコード作成
  async createRecord(domain: string, record: CreateDNSRecordRequest): Promise<{ message: string }> {
    return this.request(`/domains/${domain}/records`, {
      method: 'POST',
      body: JSON.stringify(record),
    });
  }

  // DNSレコード更新
  async updateRecord(
    domain: string,
    name: string,
    type: string,
    update: UpdateDNSRecordRequest
  ): Promise<{ message: string }> {
    return this.request(`/domains/${domain}/records/${name}/${type}`, {
      method: 'PUT',
      body: JSON.stringify(update),
    });
  }

  // DNSレコード削除
  async deleteRecord(domain: string, name: string, type: string): Promise<void> {
    return this.request(`/domains/${domain}/records/${name}/${type}`, {
      method: 'DELETE',
    });
  }
}
```

## よく使うパターン

### Aレコードの設定（Webサーバー）

```typescript
// setup-web-server.ts
import { GandiDNS } from './gandi-dns';

const dns = new GandiDNS();
const domain = 'example.com';
const serverIP = '203.0.113.1';

// ルートドメイン
await dns.createRecord(domain, {
  rrset_name: '@',
  rrset_type: 'A',
  rrset_values: [serverIP],
  rrset_ttl: 300,
});

// wwwサブドメイン
await dns.createRecord(domain, {
  rrset_name: 'www',
  rrset_type: 'A',
  rrset_values: [serverIP],
  rrset_ttl: 300,
});

console.log('✅ Webサーバー設定完了');
```

### MXレコードの設定（メールサーバー）

```typescript
// setup-mail-server.ts
import { GandiDNS } from './gandi-dns';

const dns = new GandiDNS();
const domain = 'example.com';

await dns.createRecord(domain, {
  rrset_name: '@',
  rrset_type: 'MX',
  rrset_values: [
    '10 mail1.example.com.',
    '20 mail2.example.com.',
  ],
  rrset_ttl: 300,
});

console.log('✅ メールサーバー設定完了');
```

### TXTレコードの設定（SPF, DKIM等）

```typescript
// setup-email-verification.ts
import { GandiDNS } from './gandi-dns';

const dns = new GandiDNS();
const domain = 'example.com';

// SPFレコード
await dns.createRecord(domain, {
  rrset_name: '@',
  rrset_type: 'TXT',
  rrset_values: ['v=spf1 include:_spf.google.com ~all'],
  rrset_ttl: 300,
});

// DKIM レコード
await dns.createRecord(domain, {
  rrset_name: 'default._domainkey',
  rrset_type: 'TXT',
  rrset_values: ['v=DKIM1; k=rsa; p=MIGfMA0GCSqGSIb3DQEBAQUAA...'],
  rrset_ttl: 300,
});

// DMARC レコード
await dns.createRecord(domain, {
  rrset_name: '_dmarc',
  rrset_type: 'TXT',
  rrset_values: ['v=DMARC1; p=quarantine; rua=mailto:dmarc@example.com'],
  rrset_ttl: 300,
});

console.log('✅ メール認証設定完了');
```

### CNAMEレコードの設定

```typescript
// setup-cname.ts
import { GandiDNS } from './gandi-dns';

const dns = new GandiDNS();
const domain = 'example.com';

// GitHub Pages
await dns.createRecord(domain, {
  rrset_name: 'blog',
  rrset_type: 'CNAME',
  rrset_values: ['username.github.io.'],
  rrset_ttl: 300,
});

// Vercel
await dns.createRecord(domain, {
  rrset_name: 'docs',
  rrset_type: 'CNAME',
  rrset_values: ['cname.vercel-dns.com.'],
  rrset_ttl: 300,
});

console.log('✅ CNAME設定完了');
```

## ダイナミックDNS

自宅サーバーなど、IPアドレスが動的に変わる環境で使用。

```typescript
// dynamic-dns.ts
import { GandiDNS } from './gandi-dns';

async function getCurrentIP(): Promise<string> {
  const response = await fetch('https://api.ipify.org?format=json');
  const data = await response.json();
  return data.ip;
}

async function updateDynamicDNS(
  domain: string,
  recordName: string,
  checkInterval = 300000 // 5分
) {
  const dns = new GandiDNS();
  let lastIP: string | null = null;

  console.log(`🔄 ダイナミックDNS開始: ${recordName}.${domain}`);

  while (true) {
    try {
      const currentIP = await getCurrentIP();

      if (currentIP !== lastIP) {
        console.log(`🔀 IP変更検出: ${lastIP} -> ${currentIP}`);
        
        await dns.updateRecord(domain, recordName, 'A', {
          rrset_values: [currentIP],
          rrset_ttl: 300,
        });

        console.log(`✅ DNS更新完了: ${recordName}.${domain} -> ${currentIP}`);
        lastIP = currentIP;
      } else {
        console.log(`✓ IP変更なし: ${currentIP}`);
      }

      await Bun.sleep(checkInterval);
    } catch (error) {
      console.error('❌ エラー:', error);
      await Bun.sleep(60000); // エラー時は1分待機
    }
  }
}

// 実行
await updateDynamicDNS('example.com', 'home', 300000);
```

## 一括操作

### 複数レコードの作成

```typescript
// bulk-create-records.ts
import { GandiDNS } from './gandi-dns';
import type { CreateDNSRecordRequest } from './types/dns';

const dns = new GandiDNS();
const domain = 'example.com';

const records: CreateDNSRecordRequest[] = [
  {
    rrset_name: '@',
    rrset_type: 'A',
    rrset_values: ['203.0.113.1'],
    rrset_ttl: 300,
  },
  {
    rrset_name: 'www',
    rrset_type: 'A',
    rrset_values: ['203.0.113.1'],
    rrset_ttl: 300,
  },
  {
    rrset_name: 'api',
    rrset_type: 'A',
    rrset_values: ['203.0.113.10'],
    rrset_ttl: 300,
  },
  {
    rrset_name: '@',
    rrset_type: 'MX',
    rrset_values: ['10 spool.mail.gandi.net.', '50 fb.mail.gandi.net.'],
    rrset_ttl: 10800,
  },
];

for (const record of records) {
  try {
    await dns.createRecord(domain, record);
    console.log(`✅ ${record.rrset_name} ${record.rrset_type}`);
  } catch (error) {
    console.error(`❌ ${record.rrset_name} ${record.rrset_type}:`, error);
  }
  
  // レート制限を避けるため少し待つ
  await Bun.sleep(500);
}

console.log('✅ 一括作成完了');
```

### レコード一覧の表示

```typescript
// list-all-records.ts
import { GandiDNS } from './gandi-dns';

const dns = new GandiDNS();
const domain = 'example.com';

const records = await dns.listRecords(domain);

console.log(`\n📋 ${domain} の DNS レコード一覧:\n`);

for (const record of records) {
  const name = record.rrset_name === '@' ? domain : `${record.rrset_name}.${domain}`;
  const values = record.rrset_values.join(', ');
  console.log(`${name.padEnd(30)} ${record.rrset_type.padEnd(8)} TTL:${record.rrset_ttl.toString().padEnd(6)} ${values}`);
}
```

## バックアップと復元

```typescript
// dns-backup.ts
import { GandiDNS } from './gandi-dns';
import { writeFile, file } from 'bun';

const dns = new GandiDNS();

// バックアップ
async function backupDNS(domain: string): Promise<string> {
  const records = await dns.listRecords(domain);
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const filename = `dns-backup-${domain}-${timestamp}.json`;
  
  await writeFile(filename, JSON.stringify(records, null, 2));
  console.log(`✅ バックアップ完了: ${filename}`);
  
  return filename;
}

// 復元
async function restoreDNS(domain: string, backupFile: string): Promise<void> {
  const fileContent = await file(backupFile).text();
  const records = JSON.parse(fileContent);
  
  for (const record of records) {
    try {
      await dns.createRecord(domain, {
        rrset_name: record.rrset_name,
        rrset_type: record.rrset_type,
        rrset_values: record.rrset_values,
        rrset_ttl: record.rrset_ttl,
      });
      console.log(`✅ 復元: ${record.rrset_name} ${record.rrset_type}`);
    } catch (error) {
      console.error(`⚠️  スキップ: ${record.rrset_name}`, error);
    }
    
    await Bun.sleep(500);
  }
  
  console.log('✅ 復元完了');
}

// 使用例
const backupFile = await backupDNS('example.com');
// await restoreDNS('example.com', backupFile);
```

## トラブルシューティング

### DNS変更が反映されない

```typescript
// dns-propagation-check.ts
async function checkDNSPropagation(domain: string, recordName: string): Promise<void> {
  const fqdn = recordName === '@' ? domain : `${recordName}.${domain}`;
  
  console.log(`🔍 DNS伝播チェック: ${fqdn}`);
  console.log('📌 チェックサイト: https://dnschecker.org/');
  console.log('⏱️  伝播には最大48時間かかる場合があります');
  
  // 実際のDNS解決をチェック
  const dnsResolver = Bun.spawn(['dig', '+short', fqdn]);
  const output = await new Response(dnsResolver.stdout).text();
  
  console.log(`\n現在の解決結果:\n${output || '（レコードなし）'}`);
}

await checkDNSPropagation('example.com', 'www');
```

### レコード作成エラー

```typescript
// validate-record.ts
import type { CreateDNSRecordRequest } from './types/dns';

function validateRecord(record: CreateDNSRecordRequest): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  // CNAME の場合、末尾にドットが必要
  if (record.rrset_type === 'CNAME') {
    for (const value of record.rrset_values) {
      if (!value.endsWith('.')) {
        errors.push(`CNAME値には末尾にドット（.）が必要です: ${value}`);
      }
    }
  }

  // MX の場合、優先度が必要
  if (record.rrset_type === 'MX') {
    for (const value of record.rrset_values) {
      if (!/^\d+\s+/.test(value)) {
        errors.push(`MX値には優先度が必要です: ${value}`);
      }
    }
  }

  // A レコードはIPv4アドレス
  if (record.rrset_type === 'A') {
    const ipv4Regex = /^(\d{1,3}\.){3}\d{1,3}$/;
    for (const value of record.rrset_values) {
      if (!ipv4Regex.test(value)) {
        errors.push(`無効なIPv4アドレス: ${value}`);
      }
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

// 使用例
const record: CreateDNSRecordRequest = {
  rrset_name: 'blog',
  rrset_type: 'CNAME',
  rrset_values: ['username.github.io'],  // エラー: 末尾に.がない
  rrset_ttl: 300,
};

const validation = validateRecord(record);
if (!validation.valid) {
  console.error('❌ バリデーションエラー:');
  validation.errors.forEach(err => console.error(`  - ${err}`));
}
```

## 公式ドキュメント

- LiveDNS API: https://api.gandi.net/docs/livedns/
- DNS レコードタイプ: https://api.gandi.net/docs/livedns/#record-types

---

次のステップ: [ドメイン管理](./domain_management.md)
