use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;
use tracing::{Level, info};
use unison::network::channel::UnisonChannel;
use unison::network::MessageType;
use unison::network::quic::QuicClient;
use unison::{ProtocolClient, ProtocolServer};

/// ベンチマーク結果
#[derive(Debug, Clone)]
struct BenchmarkResult {
    message_size: usize,
    avg_latency_us: f64,
    p50_latency_us: f64,
    p99_latency_us: f64,
    throughput_msg_per_sec: f64,
}

/// レイテンシを測定
async fn measure_latency(
    channel: &UnisonChannel,
    message_size: usize,
    iterations: usize,
) -> Vec<u64> {
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let message = json!({
            "data": "x".repeat(message_size),
            "sequence": i
        });

        let start = Instant::now();
        let _ = channel.request("echo", message).await;
        latencies.push(start.elapsed().as_micros() as u64);
    }

    latencies.sort_unstable();
    latencies
}

/// スループットを測定
async fn measure_throughput(
    channel: &UnisonChannel,
    message_size: usize,
    duration: Duration,
) -> f64 {
    let start = Instant::now();
    let mut count = 0u64;

    while start.elapsed() < duration {
        let message = json!({
            "data": "x".repeat(message_size),
            "sequence": count
        });

        if channel.request("echo", message).await.is_ok() {
            count += 1;
        }
    }

    count as f64 / start.elapsed().as_secs_f64()
}

/// ベンチマークサーバーを起動
async fn start_benchmark_server() -> Result<()> {
    let server = ProtocolServer::new();

    server
        .register_channel("bench", |_ctx, stream| async move {
            let channel = UnisonChannel::new(stream);

            loop {
                let msg = match channel.recv().await {
                    Ok(msg) => msg,
                    Err(_) => break,
                };

                if msg.msg_type == MessageType::Request {
                    let payload = msg.payload_as_value().unwrap_or_default();
                    channel.send_response(msg.id, &msg.method, payload).await?;
                }
            }

            Ok(())
        })
        .await;

    info!("Benchmark server starting on [::1]:8080");
    server.listen("[::1]:8080").await?;

    Ok(())
}

/// ベンチマークを実行
async fn run_benchmark(message_size: usize) -> Result<BenchmarkResult> {
    let quic_client = QuicClient::new()?;
    let client = ProtocolClient::new(quic_client);
    client.connect("[::1]:8080").await?;

    let channel = client.open_channel("bench").await?;

    info!("Testing with message size: {} bytes", message_size);

    // レイテンシ測定
    info!("  Measuring latency...");
    let latencies = measure_latency(&channel, message_size, 1000).await;

    let avg_latency = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
    let p50_latency = latencies[latencies.len() / 2] as f64;
    let p99_latency = latencies[latencies.len() * 99 / 100] as f64;

    // スループット測定
    info!("  Measuring throughput...");
    let throughput = measure_throughput(&channel, message_size, Duration::from_secs(5)).await;

    channel.close().await?;
    client.disconnect().await?;

    Ok(BenchmarkResult {
        message_size,
        avg_latency_us: avg_latency,
        p50_latency_us: p50_latency,
        p99_latency_us: p99_latency,
        throughput_msg_per_sec: throughput,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Unison Protocol Benchmark");
    info!("=============================");

    // サーバーを別タスクで起動
    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = barrier.clone();

    tokio::spawn(async move {
        let _ = start_benchmark_server().await;
        barrier_clone.wait().await;
    });

    // サーバーの起動を待つ
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 各メッセージサイズでベンチマーク実行
    let message_sizes = vec![64, 256, 1024, 4096, 16384];
    let mut results = Vec::new();

    for size in message_sizes {
        match run_benchmark(size).await {
            Ok(result) => {
                results.push(result.clone());
                info!("Completed benchmark for {} bytes", size);
                info!("   - Avg latency: {:.2} us", result.avg_latency_us);
                info!("   - P50 latency: {:.2} us", result.p50_latency_us);
                info!("   - P99 latency: {:.2} us", result.p99_latency_us);
                info!(
                    "   - Throughput: {:.0} msg/s",
                    result.throughput_msg_per_sec
                );
            }
            Err(e) => {
                eprintln!("Benchmark failed for {} bytes: {}", size, e);
            }
        }
    }

    // 結果のサマリーを表示
    info!("");
    info!("Benchmark Summary");
    info!("====================");
    info!("");
    info!("| Message Size | Avg Latency | P50 Latency | P99 Latency | Throughput |");
    info!("|-------------|-------------|-------------|-------------|------------|");

    for result in &results {
        info!(
            "| {:>11} | {:>9.2} us | {:>9.2} us | {:>9.2} us | {:>7.0} msg/s |",
            format!("{} B", result.message_size),
            result.avg_latency_us,
            result.p50_latency_us,
            result.p99_latency_us,
            result.throughput_msg_per_sec,
        );
    }

    info!("");
    info!("Benchmark completed!");

    barrier.wait().await;

    Ok(())
}
