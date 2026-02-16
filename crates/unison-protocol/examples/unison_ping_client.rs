use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use std::time::Instant;
use tracing::{Level, info};
use tracing_subscriber;
use unison::network::UnisonClient;
use unison::{ProtocolClient, UnisonProtocol};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Unison Protocol Ping Client Starting");
    info!("Connecting to 127.0.0.1:8080 via QUIC...");

    // Create Unison protocol instance
    let mut protocol = UnisonProtocol::new();

    // Load the ping-pong protocol schema
    protocol.load_schema(include_str!("../../../schemas/ping_pong.kdl"))?;

    // Create client
    let mut client = protocol.create_client()?;

    // Connect to server (QUIC uses IP:Port format)
    client.connect("127.0.0.1:8080").await?;
    info!("Connected to Unison Protocol server!");

    // Open the "ping" channel
    let channel = client.open_channel("ping").await?;

    // Run tests via channel
    run_channel_tests(&channel).await?;

    // Close channel and disconnect
    channel.close().await?;
    info!("Disconnecting...");
    client.disconnect().await?;
    info!("Disconnected from Unison Protocol server!");

    Ok(())
}

async fn run_channel_tests(
    channel: &unison::network::channel::UnisonChannel,
) -> Result<()> {
    info!("");
    info!("Starting Unison Protocol Tests");
    info!("===================================");

    // Test 1: Get server time
    info!("");
    info!("Test 1: Get Server Time");
    info!("------------------------");
    let response = channel.request("get_server_time", json!({})).await?;

    if let (Some(server_time), Some(uptime)) = (
        response.get("server_time").and_then(|v| v.as_str()),
        response.get("uptime_seconds").and_then(|v| v.as_u64()),
    ) {
        info!("Server time: {} (uptime: {}s)", server_time, uptime);
    }

    // Test 2: Basic ping-pong
    info!("");
    info!("Test 2: Unison Protocol Ping-Pong (5 rounds)");
    info!("----------------------------------------------");
    for i in 1..=5 {
        let start_time = Instant::now();

        let response = channel
            .request(
                "ping",
                json!({
                    "message": format!("Hello from Unison client #{}", i),
                    "sequence": i,
                }),
            )
            .await?;

        let latency = start_time.elapsed();

        if let (Some(message), Some(server_info)) = (
            response.get("message").and_then(|v| v.as_str()),
            response.get("server_info").and_then(|v| v.as_str()),
        ) {
            info!(
                "Round {}: \"{}\" from {} (latency: {:?})",
                i, message, server_info, latency
            );
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }

    // Test 3: Echo with transformations
    info!("");
    info!("Test 3: Echo with Transformations");
    info!("----------------------------------");

    let test_cases = [
        ("uppercase", "Hello Unison Protocol!", "Transform to uppercase"),
        ("reverse", "protocol", "Reverse the string"),
        ("", "No transformation", "Echo as-is"),
    ];

    for (transform, data, description) in test_cases {
        let response = channel
            .request(
                "echo",
                json!({
                    "data": data,
                    "transform": transform
                }),
            )
            .await?;

        if let Some(echoed_data) = response.get("echoed_data") {
            info!("{}: \"{}\" -> \"{}\"", description, data, echoed_data);
        }
    }

    // Test 4: JSON Echo Test
    info!("");
    info!("Test 4: JSON Echo Test");
    info!("----------------------");

    let complex_data = json!({
        "user": {
            "name": "Alice",
            "age": 30,
            "preferences": ["music", "coding", "travel"]
        },
        "timestamp": Utc::now().to_rfc3339(),
        "metadata": {
            "version": "1.0.0",
            "protocol": "unison"
        }
    });

    let response = channel
        .request(
            "echo",
            json!({
                "data": complex_data,
                "transform": ""
            }),
        )
        .await?;

    if response.get("echoed_data").is_some() {
        info!("Complex JSON echoed successfully");
    }

    // Test 5: Performance test
    info!("");
    info!("Test 5: Performance Test (20 rapid pings)");
    info!("------------------------------------------");
    let perf_start = Instant::now();
    let mut total_latency = std::time::Duration::ZERO;

    for i in 1..=20 {
        let ping_start = Instant::now();

        let _response = channel
            .request(
                "ping",
                json!({
                    "message": format!("Perf test #{}", i),
                    "sequence": i + 1000
                }),
            )
            .await?;

        total_latency += ping_start.elapsed();

        if i % 5 == 0 {
            info!("Progress: {}/20 pings completed", i);
        }
    }

    let total_perf_time = perf_start.elapsed();
    let avg_latency = total_latency / 20;

    info!("");
    info!("Performance Test Results:");
    info!("  Total time: {:?}", total_perf_time);
    info!("  Average latency: {:?}", avg_latency);
    info!(
        "  Throughput: {:.1} pings/sec",
        20.0 / total_perf_time.as_secs_f64()
    );

    // Test 6: Final server status
    info!("");
    info!("Test 6: Final Server Status");
    info!("----------------------------");
    let final_response = channel.request("get_server_time", json!({})).await?;
    if let Some(uptime) = final_response
        .get("uptime_seconds")
        .and_then(|v| v.as_u64())
    {
        info!("Final server uptime: {}s", uptime);
    }

    info!("");
    info!("All Unison Protocol tests completed successfully!");
    info!("====================================================");

    Ok(())
}
