//! シンプルなクエリの例

use unison_agent::AgentClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロギングの初期化
    tracing_subscriber::fmt()
        .with_env_filter("unison_agent=debug,claude_agent_sdk=debug")
        .init();

    println!("=== Unison Agent: Simple Query Example ===\n");

    // エージェントクライアントの作成
    let client = AgentClient::new();

    // シンプルなクエリを実行
    println!("Querying Claude...");
    let response = client
        .query("What is 2 + 2? Please answer briefly.")
        .await?;

    println!("\n--- Response ---");
    println!("{}", response);

    Ok(())
}
