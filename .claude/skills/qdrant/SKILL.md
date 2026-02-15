# Qdrant スキル - クイックスタート

## 起動と停止

### Vantage CLIで管理（推奨）

```bash
# Qdrant起動（自動でDockerコンテナを起動）
vantage qdrant start

# 状態確認
vantage qdrant status

# 停止
vantage qdrant stop

# 再起動
vantage qdrant restart
```

### 手動起動

```bash
# Docker起動
docker run -d --name vantage-qdrant \
    -p 6333:6333 -p 6334:6334 \
    -v ./.qdrant_storage:/qdrant/storage \
    qdrant/qdrant

# 停止
docker stop vantage-qdrant

# 削除
docker rm vantage-qdrant
```

## コレクション管理

### コレクション作成

```rust
use qdrant_client::prelude::*;
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::{CreateCollection, VectorParams, VectorsConfig, Distance};

let client = QdrantClient::from_url("http://localhost:6334").build()?;

client.create_collection(&CreateCollection {
    collection_name: "vantage_memory".to_string(),
    vectors_config: Some(VectorsConfig {
        config: Some(Config::Params(VectorParams {
            size: 1536,  // OpenAI embedding dimension
            distance: Distance::Cosine.into(),
        })),
    }),
    ..Default::default()
}).await?;
```

### コレクション一覧

```rust
let collections = client.list_collections().await?;
for collection in collections.collections {
    println!("Collection: {}", collection.name);
}
```

### コレクション情報

```rust
let info = client.collection_info("vantage_memory").await?;
println!("Points count: {}", info.result.unwrap().points_count);
```

### コレクション削除

```rust
client.delete_collection("vantage_memory").await?;
```

## ポイント操作

### ポイント追加（単一）

```rust
use qdrant_client::qdrant::{PointStruct, Value};

let point = PointStruct::new(
    uuid::Uuid::new_v4().to_string(),  // ID
    vec![0.1; 1536],                    // 埋め込みベクトル
    [
        ("event_type", "agent".into()),
        ("agent_id", "claude".into()),
        ("timestamp", Value::from(1699000000)),
        ("content", "ユーザーがRust開発について質問".into()),
    ].into(),
);

client.upsert_points("vantage_memory", vec![point], None).await?;
```

### ポイント追加（バッチ）

```rust
let points: Vec<PointStruct> = events.into_iter()
    .map(|event| PointStruct::new(
        event.id.to_string(),
        event.embedding,
        [
            ("event_type", event.event_type.into()),
            ("timestamp", Value::from(event.timestamp.timestamp())),
        ].into(),
    ))
    .collect();

client.upsert_points_batch("vantage_memory", points, None, 100).await?;
```

### ポイント取得

```rust
use qdrant_client::qdrant::GetPoints;

let result = client.get_points(
    "vantage_memory",
    &[event_id.to_string()],
    Some(true),  // with_payload
    Some(true),  // with_vectors
    None,
).await?;
```

### ポイント削除

```rust
use qdrant_client::qdrant::PointsSelector;

// IDで削除
client.delete_points(
    "vantage_memory",
    &[event_id.to_string()].into(),
    None,
).await?;

// フィルター条件で削除
use qdrant_client::qdrant::{Filter, Condition, FieldCondition, Match};

client.delete_points(
    "vantage_memory",
    &PointsSelector {
        points_selector_one_of: Some(
            qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Filter(
                Filter {
                    must: vec![Condition {
                        condition_one_of: Some(
                            qdrant_client::qdrant::condition::ConditionOneOf::Field(
                                FieldCondition {
                                    key: "event_type".to_string(),
                                    r#match: Some(Match {
                                        match_value: Some(
                                            qdrant_client::qdrant::r#match::MatchValue::Keyword("system".to_string())
                                        ),
                                    }),
                                    ..Default::default()
                                }
                            )
                        ),
                    }],
                    ..Default::default()
                }
            )
        ),
    },
    None,
).await?;
```

## 検索

### 基本的なベクトル検索

```rust
use qdrant_client::qdrant::SearchPoints;

let search_result = client.search_points(&SearchPoints {
    collection_name: "vantage_memory".to_string(),
    vector: query_embedding,  // Vec<f32>
    limit: 10,
    with_payload: Some(true.into()),
    ..Default::default()
}).await?;

for point in search_result.result {
    println!("Score: {}, ID: {}", point.score, point.id.unwrap());
    println!("Payload: {:?}", point.payload);
}
```

### フィルター付き検索

```rust
use qdrant_client::qdrant::{Filter, Condition, FieldCondition, Match};

let filter = Filter {
    must: vec![
        Condition {
            condition_one_of: Some(
                qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "event_type".to_string(),
                        r#match: Some(Match {
                            match_value: Some(
                                qdrant_client::qdrant::r#match::MatchValue::Keyword("agent".to_string())
                            ),
                        }),
                        ..Default::default()
                    }
                )
            ),
        },
    ],
    ..Default::default()
};

let search_result = client.search_points(&SearchPoints {
    collection_name: "vantage_memory".to_string(),
    vector: query_embedding,
    filter: Some(filter),
    limit: 10,
    with_payload: Some(true.into()),
    ..Default::default()
}).await?;
```

### 範囲フィルター

```rust
use qdrant_client::qdrant::{Range, FieldCondition};

// タイムスタンプ範囲
let filter = Filter {
    must: vec![
        Condition {
            condition_one_of: Some(
                qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "timestamp".to_string(),
                        range: Some(Range {
                            gte: Some(start_timestamp as f64),
                            lte: Some(end_timestamp as f64),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }
                )
            ),
        },
    ],
    ..Default::default()
};
```

### スコア閾値

```rust
let search_result = client.search_points(&SearchPoints {
    collection_name: "vantage_memory".to_string(),
    vector: query_embedding,
    limit: 10,
    score_threshold: Some(0.8),  // 0.8以上のスコアのみ
    with_payload: Some(true.into()),
    ..Default::default()
}).await?;
```

## よくあるパターン

### Vantageメモリイベントの保存

```rust
use vantage_memory::{MemoryEvent, EventType};

async fn store_memory_event(
    client: &QdrantClient,
    event: &MemoryEvent,
    embedding: Vec<f32>,
) -> Result<()> {
    let point = PointStruct::new(
        event.id.to_string(),
        embedding,
        [
            ("event_type", event.event_type.to_string().into()),
            ("timestamp", Value::from(event.timestamp.timestamp())),
            ("session_id", event.metadata.session_id.clone()
                .unwrap_or_default().into()),
            ("content", event.content.clone().into()),
        ].into(),
    );
    
    client.upsert_points("vantage_memory", vec![point], None).await?;
    Ok(())
}
```

### セッションIDでフィルタリングした類似検索

```rust
async fn search_in_session(
    client: &QdrantClient,
    query_embedding: Vec<f32>,
    session_id: &str,
    limit: u64,
) -> Result<Vec<ScoredPoint>> {
    let filter = Filter {
        must: vec![
            Condition {
                condition_one_of: Some(
                    qdrant_client::qdrant::condition::ConditionOneOf::Field(
                        FieldCondition {
                            key: "session_id".to_string(),
                            r#match: Some(Match {
                                match_value: Some(
                                    qdrant_client::qdrant::r#match::MatchValue::Keyword(
                                        session_id.to_string()
                                    )
                                ),
                            }),
                            ..Default::default()
                        }
                    )
                ),
            },
        ],
        ..Default::default()
    };
    
    let result = client.search_points(&SearchPoints {
        collection_name: "vantage_memory".to_string(),
        vector: query_embedding,
        filter: Some(filter),
        limit,
        with_payload: Some(true.into()),
        ..Default::default()
    }).await?;
    
    Ok(result.result)
}
```

## トラブルシューティング

### 接続できない

```bash
# Qdrantが起動しているか確認
docker ps | grep qdrant

# ポートが開いているか確認
curl http://localhost:6333/health

# ログ確認
docker logs vantage-qdrant
```

### パフォーマンスが悪い

- インデックスの設定を確認
- バッチサイズを調整（推奨: 100-1000）
- メモリ設定を確認

### データが見つからない

```rust
// コレクション内のポイント数確認
let info = client.collection_info("vantage_memory").await?;
println!("Points: {}", info.result.unwrap().points_count);

// スクロールで全ポイント取得
use qdrant_client::qdrant::ScrollPoints;

let scroll_result = client.scroll(&ScrollPoints {
    collection_name: "vantage_memory".to_string(),
    limit: Some(10),
    with_payload: Some(true.into()),
    ..Default::default()
}).await?;
```

## 参考資料

- [Qdrant公式ドキュメント](https://qdrant.tech/documentation/)
- [Rust Client GitHub](https://github.com/qdrant/rust-client)
- [Vantage Memory System仕様](../../../spec/memory-system.md)
