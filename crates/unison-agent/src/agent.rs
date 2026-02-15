//! Agent機能の実装

use claude_agent_sdk::query;
use futures::StreamExt;
use tracing::{debug, info};

use crate::error::{AgentError, Result};

/// Claude Agentのシンプルなクライアント
pub struct AgentClient;

impl AgentClient {
    /// 新しいクライアントを作成
    pub fn new() -> Self {
        Self
    }

    /// シンプルなクエリを実行（ワンショット）
    ///
    /// # 引数
    ///
    /// * `prompt` - Claudeに送信するプロンプト
    ///
    /// # 戻り値
    ///
    /// すべてのレスポンスメッセージを結合した文字列
    pub async fn query(&self, prompt: &str) -> Result<String> {
        info!("Querying Claude: {}", prompt);

        let stream = query(prompt, None)
            .await
            .map_err(|e| AgentError::ClaudeAgent(e.to_string()))?;

        let mut stream = Box::pin(stream);
        let mut responses = Vec::new();

        while let Some(message_result) = stream.next().await {
            let message = message_result.map_err(|e| AgentError::Communication(e.to_string()))?;

            debug!("Received message chunk: {:?}", message);

            // メッセージをJSON文字列に変換
            let json_str =
                serde_json::to_string(&message).map_err(|e| AgentError::Other(e.into()))?;

            responses.push(json_str);
        }

        Ok(responses.join("\n"))
    }

    /// 複数のクエリをバッチ処理
    pub async fn batch_query(&self, prompts: Vec<&str>) -> Result<Vec<String>> {
        let mut results = Vec::new();

        for prompt in prompts {
            let response = self.query(prompt).await?;
            results.push(response);
        }

        Ok(results)
    }
}

impl Default for AgentClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_client_creation() {
        let _client = AgentClient::new();
    }

    #[test]
    fn test_agent_client_default() {
        let _client = AgentClient::default();
    }
}
