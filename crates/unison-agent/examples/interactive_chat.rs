//! 対話型チャットの例

use unison_agent::client::InteractiveClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロギングの初期化
    tracing_subscriber::fmt()
        .with_env_filter("unison_agent=info,claude_agent_sdk=info")
        .init();

    println!("=== Unison Agent: Interactive Chat Example ===\n");

    // 対話型クライアントの作成
    let mut client = InteractiveClient::new().await?;

    println!("Interactive client created. Starting conversation...\n");

    // 最初のメッセージを送信
    println!("Question: Hello! Can you tell me a fun fact about Rust programming language?");
    let responses = client
        .query("Hello! Can you tell me a fun fact about Rust programming language?")
        .await?;

    println!("\n--- Claude's Response ---");
    for response in &responses {
        println!("{}", response);
    }

    // 2回目のメッセージ
    println!("\n\n--- Follow-up Question ---");
    println!("Question: That's interesting! Can you give me another fact?");
    let responses = client
        .query("That's interesting! Can you give me another fact?")
        .await?;

    println!("\n--- Claude's Response ---");
    for response in &responses {
        println!("{}", response);
    }

    // クライアントを閉じる
    client.close().await?;
    println!("\n\nChat session completed!");

    Ok(())
}
