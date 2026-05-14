//! UnisonProbe — MCP server 本体。3 つの tool を保持する。
//!
//! ## Tools
//! - `unison_ping` — 指定 endpoint に接続して疎通確認
//! - `unison_call` — 任意 channel を open して payload を送信、response を返す
//! - `unison_channel_list` — **TODO**: サーバ側登録済み channel を列挙 (要 Unison 側 API 追加)

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

/// MCP server の state。現時点では stateless (毎回 client を作り直す)。
#[derive(Clone)]
pub struct UnisonProbe {
    tool_router: ToolRouter<UnisonProbe>,
}

impl UnisonProbe {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for UnisonProbe {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tool input schemas
// ---------------------------------------------------------------------------

/// Trust mode selector for the probe (matches `club_unison::TrustAnchors`).
///
/// - `"skip"`: skip cert verification (dev only, against self-signed servers)
/// - `"system"`: use OS/webpki-roots trust store (for public servers)
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TrustMode {
    Skip,
    System,
}

impl Default for TrustMode {
    fn default() -> Self {
        Self::Skip
    }
}

impl TrustMode {
    fn to_anchors(&self) -> club_unison::network::TrustAnchors {
        match self {
            Self::Skip => club_unison::network::TrustAnchors::SkipVerification,
            Self::System => club_unison::network::TrustAnchors::System,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PingArgs {
    /// Unison サーバの URL (例: `quic://[::1]:7878`)
    pub endpoint: String,
    /// Trust anchor mode (default: `skip` — dev self-signed 用)
    #[serde(default)]
    pub trust: TrustMode,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CallArgs {
    /// Unison サーバの URL
    pub endpoint: String,
    /// 対象 channel 名
    pub channel_name: String,
    /// 対象 method 名 (KDL schema の `request "Name"` の Name 部分)
    pub method: String,
    /// 送信する JSON payload
    pub payload: serde_json::Value,
    /// Trust anchor mode (default: `skip`)
    #[serde(default)]
    pub trust: TrustMode,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChannelListArgs {
    /// Unison サーバの URL
    pub endpoint: String,
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

#[tool_router]
impl UnisonProbe {
    #[tool(description = "Unison サーバへの疎通確認。endpoint に接続して切断する")]
    async fn unison_ping(
        &self,
        Parameters(args): Parameters<PingArgs>,
    ) -> Result<CallToolResult, McpError> {
        use club_unison::ProtocolClient;
        use club_unison::network::quic::QuicClient;

        // v0.8.0 builder で trust anchor を明示
        let quic = QuicClient::builder()
            .trust_anchors(args.trust.to_anchors())
            .build()
            .map_err(|e| McpError::internal_error(format!("client init failed: {e}"), None))?;
        let client = ProtocolClient::new(quic);

        client
            .connect(&args.endpoint)
            .await
            .map_err(|e| McpError::internal_error(format!("connect failed: {e}"), None))?;

        let msg = format!("✅ connected to {} (trust={:?})", args.endpoint, args.trust);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    #[tool(description = "任意の Unison channel を open し、method に payload を request として送信して response を取得する")]
    async fn unison_call(
        &self,
        Parameters(args): Parameters<CallArgs>,
    ) -> Result<CallToolResult, McpError> {
        use club_unison::ProtocolClient;
        use club_unison::network::quic::QuicClient;

        // v0.8.0 builder で trust anchor を明示
        let quic = QuicClient::builder()
            .trust_anchors(args.trust.to_anchors())
            .build()
            .map_err(|e| McpError::internal_error(format!("client init failed: {e}"), None))?;
        let client = ProtocolClient::new(quic);

        client
            .connect(&args.endpoint)
            .await
            .map_err(|e| McpError::internal_error(format!("connect failed: {e}"), None))?;

        let channel = client
            .open_channel(&args.channel_name)
            .await
            .map_err(|e| McpError::internal_error(format!("open_channel failed: {e}"), None))?;

        let response: serde_json::Value = channel
            .request(&args.method, &args.payload)
            .await
            .map_err(|e| McpError::internal_error(format!("request failed: {e}"), None))?;

        let result = serde_json::json!({
            "channel": args.channel_name,
            "method": args.method,
            "response": response,
        });

        Ok(CallToolResult::success(vec![Content::text(result.to_string())]))
    }

    #[tool(description = "サーバに登録されている channel 一覧を取得する (サーバ側 API 追加が前提)")]
    async fn unison_channel_list(
        &self,
        Parameters(_args): Parameters<ChannelListArgs>,
    ) -> Result<CallToolResult, McpError> {
        // TODO: Unison サーバに "__channels:list" のような meta channel を追加する必要あり。
        // 現時点ではサーバ側で channel を列挙する API が無いので、未実装として明示する。
        Err(McpError::internal_error(
            "unison_channel_list: サーバ側 meta API が未実装です (将来対応)",
            None,
        ))
    }
}

// ---------------------------------------------------------------------------
// ServerHandler
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for UnisonProbe {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "unison-mcp-probe".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: Some("MCP probe for Unison Protocol servers".to_string()),
                ..Default::default()
            },
            instructions: Some(
                "Unison Protocol サーバをつつくための MCP probe。開発中の Unison endpoint を指定して使う。"
                    .to_string(),
            ),
        }
    }
}
