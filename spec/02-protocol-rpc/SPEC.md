# spec/02: Unison Protocol - RPC プロトコル仕様

## 概要

Unison ProtocolのRPC層は、KDLベースのスキーマ定義から型安全なクライアント・サーバーコードを自動生成する、型安全通信フレームワークです。

## 1. 設計思想

### 1.1 目標

- **型安全性**: コンパイル時・実行時の型チェック保証
- **開発者体験**: シンプルで直感的なAPI
- **多言語サポート**: Rust、TypeScript等への自動コード生成
- **リアルタイム通信**: 低レイテンシー双方向通信
- **拡張性**: 新しいメソッド、型、プロトコルの簡単な追加

### 1.2 設計原則

- **スキーマファースト**: プロトコル定義駆動開発
- **非同期優先**: async/awaitパターンを基盤
- **エラー耐性**: 包括的なエラーハンドリングと回復メカニズム
- **トランスポート非依存**: QUIC、WebSocket、TCP等に対応
- **バージョン互換性**: 前方・後方互換性サポート

## 2. プロトコル構造

### 2.1 階層構造

```
Protocol（プロトコル）
├── Metadata（メタデータ） (name, version, namespace, description)
├── Types（型） (カスタム型定義)
├── Messages（メッセージ） (構造化データ定義)
└── Services（サービス）
    └── Methods（メソッド）
```

### 2.2 プロトコル定義フォーマット（KDL）

```kdl
protocol "service-name" version="1.0.0" {
    namespace "com.example.service"
    description "サービス説明"
    
    // 型定義
    // メッセージ定義
    // サービス定義
}
```

## 3. コアメッセージ型

### 3.1 UnisonMessage

全てのRPC通信における標準メッセージ形式：

```rust
struct UnisonMessage {
    id: String,           // 一意メッセージ識別子
    method: String,       // RPCメソッド名
    payload: JsonValue,   // JSON形式のメソッドパラメータ
    timestamp: DateTime,  // メッセージ作成タイムスタンプ
    version: String,      // プロトコルバージョン（デフォルト: "1.0.0"）
}
```

### 3.2 UnisonResponse

標準レスポンス形式：

```rust
struct UnisonResponse {
    id: String,                    // 対応するリクエストメッセージID
    success: bool,                 // 操作成功インジケーター
    payload: Option<JsonValue>,    // JSON形式のレスポンスデータ
    error: Option<String>,         // 操作失敗時のエラーメッセージ
    timestamp: DateTime,           // レスポンス作成タイムスタンプ
    version: String,               // プロトコルバージョン
}
```

### 3.3 UnisonError

構造化されたエラー情報：

```rust
struct UnisonError {
    code: String,                  // エラーコード識別子
    message: String,               // 人間が読めるエラーメッセージ
    details: Option<JsonValue>,    // 追加のエラーコンテキスト
    timestamp: DateTime,           // エラー発生タイムスタンプ
}
```

## 4. スキーマ定義言語（KDL）

### 4.1 基本型

| 型 | 説明 | Rustマッピング | TypeScriptマッピング |
|------|-------------|--------------|---------------------|
| `string` | UTF-8テキスト | `String` | `string` |
| `number` | 数値 | `f64` | `number` |
| `bool` | 真偽値 | `bool` | `boolean` |
| `timestamp` | ISO-8601日時 | `DateTime<Utc>` | `string` |
| `json` | 任意のJSON | `serde_json::Value` | `any` |
| `array` | アイテムのリスト | `Vec<T>` | `T[]` |

### 4.2 フィールド修飾子

- `required=true`: フィールドが必須（デフォルト: false）
- `default=value`: オプションフィールドのデフォルト値
- `description="text"`: フィールドドキュメンテーション

### 4.3 スキーマ例

#### サービス定義例

```kdl
protocol "user-management" version="1.0.0" {
    namespace "com.example.users"
    description "ユーザー管理サービス"

    message "User" {
        description "ユーザーアカウント情報"
        field "id" type="string" required=true
        field "username" type="string" required=true
        field "email" type="string" required=true
        field "created_at" type="timestamp" required=true
        field "is_active" type="bool" required=false default=true
    }

    service "UserService" {
        description "ユーザーアカウント管理操作"

        method "create_user" {
            description "新しいユーザーアカウントを作成"
            request {
                field "username" type="string" required=true
                field "email" type="string" required=true
                field "password" type="string" required=true
            }
            response {
                field "user" type="User" required=true
                field "session_token" type="string" required=true
            }
        }

        method "get_user" {
            description "IDによってユーザー情報を取得"
            request {
                field "user_id" type="string" required=true
            }
            response {
                field "user" type="User" required=true
            }
        }
    }
}
```

#### チャネル定義例（creo_sync.kdl より抜粋）

```kdl
protocol "creo-sync" version="1.0.0" {
    namespace "club.chronista.sync"

    // Control Plane: クライアント起点の永続チャネル
    channel "control" from="client" lifetime="persistent" {
        send "Subscribe" {
            field "category" type="string"
            field "tags" type="string"
        }
        recv "Ack" {
            field "status" type="string"
            field "channel_ref" type="string"
        }
    }

    // Data Plane: サーバー起点のイベント配信チャネル
    channel "events" from="server" lifetime="persistent" {
        send "MemoryEvent" {
            field "event_type" type="string" required=#true
            field "memory_id" type="string" required=#true
            field "category" type="string"
            field "from" type="string"
            field "timestamp" type="string"
        }
    }

    // Data Plane: トランジェントなクエリチャネル（エラー型付き）
    channel "query" from="client" lifetime="transient" {
        send "Query" {
            field "method" type="string" required=#true
            field "params" type="json"
        }
        recv "Result" {
            field "data" type="json"
        }
        error "QueryError" {
            field "code" type="string"
            field "message" type="string"
        }
    }
}
```

### 4.4 Channel 定義構文

サービスメソッドに加えて、KDL スキーマでは `channel` キーワードによる型安全なストリームチャネルの定義をサポートする。

#### 構文

```kdl
channel "<name>" from="<direction>" lifetime="<lifetime>" {
    send "<MessageType>" {
        field "<name>" type="<type>" [required=#true]
    }
    recv "<MessageType>" {
        field "<name>" type="<type>" [required=#true]
    }
    error "<ErrorType>" {
        field "<name>" type="<type>"
    }
}
```

#### 属性

| 属性 | 値 | 説明 |
|------|-----|------|
| `from` | `"client"` / `"server"` / `"either"` | チャネルの方向性 |
| `lifetime` | `"persistent"` / `"transient"` | チャネルのライフタイム |

#### メッセージブロック

| ブロック | 説明 |
|---------|------|
| `send` | 送信メッセージ型の定義 |
| `recv` | 受信メッセージ型の定義（省略可: 一方向チャネル） |
| `error` | チャネル固有エラー型の定義（省略可） |

チャネル仕様の詳細は [spec/03: Stream Channel 仕様](../03-stream-channels/SPEC.md) を参照。

## 5. RPCメッセージフロー

### 5.1 接続確立

1. クライアントがサーバーへの接続を開始（QUIC、WebSocket等）
2. バージョンネゴシエーション用のオプションハンドシェイク交換

### 5.2 メソッド呼び出し

1. クライアントがメソッド名とパラメータと共に`UnisonMessage`を送信
2. サーバーがリクエストを処理し、`UnisonResponse`を送信
3. エラーは`success: false`の`UnisonResponse`として返される

### 5.3 エラーハンドリング

#### クライアントサイドエラー
- 接続失敗
- タイムアウトエラー
- シリアライゼーション/デシリアライゼーションエラー
- プロトコルバージョン不整合

#### サーバーサイドエラー
- メソッドが見つからない
- 無効なパラメータ
- 処理失敗
- リソース制限

#### エラーレスポンス形式

```json
{
  "id": "request-message-id",
  "success": false,
  "error": "メソッドが見つかりません: unknown_method",
  "timestamp": "2025-01-04T10:30:00Z",
  "version": "1.0.0"
}
```

## 6. コード生成

### 6.1 Rustコード生成

生成されるRustコードには以下が含まれます：

- **型定義**: Serde注釈付きの構造体
- **クライアントトレイト**: 各サービスメソッドの非同期メソッド
- **サーバートレイト**: メソッドのハンドラー登録
- **検証**: リクエスト/レスポンス検証ロジック

生成コードの例：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_is_active")]
    pub is_active: bool,
}

#[async_trait]
pub trait UserServiceClient {
    async fn create_user(&self, request: CreateUserRequest) 
        -> Result<CreateUserResponse, NetworkError>;
    async fn get_user(&self, user_id: String) 
        -> Result<User, NetworkError>;
}
```

### 6.2 Channel型 コード生成

KDL スキーマの `channel` 定義から、Rust の `QuicBackedChannel<S, R>` 型を自動生成する。

#### 生成対象

| KDL 定義 | 生成される Rust コード |
|----------|----------------------|
| `send` ブロック | 送信メッセージ構造体 |
| `recv` ブロック | 受信メッセージ構造体 |
| `error` ブロック | エラー列挙型 |
| `channel` 全体 | `QuicBackedChannel<Send, Recv>` の型エイリアス |

#### 生成コード例

creo_sync.kdl の `query` チャネルから生成されるコード:

```rust
// 送信メッセージ型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub method: String,
    pub params: Option<serde_json::Value>,
}

// 受信メッセージ型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub data: Option<serde_json::Value>,
}

// エラー型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryError {
    pub code: String,
    pub message: String,
}

// チャネル型エイリアス
pub type QueryChannel = QuicBackedChannel<Query, QueryResult>;
```

#### ConnectionBuilder パターン

複数チャネルの一括開設には `ConnectionBuilder` パターンを使用する:

```rust
// コード生成で自動生成される接続ビルダー
pub struct CreoSyncConnection {
    pub control: QuicBackedChannel<Subscribe, Ack>,
    pub events: QuicBackedChannel<MemoryEvent, ()>,
    pub query: QuicBackedChannel<Query, QueryResult>,
    pub messaging: QuicBackedChannel<CCMessage, CCMessage>,
    pub urgent: QuicBackedChannel<Alert, ()>,
}
```

### 6.3 TypeScriptコード生成（計画中）

- **インターフェース定義**: すべての型のTypeScriptインターフェース
- **クライアントクラス**: Promiseベースのクライアント実装
- **型ガード**: 実行時型検証
- **エラー型**: 構造化されたエラーハンドリング

## 7. セキュリティ

### 7.1 認証と認可

- プロトコルレベルの認証は未指定（トランスポートレイヤーの責任）
- サービスレベルの認可はカスタムハンドラーで実装
- セッション管理はアプリケーション固有トークンで実現

### 7.2 入力検証

- 必須フィールドの自動検証
- 全パラメータの型チェック
- カスタム検証はハンドラー実装で対応

### 7.3 トランスポートセキュリティ

- 本番使用にはTLS（QUIC、WSS）を推奨
- 証明書検証とピン留め
- 接続暗号化と完全性

## 8. パフォーマンス

### 8.1 メッセージサイズ

- JSONベースのシリアライゼーション
- 典型的なメッセージオーバーヘッド: 100-200バイト
- ペイロードサイズはトランスポートレイヤーによって制限

### 8.2 レイテンシー

- サブミリ秒のプロトコルオーバーヘッド
- ネットワークレイテンシーが全体的なパフォーマンスを決定
- 非同期処理によってブロッキング操作を排除

### 8.3 スループット

- トランスポートレイヤーとハンドラー実装によって制限
- 非同期ランタイムを通じた同時リクエストハンドリング
- 高負荷シナリオ向けの接続プール

## 9. バージョニングと互換性

### 9.1 プロトコルバージョニング

- セマンティックバージョニング（MAJOR.MINOR.PATCH）
- プロトコル定義で指定されるバージョン
- ハンドシェイク時のバージョンネゴシエーション

### 9.2 後方互換性

- 新しいオプションフィールド: 互換
- 新しい必須フィールド: 破壊的変更
- 新しいメソッド: 互換
- メソッドシグネチャーの変更: 破壊的変更

### 9.3 前方互換性

- デシリアライゼーション時に不明フィールドは無視
- 不明メソッドは「メソッドが見つかりません」エラーを返す
- バージョン不整合ハンドリング

## 10. 今後の拡張

### 10.1 計画中の機能

- **スキーマ進化**: 実行時スキーマ更新とマイグレーション
- **圧縮**: 大きなペイロード向けのメッセージ圧縮
- **バッチ操作**: 単一リクエストでの複数操作

### 10.2 言語サポート拡張

- TypeScript クライアント・サーバー生成の完成
- Python、Go等への展開

## 11. 関連ドキュメント

### 仕様書
- [spec/01: コアネットワーク](../01-core-concept/SPEC.md) - トランスポート層（QUIC）
- [spec/03: Stream Channel](../03-stream-channels/SPEC.md) - チャネル仕様

### 設計ドキュメント
- [KDLスキーマ例](../../schemas/) - 実際のスキーマ定義

### 参考資料
- [KDL仕様](https://kdl.dev/)
- [JSONスキーマ](https://json-schema.org/)

---

**仕様バージョン**: 1.1.0
**最終更新**: 2026-02-16
**ステータス**: Draft
