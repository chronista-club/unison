# よく使うパターン集（TypeScript + Bun）

実践的なGandi API使用パターンをまとめました。

## 1. 新規ドメインのセットアップ

```typescript
// setup-domain.ts
import { GandiDNS } from './gandi-dns';
import { GandiEmail } from './gandi-email';

const domain = 'example.com';
const serverIP = '203.0.113.1';

const dns = new GandiDNS();
const email = new GandiEmail();

// 1. Webサーバー設定
await dns.createRecord(domain, {
  rrset_name: '@',
  rrset_type: 'A',
  rrset_values: [serverIP],
  rrset_ttl: 300,
});

await dns.createRecord(domain, {
  rrset_name: 'www',
  rrset_type: 'A',
  rrset_values: [serverIP],
  rrset_ttl: 300,
});

// 2. メールサーバー設定
await dns.createRecord(domain, {
  rrset_name: '@',
  rrset_type: 'MX',
  rrset_values: ['10 spool.mail.gandi.net.', '50 fb.mail.gandi.net.'],
  rrset_ttl: 10800,
});

// 3. SPFレコード
await dns.createRecord(domain, {
  rrset_name: '@',
  rrset_type: 'TXT',
  rrset_values: ['v=spf1 include:_mailcust.gandi.net ~all'],
  rrset_ttl: 10800,
});

// 4. メールボックス作成
await email.createMailbox(domain, 'contact', process.env.CONTACT_PASSWORD!);
await email.createMailbox(domain, 'info', process.env.INFO_PASSWORD!);

// 5. エイリアス設定
await email.createForward(domain, 'admin@example.com', 'contact@example.com');
await email.createForward(domain, 'support@example.com', 'contact@example.com');

console.log(`✅ ${domain} のセットアップ完了`);
```

## 2. ダイナミックDNS

```typescript
// ddns.ts
import { GandiDNS } from './gandi-dns';

async function getCurrentIP(): Promise<string> {
  const response = await fetch('https://api.ipify.org?format=json');
  const data = await response.json();
  return data.ip;
}

async function runDynamicDNS(domain: string, recordName: string, intervalMs = 300000) {
  const dns = new GandiDNS();
  let lastIP: string | null = null;

  console.log(`🔄 ダイナミックDNS開始: ${recordName}.${domain}`);

  while (true) {
    try {
      const currentIP = await getCurrentIP();

      if (currentIP !== lastIP) {
        console.log(`🔀 IP変更: ${lastIP} -> ${currentIP}`);
        await dns.updateRecord(domain, recordName, 'A', {
          rrset_values: [currentIP],
          rrset_ttl: 300,
        });
        console.log(`✅ 更新完了`);
        lastIP = currentIP;
      }

      await Bun.sleep(intervalMs);
    } catch (error) {
      console.error('❌ エラー:', error);
      await Bun.sleep(60000);
    }
  }
}

await runDynamicDNS('example.com', 'home');
```

## 3. バックアップと復元

```typescript
// backup-restore.ts
import { GandiDNS } from './gandi-dns';
import { writeFile, file } from 'bun';

const dns = new GandiDNS();

// バックアップ
async function backup(domain: string): Promise<string> {
  const records = await dns.listRecords(domain);
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const filename = `backup-${domain}-${timestamp}.json`;
  
  await writeFile(filename, JSON.stringify(records, null, 2));
  console.log(`✅ バックアップ: ${filename}`);
  
  return filename;
}

// 復元
async function restore(domain: string, backupFile: string) {
  const content = await file(backupFile).text();
  const records = JSON.parse(content);
  
  for (const record of records) {
    try {
      await dns.createRecord(domain, record);
      console.log(`✅ ${record.rrset_name} ${record.rrset_type}`);
    } catch (error) {
      console.error(`⚠️  ${record.rrset_name}:`, error);
    }
    await Bun.sleep(500);
  }
}

// 実行
const backupFile = await backup('example.com');
// await restore('example.com', backupFile);
```

## 4. 一括DNS更新

```typescript
// bulk-update.ts
import { GandiDNS } from './gandi-dns';

const dns = new GandiDNS();
const domain = 'example.com';

const updates = [
  { name: 'www', type: 'A', values: ['203.0.113.1'], ttl: 300 },
  { name: 'api', type: 'A', values: ['203.0.113.10'], ttl: 300 },
  { name: 'blog', type: 'CNAME', values: ['username.github.io.'], ttl: 300 },
];

for (const update of updates) {
  try {
    await dns.updateRecord(domain, update.name, update.type, {
      rrset_values: update.values,
      rrset_ttl: update.ttl,
    });
    console.log(`✅ ${update.name} ${update.type}`);
  } catch (error) {
    console.error(`❌ ${update.name}:`, error);
  }
  await Bun.sleep(500);
}
```

## 5. 環境別設定管理

```typescript
// config.ts
interface Config {
  domain: string;
  dns: Array<{
    name: string;
    type: string;
    values: string[];
    ttl?: number;
  }>;
  email?: {
    mailboxes?: Array<{ login: string; password: string }>;
    forwards?: Array<{ source: string; destination: string }>;
  };
}

const productionConfig: Config = {
  domain: 'example.com',
  dns: [
    { name: '@', type: 'A', values: ['203.0.113.1'], ttl: 300 },
    { name: 'www', type: 'A', values: ['203.0.113.1'], ttl: 300 },
    { name: 'api', type: 'A', values: ['203.0.113.10'], ttl: 300 },
  ],
  email: {
    mailboxes: [
      { login: 'contact', password: process.env.CONTACT_PASSWORD! },
    ],
    forwards: [
      { source: 'info@example.com', destination: 'contact@example.com' },
    ],
  },
};

async function applyConfig(config: Config) {
  const dns = new GandiDNS();
  const email = new GandiEmail();

  // DNS設定
  for (const record of config.dns) {
    await dns.createRecord(config.domain, {
      rrset_name: record.name,
      rrset_type: record.type,
      rrset_values: record.values,
      rrset_ttl: record.ttl,
    });
    console.log(`✅ DNS: ${record.name} ${record.type}`);
    await Bun.sleep(500);
  }

  // メール設定
  if (config.email?.mailboxes) {
    for (const mb of config.email.mailboxes) {
      await email.createMailbox(config.domain, mb.login, mb.password);
      console.log(`✅ Mailbox: ${mb.login}`);
      await Bun.sleep(500);
    }
  }
}

await applyConfig(productionConfig);
```

## 6. エラーハンドリングとリトライ

```typescript
// retry-helper.ts
async function retryWithBackoff<T>(
  fn: () => Promise<T>,
  maxRetries = 3,
  baseDelay = 1000
): Promise<T> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      
      const delay = baseDelay * Math.pow(2, i);
      console.log(`⏳ リトライ ${i + 1}/${maxRetries} (${delay}ms後)`);
      await Bun.sleep(delay);
    }
  }
  throw new Error('Max retries exceeded');
}

// 使用例
const dns = new GandiDNS();
const records = await retryWithBackoff(() => dns.listRecords('example.com'));
```

## 7. DNS変更監視とSlack通知

```typescript
// monitor-and-notify.ts
import { GandiDNS } from './gandi-dns';

async function notifySlack(webhookUrl: string, message: string) {
  await fetch(webhookUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ text: message }),
  });
}

async function monitorDNS(domain: string, webhookUrl: string, intervalMs = 3600000) {
  const dns = new GandiDNS();
  let lastSnapshot: any = null;

  while (true) {
    const currentSnapshot = await dns.listRecords(domain);
    
    if (lastSnapshot && JSON.stringify(currentSnapshot) !== JSON.stringify(lastSnapshot)) {
      await notifySlack(webhookUrl, `🔔 DNS変更検出: ${domain}`);
    }
    
    lastSnapshot = currentSnapshot;
    await Bun.sleep(intervalMs);
  }
}

// await monitorDNS('example.com', process.env.SLACK_WEBHOOK_URL!);
```

## ベストプラクティス

### 1. 型安全な実装

```typescript
interface DNSRecordConfig {
  name: string;
  type: 'A' | 'AAAA' | 'CNAME' | 'MX' | 'TXT';
  values: string[];
  ttl?: number;
}

function validateRecord(record: DNSRecordConfig): boolean {
  // バリデーションロジック
  if (record.type === 'CNAME' && !record.values[0].endsWith('.')) {
    throw new Error('CNAME must end with a dot');
  }
  return true;
}
```

### 2. 環境変数の管理

```bash
# .env
GANDI_API_KEY=your-api-key
CONTACT_PASSWORD=secure-password
SLACK_WEBHOOK_URL=https://hooks.slack.com/...
```

```typescript
// .env.example をコミット
// .env は .gitignore に追加
```

### 3. ログ記録

```typescript
// logger.ts
const log = {
  info: (msg: string) => console.log(`[INFO] ${new Date().toISOString()} ${msg}`),
  error: (msg: string) => console.error(`[ERROR] ${new Date().toISOString()} ${msg}`),
  success: (msg: string) => console.log(`[SUCCESS] ${new Date().toISOString()} ${msg}`),
};

export default log;
```

---

これらのパターンを組み合わせることで、効率的なドメイン・DNS・メール管理が可能になります。
