# Qdrant - ベクトル検索スキル

Vantageプロジェクトで使用するQdrantベクトルデータベースの運用・開発スキルセット。

## 概要

Qdrantは、高性能なベクトル類似検索エンジンで、Vantage Memory Systemのセマンティック検索に使用します。

## スキル構成

- **[SKILL.md](./SKILL.md)** - クイックスタートガイド
- **[reference/](./reference/)** - 詳細リファレンス
  - API仕様
  - コレクション管理
  - クエリパターン
- **[examples/](./examples/)** - 実践例

## 主な用途

### Vantageでの役割

- AIエージェントの会話履歴のセマンティック検索
- ユーザー行動の類似パターン発見
- システムイベントの意味的分析
- コンテキストの長期保存と類似検索

### 技術スタック

- **Qdrant**: v1.x
- **Rustクライアント**: `qdrant-client`
- **埋め込み**: OpenAI Embeddings (text-embedding-3-small)
- **次元数**: 1536

## クイックスタート

### 1. Qdrantの起動

```bash
# Vantage CLIで自動起動（推奨）
vantage qdrant start

# または手動でDocker起動
docker run -p 6333:6333 -p 6334:6334 \
    -v ./.qdrant_storage:/qdrant/storage \
    qdrant/qdrant
```

### 2. 接続確認

```bash
curl http://localhost:6333/health
```

### 3. Rustでの基本操作

```rust
use qdrant_client::prelude::*;

// 接続
let client = QdrantClient::from_url("http://localhost:6334").build()?;

// コレクション作成
client.create_collection(&CreateCollection {
    collection_name: "vantage_memory".to_string(),
    vectors_config: Some(VectorsConfig {
        config: Some(Config::Params(VectorParams {
            size: 1536,
            distance: Distance::Cosine.into(),
        })),
    }),
    ..Default::default()
}).await?;

// ベクトル追加
let points = vec![PointStruct::new(
    uuid::Uuid::new_v4().to_string(),
    vec![0.1; 1536],  // 埋め込みベクトル
    [("event_type", "agent".into())].into(),
)];

client.upsert_points("vantage_memory", points, None).await?;

// 検索
let search_result = client.search_points(&SearchPoints {
    collection_name: "vantage_memory".to_string(),
    vector: vec![0.1; 1536],
    limit: 10,
    with_payload: Some(true.into()),
    ..Default::default()
}).await?;
```

## 関連ドキュメント

- [Memory System仕様](../../../spec/memory-system.md)
- [ADR-001: ハイブリッドストレージアーキテクチャ](../../../spec/adr/001-hybrid-storage-architecture.md)
- [ADR-002: 埋め込みプロバイダー](../../../spec/adr/002-embedding-provider.md)

## 参考リンク

- [Qdrant公式ドキュメント](https://qdrant.tech/documentation/)
- [Rust Client](https://github.com/qdrant/rust-client)
- [APIリファレンス](https://qdrant.github.io/qdrant/redoc/index.html)
