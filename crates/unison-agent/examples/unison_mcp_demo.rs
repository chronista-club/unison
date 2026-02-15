//! Unison Protocol + MCP統合デモ
//!
//! このExampleは、Claude AgentがMCP経由でUnison Protocolツールを
//! 使用して外部サービスにアクセスする方法を示します。

use claude_agent_sdk::{ClaudeAgentOptions, query};
use futures::StreamExt;
use unison_agent::tools::UnisonTools;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロギングの初期化
    tracing_subscriber::fmt()
        .with_env_filter("unison_agent=debug,claude_agent_sdk=info")
        .init();

    println!("=== Unison Agent: MCP Integration Demo ===\n");

    // Unison ToolsをMCP Serverとして構築
    let unison_mcp = UnisonTools::build_mcp_server();

    println!("Building Claude Agent with Unison Protocol tools...\n");

    // Claude AgentのオプションにMCP Serverを追加
    let options = ClaudeAgentOptions::builder()
        .system_prompt(
            "You are an AI assistant with access to Unison Protocol services. \
             You can connect to Unison servers, list services, and call methods on them. \
             Use the unison_* tools to interact with Unison Protocol servers.",
        )
        .mcp_server(unison_mcp)
        .build();

    println!("Agent created with Unison tools available.\n");

    // デモクエリ: Unisonツールの使い方を聞く
    println!("--- Demo Query 1: Ask about available tools ---");
    println!("Query: What Unison Protocol tools do you have access to?\n");

    let stream = query(
        "What Unison Protocol tools do you have access to? \
         Please list them and briefly explain what each one does.",
        Some(options.clone()),
    )
    .await?;

    let mut stream = Box::pin(stream);

    while let Some(message) = stream.next().await {
        match message? {
            claude_agent_sdk::Message::Assistant { message, .. } => {
                for block in &message.content {
                    if let claude_agent_sdk::ContentBlock::Text { text } = block {
                        println!("Claude: {}\n", text);
                    }
                }
            }
            claude_agent_sdk::Message::Result { .. } => {
                break;
            }
            _ => {}
        }
    }

    println!("\n--- Demo Query 2: Simulate Unison workflow ---");
    println!("Query: Show me how to connect to a Unison server and call a service\n");

    let stream2 = query(
        "I want to connect to a Unison Protocol server at 'https://[::1]:8080', \
         then list available services, and call the 'ping' method on the 'EchoService'. \
         Can you walk me through the steps using your tools? \
         (Note: This is a simulation, the server doesn't actually exist)",
        Some(options),
    )
    .await?;

    let mut stream2 = Box::pin(stream2);

    while let Some(message) = stream2.next().await {
        match message? {
            claude_agent_sdk::Message::Assistant { message, .. } => {
                for block in &message.content {
                    if let claude_agent_sdk::ContentBlock::Text { text } = block {
                        println!("Claude: {}\n", text);
                    }
                }
            }
            claude_agent_sdk::Message::Result { .. } => {
                break;
            }
            _ => {}
        }
    }

    println!("\n=== Demo Completed ===");
    println!("\nThis demo showed:");
    println!("1. How to build MCP tools for Unison Protocol");
    println!("2. How to integrate them with Claude Agent SDK");
    println!("3. How Claude can discover and use these tools");
    println!("\nNext steps:");
    println!("- Implement actual Unison server connection");
    println!("- Add more sophisticated error handling");
    println!("- Create real Unison services to interact with");

    Ok(())
}
