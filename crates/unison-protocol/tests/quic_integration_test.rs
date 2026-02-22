use anyhow::Result;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{Level, info};
use unison::network::channel::UnisonChannel;
use unison::network::MessageType;
use unison::{ProtocolServer, UnisonProtocol};

/// QUIC統合テスト - サーバーとクライアントを同一プロセスでテスト
/// TODO: Fix ping_pong.kdl schema parsing
#[tokio::test]
#[ignore]
async fn test_quic_server_client_integration() -> Result<()> {
    // ログ初期化
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_test_writer()
        .init();

    info!("Starting QUIC integration test");

    // サーバーとクライアントを同時に実行
    let server_handle = tokio::spawn(run_test_server());
    let client_handle = tokio::spawn(run_test_client());

    // サーバーが起動するまで少し待機
    tokio::time::sleep(Duration::from_millis(500)).await;

    // クライアントテストが完了するまで待機（タイムアウト付き）
    let client_result = timeout(Duration::from_secs(30), client_handle)
        .await
        .map_err(|_| anyhow::anyhow!("Test timeout"))??;

    // サーバーを停止
    server_handle.abort();

    info!("QUIC integration test completed successfully");
    client_result
}

/// テスト用サーバーの実行
async fn run_test_server() -> Result<()> {
    info!("Starting test server...");

    // Unison protocolインスタンス作成
    let mut protocol = UnisonProtocol::new();
    protocol.load_schema(include_str!("../../../schemas/ping_pong.kdl"))?;

    // サーバー作成とハンドラー登録
    let server = protocol.create_server();
    let start_time = Instant::now();

    register_test_channel_handlers(&server, start_time).await;

    info!("Test server started on [::1]:8080 (IPv6)");

    // サーバー開始（無限ループ）
    server.listen("[::1]:8080").await?;

    Ok(())
}

/// テスト用クライアントの実行
async fn run_test_client() -> Result<()> {
    info!("Starting test client...");

    // サーバーが完全に起動するまで待機
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Unison protocolインスタンス作成
    let mut protocol = UnisonProtocol::new();
    protocol.load_schema(include_str!("../../../schemas/ping_pong.kdl"))?;

    // クライアント作成と接続
    let client = protocol.create_client()?;
    client.connect("[::1]:8080").await?;
    info!("Connected to test server via IPv6");

    // チャネルを開いてテスト実行
    let channel = client.open_channel("ping").await?;
    run_integration_tests(&channel).await?;

    // 切断
    channel.close().await?;
    client.disconnect().await?;
    info!("Disconnected from test server");

    Ok(())
}

/// テスト用チャネルハンドラーの登録
async fn register_test_channel_handlers(server: &ProtocolServer, start_time: Instant) {
    server
        .register_channel("ping", move |_ctx, stream| async move {
            let channel = UnisonChannel::new(stream);

            loop {
                let msg = match channel.recv().await {
                    Ok(msg) => msg,
                    Err(_) => break,
                };

                if msg.msg_type != MessageType::Request {
                    continue;
                }

                let payload = msg.payload_as_value().unwrap_or_default();
                let request_id = msg.id;
                let method = msg.method.clone();

                let response = match method.as_str() {
                    "ping" => {
                        let message = payload
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Hello!")
                            .to_string();
                        let sequence = payload
                            .get("sequence")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        json!({
                            "message": format!("Pong: {}", message),
                            "sequence": sequence,
                            "server_info": "Test Server v1.0.0"
                        })
                    }
                    "echo" => {
                        let data = payload.get("data").cloned().unwrap_or_default();
                        let transform = payload
                            .get("transform")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        let echoed_data = match transform {
                            "uppercase" if data.is_string() => {
                                json!(data.as_str().unwrap().to_uppercase())
                            }
                            "reverse" if data.is_string() => {
                                json!(data.as_str().unwrap().chars().rev().collect::<String>())
                            }
                            _ => data.clone(),
                        };

                        json!({
                            "echoed_data": echoed_data,
                            "transformation_applied": if transform.is_empty() { None } else { Some(transform) }
                        })
                    }
                    "get_server_time" => {
                        let uptime_seconds = start_time.elapsed().as_secs();

                        json!({
                            "server_time": chrono::Utc::now().to_rfc3339(),
                            "timezone": "UTC",
                            "uptime_seconds": uptime_seconds
                        })
                    }
                    _ => json!({"error": "unknown method"}),
                };

                if let Err(e) = channel.send_response(request_id, &method, response).await {
                    tracing::warn!("Failed to send response: {}", e);
                    break;
                }
            }

            Ok(())
        })
        .await;

    info!("Test channel handlers registered");

    // Wait for handlers to be fully registered
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
}

/// 統合テストの実行
async fn run_integration_tests(channel: &UnisonChannel) -> Result<()> {
    info!("Running integration tests...");

    // Test 1: Server time check
    info!("Test 1: Server time check");
    let response = channel.request("get_server_time", json!({})).await?;
    info!(
        "Server response: {}",
        serde_json::to_string_pretty(&response)?
    );
    assert!(
        response.get("server_time").is_some(),
        "Server time should be present"
    );
    assert!(
        response.get("uptime_seconds").is_some(),
        "Uptime should be present"
    );
    info!("Server time test passed");

    // Test 2: Basic ping-pong
    info!("Test 2: Basic ping-pong");
    for i in 1..=3 {
        let response = channel
            .request(
                "ping",
                json!({
                    "message": format!("Test message {}", i),
                    "sequence": i
                }),
            )
            .await?;

        let pong_message = response
            .get("message")
            .and_then(|v| v.as_str())
            .expect("Pong message should be present");

        assert!(
            pong_message.contains("Pong:"),
            "Response should contain 'Pong:'"
        );
        assert_eq!(
            response.get("sequence").and_then(|v| v.as_i64()).unwrap(),
            i,
            "Sequence number should match"
        );
    }
    info!("Ping-pong test passed");

    // Test 3: Echo with transformations
    info!("Test 3: Echo transformations");

    // Uppercase test
    let response = channel
        .request(
            "echo",
            json!({
                "data": "hello world",
                "transform": "uppercase"
            }),
        )
        .await?;
    assert_eq!(
        response
            .get("echoed_data")
            .and_then(|v| v.as_str())
            .unwrap(),
        "HELLO WORLD",
        "Uppercase transformation should work"
    );

    // Reverse test
    let response = channel
        .request(
            "echo",
            json!({
                "data": "abcd",
                "transform": "reverse"
            }),
        )
        .await?;
    assert_eq!(
        response
            .get("echoed_data")
            .and_then(|v| v.as_str())
            .unwrap(),
        "dcba",
        "Reverse transformation should work"
    );

    // No transformation test
    let response = channel
        .request(
            "echo",
            json!({
                "data": "unchanged",
                "transform": ""
            }),
        )
        .await?;
    assert_eq!(
        response
            .get("echoed_data")
            .and_then(|v| v.as_str())
            .unwrap(),
        "unchanged",
        "No transformation should leave data unchanged"
    );
    info!("Echo transformation tests passed");

    // Test 4: Performance test (reduced size for integration test)
    info!("Test 4: Performance test");
    let start_time = Instant::now();

    for i in 1..=10 {
        let _response = channel
            .request(
                "ping",
                json!({
                    "message": format!("Perf test {}", i),
                    "sequence": i + 100
                }),
            )
            .await?;
    }

    let elapsed = start_time.elapsed();
    info!("Performance test completed in {:?}", elapsed);

    // Test 5: Complex JSON handling
    info!("Test 5: Complex JSON handling");
    let complex_data = json!({
        "nested": {
            "array": [1, 2, 3],
            "string": "test",
            "boolean": true
        },
        "number": 42
    });

    let response = channel
        .request(
            "echo",
            json!({
                "data": complex_data.clone(),
                "transform": ""
            }),
        )
        .await?;

    let echoed = response.get("echoed_data").unwrap();
    assert_eq!(
        echoed.get("nested").unwrap().get("array").unwrap(),
        &json!([1, 2, 3]),
        "Complex nested data should be preserved"
    );
    info!("Complex JSON test passed");

    info!("All integration tests passed!");
    Ok(())
}

/// rust-embed証明書の使用テスト
#[tokio::test]
async fn test_rust_embed_certificates() -> Result<()> {
    info!("Testing rust-embed certificate loading");

    use unison::network::quic::QuicServer;

    let result = QuicServer::load_cert_embedded();
    match result {
        Ok((certs, _private_key)) => {
            info!("rust-embed certificates loaded successfully");
            assert!(!certs.is_empty(), "Certificate chain should not be empty");
            info!("Certificate count: {}", certs.len());
        }
        Err(e) => {
            info!(
                "rust-embed certificates not found (expected in some environments): {}",
                e
            );
            let result = QuicServer::load_cert_auto();
            assert!(result.is_ok(), "Auto certificate loading should work");
            info!("Auto certificate loading works");
        }
    }

    Ok(())
}

/// QUIC設定の検証テスト
#[tokio::test]
async fn test_quic_configuration() -> Result<()> {
    info!("Testing QUIC configuration");

    use unison::network::quic::{QuicClient, QuicServer};

    // Server configuration test
    let server_config = QuicServer::configure_server().await;
    assert!(
        server_config.is_ok(),
        "Server configuration should be valid"
    );
    info!("Server configuration test passed");

    // Client configuration test
    let client_config = QuicClient::configure_client().await;
    assert!(
        client_config.is_ok(),
        "Client configuration should be valid"
    );
    info!("Client configuration test passed");

    Ok(())
}
