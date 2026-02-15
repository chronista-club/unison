# SSL証明書管理

Gandi APIを使用したSSL/TLS証明書の管理方法。

## エンドポイント

```
ベースURL: https://api.gandi.net/v5/certificate
```

## 証明書一覧取得

```bash
curl -H "Authorization: Bearer $GANDI_API_KEY" \
     https://api.gandi.net/v5/certificate/certificates
```

## 証明書詳細取得

```bash
curl -H "Authorization: Bearer $GANDI_API_KEY" \
     https://api.gandi.net/v5/certificate/certificates/{certificate_id}
```

## Let's Encrypt証明書の自動取得

GandiはLet's Encrypt証明書の自動取得をサポートしています。

### 前提条件
- ドメインがGandiで管理されている
- DNSがGandi LiveDNSを使用している

### 自動証明書の有効化

Webインターフェースまたは設定で有効化可能です。API経由での直接発行は、証明書管理APIを通じて行います。

## 公式ドキュメント

- Certificate API: https://api.gandi.net/docs/certificates/
- Let's Encrypt: https://docs.gandi.net/en/ssl/

---

次のステップ: [メール管理](./email_management.md)
