//! Claude Agent用のUnisonツール実装
//!
//! このモジュールは、Claude AgentがUnison Protocol経由で
//! 外部サービスにアクセスするためのツールを提供します。

use claude_agent_sdk::mcp::{SdkMcpServer, SdkMcpTool, ToolResult};
use serde_json::{Value, json};
use tracing::{debug, info};
use unison::ProtocolClient;

use crate::error::{AgentError, Result};

/// Unison Protocolツールセット
pub struct UnisonTools {
    client: Option<ProtocolClient>,
    server_url: Option<String>,
}

impl UnisonTools {
    /// 新しいUnisonツールセットを作成
    pub fn new() -> Self {
        Self {
            client: None,
            server_url: None,
        }
    }

    /// MCP ServerとしてUnisonツールを構築
    pub fn build_mcp_server() -> SdkMcpServer {
        // Tool 1: Unisonサーバーへ接続
        let connect_tool = SdkMcpTool::new(
            "unison_connect",
            "Connect to a Unison Protocol server",
            json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The server URL to connect to (e.g., 'https://[::1]:8080')"
                    }
                },
                "required": ["url"]
            }),
            |args: Value| {
                Box::pin(async move {
                    let url = args["url"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))
                        .map_err(|e| {
                            claude_agent_sdk::error::ClaudeError::Connection(e.to_string())
                        })?;

                    info!("Connecting to Unison server: {}", url);

                    // TODO: 実際の接続処理
                    // let mut client = ProtocolClient::new_default()?;
                    // client.connect(url).await?;

                    Ok(ToolResult::text(format!(
                        "Successfully connected to Unison server at {}",
                        url
                    )))
                })
            },
        );

        // Tool 2: Unisonサービスを呼び出し
        let call_tool = SdkMcpTool::new(
            "unison_call",
            "Call a method on a Unison service",
            json!({
                "type": "object",
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "The name of the service to call"
                    },
                    "method": {
                        "type": "string",
                        "description": "The method name to invoke"
                    },
                    "payload": {
                        "type": "object",
                        "description": "The request payload as JSON"
                    }
                },
                "required": ["service", "method"]
            }),
            |args: Value| {
                Box::pin(async move {
                    let service = args["service"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing 'service' parameter"))
                        .map_err(|e| {
                            claude_agent_sdk::error::ClaudeError::Connection(e.to_string())
                        })?;
                    let method = args["method"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing 'method' parameter"))
                        .map_err(|e| {
                            claude_agent_sdk::error::ClaudeError::Connection(e.to_string())
                        })?;
                    let payload = args.get("payload").cloned().unwrap_or(json!({}));

                    info!(
                        "Calling Unison service: {}::{} with payload: {}",
                        service, method, payload
                    );

                    // TODO: 実際のサービス呼び出し
                    // let response = client.call_service(service, method, payload).await?;

                    Ok(ToolResult::text(format!(
                        "Called {}::{} (mock response - actual implementation needed)",
                        service, method
                    )))
                })
            },
        );

        // Tool 3: 接続中のサービス一覧を取得
        let list_tool = SdkMcpTool::new(
            "unison_list_services",
            "List available services on the connected Unison server",
            json!({
                "type": "object",
                "properties": {}
            }),
            |_args: Value| {
                Box::pin(async move {
                    info!("Listing Unison services");

                    // TODO: 実際のサービス一覧取得
                    // let services = client.list_services().await;

                    Ok(ToolResult::text(
                        "Available services: ExampleService, AnotherService (mock list)",
                    ))
                })
            },
        );

        // Tool 4: Unisonサーバーから切断
        let disconnect_tool = SdkMcpTool::new(
            "unison_disconnect",
            "Disconnect from the Unison Protocol server",
            json!({
                "type": "object",
                "properties": {}
            }),
            |_args: Value| {
                Box::pin(async move {
                    info!("Disconnecting from Unison server");

                    // TODO: 実際の切断処理
                    // client.disconnect().await?;

                    Ok(ToolResult::text(
                        "Successfully disconnected from Unison server",
                    ))
                })
            },
        );

        SdkMcpServer::new("unison-protocol")
            .version("0.1.0")
            .tools(vec![connect_tool, call_tool, list_tool, disconnect_tool])
    }

    /// Unisonサーバーへ接続
    pub async fn connect(&mut self, url: &str) -> Result<()> {
        info!("Connecting to Unison server: {}", url);

        let mut client = ProtocolClient::new_default()
            .map_err(|e| AgentError::Communication(format!("Failed to create client: {}", e)))?;

        client
            .connect(url)
            .await
            .map_err(|e| AgentError::Communication(format!("Connection failed: {}", e)))?;

        self.client = Some(client);
        self.server_url = Some(url.to_string());

        Ok(())
    }

    /// サービスを呼び出し
    pub async fn call_service(
        &mut self,
        service: &str,
        method: &str,
        payload: Value,
    ) -> Result<Value> {
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| AgentError::Communication("Not connected to server".to_string()))?;

        debug!(
            "Calling service: {}::{} with payload: {}",
            service, method, payload
        );

        client
            .call_service(service, method, payload)
            .await
            .map_err(|e| AgentError::Communication(format!("Service call failed: {}", e)))
    }

    /// 利用可能なサービス一覧を取得
    pub async fn list_services(&self) -> Result<Vec<String>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AgentError::Communication("Not connected to server".to_string()))?;

        Ok(client.list_services().await)
    }

    /// サーバーから切断
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut client) = self.client.take() {
            client
                .disconnect()
                .await
                .map_err(|e| AgentError::Communication(format!("Disconnect failed: {}", e)))?;
        }
        self.server_url = None;
        Ok(())
    }

    /// 接続状態を確認
    pub async fn is_connected(&self) -> bool {
        if let Some(client) = &self.client {
            client.is_connected().await
        } else {
            false
        }
    }
}

impl Default for UnisonTools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unison_tools_creation() {
        let _tools = UnisonTools::new();
    }

    #[test]
    fn test_build_mcp_server() {
        let _server = UnisonTools::build_mcp_server();
    }
}
