---
title: Gandi Skills
description: Gandi APIを効率的に使うための実践的スキルセット
version: 1.0.0
author: Vantage Hub Contributors
created_at: 2025-11-02
updated_at: 2025-11-02
tags:
  - gandi
  - api
  - domain-management
  - dns
  - ssl-certificates
  - email-management
categories:
  - skill
  - infrastructure
  - api-integration
---

# Gandi スキル

Gandi APIを効率的に使うための実践的スキルセット。ドメイン管理、DNS設定、SSL証明書管理などをAPI経由で操作するためのガイドです。

## 📚 スキル構成

### コアリファレンス
- **reference/api_basics.md** - API認証と基本的な使い方
- **reference/domain_management.md** - ドメイン管理API
- **reference/dns_records.md** - DNSレコード操作
- **reference/ssl_certificates.md** - SSL証明書管理
- **reference/email_management.md** - メールボックス管理
- **reference/common_patterns.md** - よく使うパターン集

## 🎯 使い方

### 基本的な流れ
1. **api_basics.md** でAPI認証とセットアップ
2. 目的に応じて各リファレンスを参照
3. **common_patterns.md** で実践的な例を学ぶ

### Claude Codeでの使用例
```
@gandi ドメインのDNSレコードを追加したい
@gandi SSL証明書の取得方法を教えて
@gandi メールボックスを作成したい
```

## 🔗 公式リソース

- **API ドキュメント**: https://api.gandi.net/docs/
- **公式サイト**: https://www.gandi.net/ja/solutions/api
- **サポート**: https://docs.gandi.net/

## 📝 設計思想

このスキルは以下の原則で設計されています：

1. **公式ドキュメント優先**: 詳細は公式ドキュメントへのリンクで対応
2. **実践重視**: よく使うAPIエンドポイントとコード例を中心に
3. **セキュリティ重視**: API キーの安全な管理方法を明記
4. **最小限の情報**: 必要十分な内容に絞る

## 🔑 前提条件

- Gandiアカウント
- APIキー（Personal Access Token）の取得
- 基本的なREST APIの知識

## 📖 主な機能

### ドメイン管理
- ドメイン一覧取得
- ドメイン詳細情報
- ドメイン更新・移管

### DNS管理
- DNSレコードのCRUD操作
- ゾーンファイル管理
- ダイナミックDNS

### SSL証明書
- 証明書の発行
- 証明書の更新
- Let's Encrypt統合

### メール管理
- メールボックス作成・削除
- エイリアス設定
- 転送ルール設定

## 🚀 クイックスタート

### 1. APIキーの取得
```bash
# Gandiアカウントにログイン
# https://account.gandi.net/ja/users/security
# Personal Access Tokenを作成
```

### 2. 環境変数の設定
```bash
export GANDI_API_KEY="your-api-key-here"
```

### 3. APIテスト
```bash
# ドメイン一覧を取得
curl -H "Authorization: Bearer $GANDI_API_KEY" \
     https://api.gandi.net/v5/domain/domains
```

## 💡 ベストプラクティス

### API キーの管理
- 環境変数で管理する
- バージョン管理システムにコミットしない
- 定期的にローテーションする
- 必要最小限の権限のみ付与

### レート制限
- APIリクエストには制限がある
- バッチ処理では適切な間隔を空ける
- エラーハンドリングを実装

### エラー処理
- HTTPステータスコードを確認
- エラーレスポンスのメッセージを確認
- リトライロジックを実装

## 📚 参考情報

### Gandi APIの特徴
- RESTful API
- JSON形式のレスポンス
- Personal Access Token認証
- 包括的なドメイン・DNS・SSL管理

### サポートされる操作
- ドメイン登録・更新・移管
- DNS レコード管理（A, AAAA, CNAME, MX, TXT, SRV等）
- SSL/TLS 証明書管理
- メールボックス・転送設定
- Web転送設定

## 🔧 トラブルシューティング

### よくある問題

**認証エラー**
```
401 Unauthorized
```
→ APIキーが正しいか確認

**レート制限**
```
429 Too Many Requests
```
→ リクエスト間隔を空ける

**リソースが見つからない**
```
404 Not Found
```
→ ドメイン名やレコードIDを確認

## 📝 注意事項

- API操作は本番環境に直接影響します
- テスト用のドメインで動作確認することを推奨
- 重要な変更前にはバックアップを取得
- DNS変更は反映に時間がかかる場合があります（最大48時間）

## 🎓 学習リソース

1. まず **reference/api_basics.md** で認証方法を理解
2. **reference/dns_records.md** でDNS操作を学ぶ
3. **reference/common_patterns.md** で実践例を確認
4. 公式ドキュメントで詳細を確認

---

最終更新: 2025-11-02
