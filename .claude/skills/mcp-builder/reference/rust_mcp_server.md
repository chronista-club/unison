# Rust MCPサーバー実装ガイド

## 概要

このドキュメントでは、Rustを使用してMCPサーバーを実装するためのベストプラクティスと実装パターンを提供します。Rustの強力な型システム、メモリ安全性、並行処理能力を活用して、高性能で信頼性の高いMCPサーバーを構築する方法を説明します。

---

## クイックリファレンス

### 主要な依存関係 (Cargo.toml)
```toml
[dependencies]
# 非同期ランタイム
tokio = { version = "1.40", features = ["full"] }
# JSON-RPC実装
jsonrpc-core = "18.0"
jsonrpc-derive = "18.0"
# シリアライゼーション
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
# HTTPサーバー
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
# SSE (Server-Sent Events)
axum-extra = { version = "0.9", features = ["typed-header"] }
futures = "0.3"
# エラーハンドリング
thiserror = "1.0"
anyhow = "1.0"
# 環境変数
dotenv = "0.15"
# ロギング
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
# HTTPクライアント
reqwest = { version = "0.12", features = ["json"] }
# スキーマ検証
validator = { version = "0.18", features = ["derive"] }
```

### サーバーの初期化
```rust
use axum::{Router, routing::post};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロギングの初期化
    tracing_subscriber::fmt::init();

    // 環境変数の読み込み
    dotenv::dotenv().ok();

    // MCPハンドラーの作成
    let mcp_handler = McpHandler::new();

    // ルーターの設定
    let app = Router::new()
        .route("/mcp", post(handle_mcp_request))
        .route("/sse", get(handle_sse))
        .layer(CorsLayer::permissive())
        .with_state(mcp_handler);

    // サーバーの起動
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("MCPサーバーが起動しました: {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

---

## Rustの利点を活かしたMCPサーバー設計

### 1. 強力な型システムの活用

```rust
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

// MCPリクエストの型定義
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "method", content = "params")]
pub enum McpRequest {
    #[serde(rename = "initialize")]
    Initialize(InitializeParams),

    #[serde(rename = "tools/list")]
    ListTools,

    #[serde(rename = "tools/call")]
    CallTool(CallToolParams),
}

// ツールパラメータの型定義と検証
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct SearchParams {
    #[validate(length(min = 1, max = 100))]
    pub query: String,

    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,

    #[serde(default)]
    pub include_archived: bool,
}
```

### 2. エラーハンドリングの実装

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("無効なリクエスト: {0}")]
    InvalidRequest(String),

    #[error("ツールが見つかりません: {0}")]
    ToolNotFound(String),

    #[error("認証エラー: {0}")]
    AuthenticationError(String),

    #[error("レート制限を超えました")]
    RateLimitExceeded,

    #[error("内部サーバーエラー: {0}")]
    InternalError(#[from] anyhow::Error),
}

// Result型のエイリアス
pub type McpResult<T> = Result<T, McpError>;
```

### 3. トレイトベースの抽象化

```rust
use async_trait::async_trait;

// MCPツールのトレイト定義
#[async_trait]
pub trait McpTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> McpResult<ToolResponse>;
}

// ツールレジストリ
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn McpTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: McpTool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    pub async fn call_tool(&self, name: &str, params: serde_json::Value) -> McpResult<ToolResponse> {
        let tool = self.tools.get(name)
            .ok_or_else(|| McpError::ToolNotFound(name.to_string()))?;

        tool.execute(params).await
    }
}
```

---

## 並行処理とパフォーマンス最適化

### 1. 非同期処理の実装

```rust
use futures::stream::{Stream, StreamExt};
use tokio::sync::mpsc;

// SSEストリーミングの実装
pub async fn handle_sse(
    State(handler): State<McpHandler>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(100);

    // バックグラウンドタスクの起動
    tokio::spawn(async move {
        handler.stream_events(tx).await;
    });

    let stream = ReceiverStream::new(rx)
        .map(|msg| Ok(Event::default().data(msg)));

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
}
```

### 2. コネクションプーリング

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ConnectionPool {
    connections: Arc<RwLock<Vec<Connection>>>,
    max_connections: usize,
}

impl ConnectionPool {
    pub async fn acquire(&self) -> McpResult<PooledConnection> {
        let mut connections = self.connections.write().await;

        if let Some(conn) = connections.pop() {
            Ok(PooledConnection::new(conn, self.connections.clone()))
        } else if connections.len() < self.max_connections {
            let conn = Connection::new().await?;
            Ok(PooledConnection::new(conn, self.connections.clone()))
        } else {
            Err(McpError::RateLimitExceeded)
        }
    }
}
```

### 3. メモリ効率的なバッファリング

```rust
use bytes::{Bytes, BytesMut};

pub struct MessageBuffer {
    buffer: BytesMut,
    max_size: usize,
}

impl MessageBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(4096),
            max_size,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> McpResult<()> {
        if self.buffer.len() + data.len() > self.max_size {
            return Err(McpError::InvalidRequest("メッセージが大きすぎます".to_string()));
        }

        self.buffer.extend_from_slice(data);
        Ok(())
    }

    pub fn take(&mut self) -> Bytes {
        self.buffer.split().freeze()
    }
}
```

---

## ツールの実装例

### 1. シンプルなツールの実装

```rust
use async_trait::async_trait;

pub struct PingTool;

#[async_trait]
impl McpTool for PingTool {
    fn name(&self) -> &str {
        "ping"
    }

    fn description(&self) -> &str {
        "接続をテストします"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _params: serde_json::Value) -> McpResult<ToolResponse> {
        Ok(ToolResponse {
            content: vec![Content::Text {
                text: "pong".to_string(),
            }],
        })
    }
}
```

### 2. 外部APIを使用するツール

```rust
use reqwest::Client;

pub struct WeatherTool {
    client: Client,
    api_key: String,
}

#[async_trait]
impl McpTool for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }

    fn description(&self) -> &str {
        "指定された場所の天気情報を取得します"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "都市名または郵便番号"
                }
            },
            "required": ["location"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> McpResult<ToolResponse> {
        let location = params["location"].as_str()
            .ok_or_else(|| McpError::InvalidRequest("location is required".to_string()))?;

        let response = self.client
            .get("https://api.weather.com/v1/current")
            .query(&[("q", location), ("apikey", &self.api_key)])
            .send()
            .await
            .map_err(|e| McpError::InternalError(e.into()))?;

        let weather_data = response.json::<WeatherData>().await
            .map_err(|e| McpError::InternalError(e.into()))?;

        Ok(ToolResponse {
            content: vec![Content::Text {
                text: format_weather(&weather_data),
            }],
        })
    }
}
```

---

## セキュリティとベストプラクティス

### 1. 入力検証

```rust
use validator::Validate;

pub async fn validate_and_execute<T>(
    params: serde_json::Value,
    handler: impl FnOnce(T) -> McpResult<ToolResponse>
) -> McpResult<ToolResponse>
where
    T: for<'de> Deserialize<'de> + Validate,
{
    // デシリアライズ
    let typed_params: T = serde_json::from_value(params)
        .map_err(|e| McpError::InvalidRequest(e.to_string()))?;

    // 検証
    typed_params.validate()
        .map_err(|e| McpError::InvalidRequest(e.to_string()))?;

    // 実行
    handler(typed_params)
}
```

### 2. レート制限

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct RateLimiter {
    limits: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub async fn check_limit(&self, key: &str) -> McpResult<()> {
        let mut limits = self.limits.lock().await;
        let now = Instant::now();

        let timestamps = limits.entry(key.to_string()).or_insert_with(Vec::new);

        // 古いタイムスタンプを削除
        timestamps.retain(|&t| now.duration_since(t) < self.window);

        if timestamps.len() >= self.max_requests {
            return Err(McpError::RateLimitExceeded);
        }

        timestamps.push(now);
        Ok(())
    }
}
```

### 3. 機密情報の保護

```rust
use secrecy::{Secret, ExposeSecret};

pub struct Config {
    pub openai_api_key: Secret<String>,
    pub database_url: Secret<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        Ok(Self {
            openai_api_key: Secret::new(
                std::env::var("OPENAI_API_KEY")
                    .expect("OPENAI_API_KEY must be set")
            ),
            database_url: Secret::new(
                std::env::var("DATABASE_URL")
                    .expect("DATABASE_URL must be set")
            ),
        })
    }
}

// 使用時のみ公開
async fn use_api_key(config: &Config) {
    let key = config.openai_api_key.expose_secret();
    // APIキーを使用
}
```

---

## 完全な実装例

以下は、複数のツールを持つ完全なMCPサーバーの実装例です：

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::stream::{Stream, StreamExt};
use jsonrpc_core::{IoHandler, Params, Value};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

// ===== 型定義 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    pub tools: Option<ToolsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    pub call: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub content: Vec<Content>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

// ===== MCPハンドラー =====

#[derive(Clone)]
pub struct McpHandler {
    tools: Arc<RwLock<HashMap<String, Tool>>>,
    rate_limiter: Arc<RateLimiter>,
}

impl McpHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(RateLimiter::new(100, Duration::from_secs(60))),
        };

        // ツールの登録
        handler.register_tools();

        handler
    }

    fn register_tools(&self) {
        let tools = vec![
            Tool {
                name: "ping".to_string(),
                description: "接続をテストします".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            Tool {
                name: "echo".to_string(),
                description: "メッセージをエコーバックします".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "エコーバックするメッセージ"
                        }
                    },
                    "required": ["message"]
                }),
            },
            Tool {
                name: "calculate".to_string(),
                description: "簡単な計算を実行します".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["add", "subtract", "multiply", "divide"],
                            "description": "実行する演算"
                        },
                        "a": {
                            "type": "number",
                            "description": "最初の数値"
                        },
                        "b": {
                            "type": "number",
                            "description": "2番目の数値"
                        }
                    },
                    "required": ["operation", "a", "b"]
                }),
            },
        ];

        let mut tools_map = self.tools.blocking_write();
        for tool in tools {
            tools_map.insert(tool.name.clone(), tool);
        }
    }

    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // レート制限のチェック
        if let Err(e) = self.rate_limiter.check_limit("global").await {
            return JsonRpcResponse::error(request.id, -32000, e.to_string());
        }

        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_list_tools(request).await,
            "tools/call" => self.handle_call_tool(request).await,
            _ => JsonRpcResponse::error(
                request.id,
                -32601,
                format!("Method not found: {}", request.method),
            ),
        }
    }

    async fn handle_initialize(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params: InitializeRequest = match serde_json::from_value(request.params) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(request.id, -32602, e.to_string());
            }
        };

        info!("Initializing MCP server for client: {}", params.client_info.name);

        let response = serde_json::json!({
            "protocol_version": "1.0.0",
            "server_info": {
                "name": "rust_mcp_server",
                "version": "0.1.0"
            },
            "capabilities": {
                "tools": {
                    "list": true,
                    "call": true
                }
            }
        });

        JsonRpcResponse::success(request.id, response)
    }

    async fn handle_list_tools(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let tools = self.tools.read().await;
        let tools_list: Vec<&Tool> = tools.values().collect();

        JsonRpcResponse::success(
            request.id,
            serde_json::json!({ "tools": tools_list }),
        )
    }

    async fn handle_call_tool(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let tool_call: ToolCall = match serde_json::from_value(request.params) {
            Ok(tc) => tc,
            Err(e) => {
                return JsonRpcResponse::error(request.id, -32602, e.to_string());
            }
        };

        let tools = self.tools.read().await;
        if !tools.contains_key(&tool_call.name) {
            return JsonRpcResponse::error(
                request.id,
                -32602,
                format!("Tool not found: {}", tool_call.name),
            );
        }

        // ツールの実行
        let response = match tool_call.name.as_str() {
            "ping" => self.execute_ping().await,
            "echo" => self.execute_echo(tool_call.arguments).await,
            "calculate" => self.execute_calculate(tool_call.arguments).await,
            _ => {
                return JsonRpcResponse::error(
                    request.id,
                    -32602,
                    format!("Tool not implemented: {}", tool_call.name),
                );
            }
        };

        match response {
            Ok(tool_response) => JsonRpcResponse::success(request.id, serde_json::to_value(tool_response).unwrap()),
            Err(e) => JsonRpcResponse::error(request.id, -32603, e.to_string()),
        }
    }

    async fn execute_ping(&self) -> McpResult<ToolResponse> {
        Ok(ToolResponse {
            content: vec![Content::Text {
                text: "pong".to_string(),
            }],
        })
    }

    async fn execute_echo(&self, params: serde_json::Value) -> McpResult<ToolResponse> {
        let message = params["message"]
            .as_str()
            .ok_or_else(|| McpError::InvalidRequest("message is required".to_string()))?;

        Ok(ToolResponse {
            content: vec![Content::Text {
                text: message.to_string(),
            }],
        })
    }

    async fn execute_calculate(&self, params: serde_json::Value) -> McpResult<ToolResponse> {
        let operation = params["operation"]
            .as_str()
            .ok_or_else(|| McpError::InvalidRequest("operation is required".to_string()))?;

        let a = params["a"]
            .as_f64()
            .ok_or_else(|| McpError::InvalidRequest("a must be a number".to_string()))?;

        let b = params["b"]
            .as_f64()
            .ok_or_else(|| McpError::InvalidRequest("b must be a number".to_string()))?;

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Err(McpError::InvalidRequest("Division by zero".to_string()));
                }
                a / b
            }
            _ => {
                return Err(McpError::InvalidRequest(format!("Unknown operation: {}", operation)));
            }
        };

        Ok(ToolResponse {
            content: vec![Content::Text {
                text: format!("{} {} {} = {}", a, operation, b, result),
            }],
        })
    }
}

// ===== JSON-RPC型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// ===== HTTPハンドラー =====

async fn handle_mcp_request(
    State(handler): State<McpHandler>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let response = handler.handle_request(request).await;
    Json(response)
}

async fn handle_sse(
    State(handler): State<McpHandler>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    use axum::response::sse::Event;
    use std::convert::Infallible;
    use tokio_stream::wrappers::ReceiverStream;

    let (tx, rx) = tokio::sync::mpsc::channel(100);

    // 初期メッセージの送信
    let _ = tx.send(Ok(Event::default().data("connected"))).await;

    let stream = ReceiverStream::new(rx);

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// ===== メイン関数 =====

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロギングの初期化
    tracing_subscriber::fmt()
        .with_env_filter("info,rust_mcp_server=debug")
        .init();

    // 環境変数の読み込み
    dotenv::dotenv().ok();

    // MCPハンドラーの作成
    let mcp_handler = McpHandler::new();

    // ルーターの設定
    let app = Router::new()
        .route("/mcp", post(handle_mcp_request))
        .route("/sse", get(handle_sse))
        .route("/health", get(health_check))
        .layer(CorsLayer::permissive())
        .with_state(mcp_handler);

    // サーバーの起動
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;

    info!("🚀 Rust MCPサーバーが起動しました: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## MCP Inspectorでのテスト

### 対話型テストとデバッグ

**MCP Inspector**は、MCPサーバーのテストとデバッグのための公式ツールです：

```bash
# Rustサーバーのビルド
cargo build --release

# MCP Inspectorで起動
npx @modelcontextprotocol/inspector ./target/release/akasha-mcp-rust

# 引数付きの場合
npx @modelcontextprotocol/inspector ./target/release/akasha-mcp-rust --arg1 value1
```

Inspectorを使用することで、以下を対話的にテストできます：
- ツールの実行と結果の確認
- エラーハンドリングの動作確認
- リクエスト/レスポンスのリアルタイム監視
- JSON-RPCメッセージの詳細確認

### 評価ハーネスを使用した自動テスト

TypeScript版の評価ハーネスを使用して、Rustサーバーの包括的なテストが可能です：

```bash
# 評価スクリプトディレクトリに移動
cd ../scripts

# Bunを使用した高速実行
bun install
bun run evaluation.ts eval.xml -t stdio -c cargo -a run

# またはリリースビルドのテスト
bun run dev eval.xml -t stdio -c ../rust/target/release/akasha-mcp-rust
```

評価ハーネスの詳細は [scripts/README.md](../scripts/README.md) を参照してください。

## テストの実装

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_initialize() {
        let handler = McpHandler::new();
        let app = create_app(handler);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(1),
            method: "initialize".to_string(),
            params: serde_json::json!({
                "protocol_version": "1.0.0",
                "capabilities": {},
                "client_info": {
                    "name": "test_client",
                    "version": "1.0.0"
                }
            }),
        };

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ping_tool() {
        let handler = McpHandler::new();
        let response = handler.execute_ping().await.unwrap();

        assert_eq!(response.content.len(), 1);
        match &response.content[0] {
            Content::Text { text } => assert_eq!(text, "pong"),
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_calculate_tool() {
        let handler = McpHandler::new();

        let params = serde_json::json!({
            "operation": "add",
            "a": 5,
            "b": 3
        });

        let response = handler.execute_calculate(params).await.unwrap();

        assert_eq!(response.content.len(), 1);
        match &response.content[0] {
            Content::Text { text } => assert!(text.contains("8")),
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_division_by_zero() {
        let handler = McpHandler::new();

        let params = serde_json::json!({
            "operation": "divide",
            "a": 5,
            "b": 0
        });

        let result = handler.execute_calculate(params).await;
        assert!(result.is_err());
    }
}
```

---

## デプロイメント

### Dockerfileの例

```dockerfile
# ビルドステージ
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# 実行ステージ
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rust_mcp_server /usr/local/bin/

ENV PORT=3000
EXPOSE 3000

CMD ["rust_mcp_server"]
```

### systemdサービスファイル

```ini
[Unit]
Description=Rust MCP Server
After=network.target

[Service]
Type=simple
User=mcp
WorkingDirectory=/opt/rust_mcp_server
Environment="PORT=3000"
Environment="RUST_LOG=info"
ExecStart=/opt/rust_mcp_server/rust_mcp_server
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

---

## トラブルシューティング

### よくある問題と解決策

1. **接続エラー**
   - CORSヘッダーが正しく設定されているか確認
   - ポートがファイアウォールで開放されているか確認

2. **パフォーマンス問題**
   - `tokio-console`を使用してタスクのボトルネックを特定
   - `cargo flamegraph`でCPUプロファイリングを実行

3. **メモリリーク**
   - `valgrind`または`heaptrack`を使用してメモリ使用量を監視
   - 長時間実行されるタスクが適切にクリーンアップされているか確認

### デバッグログの有効化

```rust
// 環境変数で設定
RUST_LOG=debug cargo run

// コード内で動的に設定
use tracing_subscriber::EnvFilter;

tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env()
        .add_directive("rust_mcp_server=debug".parse().unwrap()))
    .init();
```

---

## まとめ

Rustを使用したMCPサーバーの実装により、以下の利点が得られます：

- **メモリ安全性**: 所有権システムによるメモリリークやデータ競合の防止
- **高パフォーマンス**: ゼロコスト抽象化と効率的な並行処理
- **型安全性**: コンパイル時の厳密な型チェック
- **エラーハンドリング**: Result型による明示的なエラー処理

このガイドで紹介したパターンとベストプラクティスを活用することで、堅牢で高性能なMCPサーバーを構築できます。