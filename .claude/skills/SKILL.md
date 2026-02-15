---
title: SurrealDB Skills
description: SurrealDBを効率的に使うための実践的スキルセット
version: 1.0.0
author: Vantage Hub Contributors
created_at: 2025-11-02
updated_at: 2025-11-02
tags:
  - surrealdb
  - database
  - nosql
  - graph-database
  - surrealql
  - rust-sdk
  - typescript-sdk
categories:
  - skill
  - database
  - backend
---

# SurrealDB スキル

SurrealDBを効率的に使うための実践的スキルセット。公式ドキュメントへのリンクを中心に、必要最小限の情報をまとめています。

## 📚 スキル構成

### コアリファレンス
- **reference/quick_start.md** - クイックスタート（インストール、起動、基本操作）
- **reference/surrealql_essentials.md** - SurrealQL必須パターン集
- **reference/type_system.md** - 型システム完全ガイド（Thing, RecordId, Reference等）
- **reference/rust_sdk.md** - Rust SDK実践ガイド
- **reference/typescript_sdk.md** - TypeScript SDK実践ガイド
- **reference/schema_management.md** - スキーママイグレーション（OVERWRITE方式）
- **reference/cli_reference.md** - CLIコマンドリファレンス
- **reference/mcp_integration.md** - MCP統合ガイド

### パターン集
- **patterns/common_queries.md** - よく使うクエリパターン
- **patterns/schema_patterns.md** - スキーマ設計パターン
- **patterns/error_handling.md** - エラーハンドリング

## 🎯 使い方

### 基本的な流れ
1. **quick_start.md** でセットアップ
2. **surrealql_essentials.md** で基本構文を学ぶ
3. 使用言語のSDKガイド（rust_sdk.md または typescript_sdk.md）を参照
4. **patterns/** で実践パターンを学ぶ

### Claude Codeでの使用例
```
@surrealdb ユーザーとポストのリレーションを作成したい
@surrealdb Rust SDKでの接続方法を教えて
@surrealdb TypeScriptでリアルタイムクエリを実装したい
```

## 🔗 公式リソース

- **公式ドキュメント**: https://surrealdb.com/docs
- **GitHub**: https://github.com/surrealdb/surrealdb
- **Discord**: https://discord.gg/surrealdb

## 📝 設計思想

このスキルは以下の原則で設計されています：

1. **公式ドキュメント優先**: 詳細は公式ドキュメントへのリンクで対応
2. **実践重視**: よく使うパターンとコード例を中心に
3. **保守性重視**: SurrealDBのバージョンアップに追随しやすい構成
4. **最小限の情報**: 必要十分な内容に絞る

## 📌 更新方針

- バージョン固有の詳細は記載しない
- APIの詳細は公式ドキュメントへのリンクで対応
- よく使うパターンのみを抽出して記載
- 四半期に1回程度の更新を推奨
