use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{Level, info};
use tracing_subscriber;
use unison::network::{MessageType, UnisonServer};
use unison::network::channel::UnisonChannel;
use unison::{ProtocolServer, UnisonProtocol};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Unison Protocol Ping Server Starting...");

    // Create Unison protocol instance
    let mut protocol = UnisonProtocol::new();

    // Load the ping-pong protocol schema
    protocol.load_schema(include_str!("../../../schemas/ping_pong.kdl"))?;

    // Create server
    let mut server = protocol.create_server();
    let start_time = Instant::now();

    // Register channel handlers
    register_channel_handlers(&server, start_time).await;

    info!("Unison Protocol Server Started!");
    info!("Listening on: quic://127.0.0.1:8080 (QUIC Transport)");
    info!("Run client with: cargo run --example unison_ping_client");
    info!("Available methods: ping, echo, get_server_time");
    info!("Press Ctrl+C to stop");

    // Start the server
    server.listen("127.0.0.1:8080").await?;

    // Keep the server running
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if !server.is_running() {
            break;
        }
    }

    Ok(())
}

async fn register_channel_handlers(server: &ProtocolServer, start_time: Instant) {
    // "ping" チャネル: ping, echo, get_server_time を処理
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
                            .unwrap_or("Hello from client!");
                        let sequence = payload
                            .get("sequence")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        info!("Received ping: \"{}\" (seq: {})", message, sequence);

                        json!({
                            "message": format!("Pong: {}", message),
                            "sequence": sequence,
                            "server_info": "Unison Protocol Server v1.0.0",
                            "processed_at": Utc::now().to_rfc3339()
                        })
                    }
                    "echo" => {
                        let data = payload.get("data").cloned().unwrap_or_default();
                        let transform = payload
                            .get("transform")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        info!("Echo request with transform: '{}'", transform);

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
                        info!("Server time requested, uptime: {}s", uptime_seconds);

                        json!({
                            "server_time": Utc::now().to_rfc3339(),
                            "timezone": "UTC",
                            "uptime_seconds": uptime_seconds
                        })
                    }
                    _ => {
                        json!({"error": format!("Unknown method: {}", method)})
                    }
                };

                if let Err(e) = channel.send_response(request_id, &method, response).await {
                    tracing::warn!("Failed to send response: {}", e);
                    break;
                }
            }

            Ok(())
        })
        .await;

    info!("Channel handlers registered successfully");
}
