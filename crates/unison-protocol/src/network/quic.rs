use anyhow::{Context, Result};
use quinn::{ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig};
use rust_embed::RustEmbed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig as RustlsClientConfig, ServerConfig as RustlsServerConfig};
use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{error, info, warn};

use super::{
    NetworkError, ProtocolFrame, ProtocolMessage,
    context::ConnectionContext, server::ProtocolServer,
};

/// Default certificate file paths for assets/certs directory
pub const DEFAULT_CERT_PATH: &str = "assets/certs/cert.pem";
pub const DEFAULT_KEY_PATH: &str = "assets/certs/private_key.der";

/// Maximum message size for QUIC streams (8MB)
const MAX_MESSAGE_SIZE: usize = 8 * 1024 * 1024;

/// Default port for QUIC connections
const DEFAULT_PORT: u16 = 8080;

/// IPv6アドレス文字列をSocketAddrに変換する共通関数
///
/// 対応形式:
/// - `[::1]:8080` — 標準 IPv6+port
/// - `::1` — IPv6 のみ（デフォルトポート付与）
/// - `8080` — ポートのみ（IPv6 ループバック）
/// - `localhost:8080` — ループバック
fn parse_ipv6_address(addr: &str) -> Result<SocketAddr> {
    // まず直接パースを試みる（IPv6のみ受け入れる）
    if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
        match socket_addr {
            SocketAddr::V6(_) => return Ok(socket_addr),
            SocketAddr::V4(_) => {
                return Err(anyhow::anyhow!(
                    "IPv4アドレスはサポートされていません: {}",
                    addr
                ));
            }
        }
    }

    // IPv6アドレスとして解析を試みる（ポートなし）
    if addr.contains(':') && !addr.contains('[') && !addr.contains('.') {
        let addr_with_brackets = format!("[{}]:{}", addr, DEFAULT_PORT);
        if let Ok(socket_addr @ SocketAddr::V6(_)) = addr_with_brackets.parse::<SocketAddr>() {
            return Ok(socket_addr);
        }
    }

    // ポート番号のみの場合はIPv6ループバックを使用
    if let Ok(port) = addr.parse::<u16>() {
        return Ok(SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port)));
    }

    // "localhost:port"形式の場合はIPv6ループバックを使用
    if let Some(stripped) = addr.strip_prefix("localhost:")
        && let Ok(port) = stripped.parse::<u16>()
    {
        return Ok(SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port)));
    }

    // [IPv6]:port 形式を解析
    if addr.starts_with('[')
        && let Some(end) = addr.find(']')
    {
        let ipv6_str = &addr[1..end];
        let port_str = if addr.len() > end + 1 && &addr[end + 1..end + 2] == ":" {
            &addr[end + 2..]
        } else {
            return Err(anyhow::anyhow!("無効なIPv6アドレス形式: {}", addr));
        };

        let ipv6 = ipv6_str
            .parse::<std::net::Ipv6Addr>()
            .map_err(|_| anyhow::anyhow!("無効なIPv6アドレス: {}", ipv6_str))?;
        let port = if port_str.is_empty() {
            DEFAULT_PORT
        } else {
            port_str
                .parse::<u16>()
                .map_err(|_| anyhow::anyhow!("無効なポート番号: {}", port_str))?
        };

        return Ok(SocketAddr::from((ipv6, port)));
    }

    Err(anyhow::anyhow!("無効なIPv6アドレス形式: {}", addr))
}

/// Length-prefixed フレームの読み取り（4バイトBE長 + データ）
/// ストリームを消費せずに1フレームだけ読む
pub async fn read_frame(recv: &mut RecvStream) -> Result<bytes::Bytes> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .context("Failed to read frame length")?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_MESSAGE_SIZE {
        return Err(anyhow::anyhow!("Frame too large: {} bytes", len));
    }
    let mut data = vec![0u8; len];
    recv.read_exact(&mut data)
        .await
        .context("Failed to read frame data")?;
    Ok(bytes::Bytes::from(data))
}

/// Length-prefixed フレームの書き込み
pub async fn write_frame(send: &mut SendStream, data: &[u8]) -> Result<()> {
    let len = (data.len() as u32).to_be_bytes();
    send.write_all(&len)
        .await
        .context("Failed to write frame length")?;
    send.write_all(data)
        .await
        .context("Failed to write frame data")?;
    Ok(())
}

/// フレームタイプタグ
pub const FRAME_TYPE_PROTOCOL: u8 = 0x00;
pub const FRAME_TYPE_RAW: u8 = 0x01;

/// Typed フレーム — type tag 付きの読み書き
/// フォーマット: [4 bytes: length][1 byte: type tag][payload]
/// length は type tag + payload の合計バイト数
///
/// Typed フレームの読み取り — type tag とペイロードを返す
pub async fn read_typed_frame(recv: &mut RecvStream) -> Result<(u8, bytes::Bytes)> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .context("Failed to read frame length")?;
    let total_len = u32::from_be_bytes(len_buf) as usize;
    if total_len == 0 {
        return Err(anyhow::anyhow!("Empty frame"));
    }
    if total_len > MAX_MESSAGE_SIZE {
        return Err(anyhow::anyhow!("Frame too large: {} bytes", total_len));
    }

    // type tag を読む
    let mut type_buf = [0u8; 1];
    recv.read_exact(&mut type_buf)
        .await
        .context("Failed to read frame type tag")?;
    let frame_type = type_buf[0];

    // payload を読む
    let payload_len = total_len - 1;
    let mut data = vec![0u8; payload_len];
    recv.read_exact(&mut data)
        .await
        .context("Failed to read frame payload")?;
    Ok((frame_type, bytes::Bytes::from(data)))
}

/// Typed フレームの書き込み
pub async fn write_typed_frame(send: &mut SendStream, frame_type: u8, data: &[u8]) -> Result<()> {
    let total_len = (1 + data.len()) as u32;
    send.write_all(&total_len.to_be_bytes())
        .await
        .context("Failed to write frame length")?;
    send.write_all(&[frame_type])
        .await
        .context("Failed to write frame type tag")?;
    send.write_all(data)
        .await
        .context("Failed to write frame payload")?;
    Ok(())
}

/// Embedded certificates for development use
#[derive(RustEmbed)]
#[folder = "assets/certs"]
#[include = "*.pem"]
#[include = "*.der"]
struct EmbeddedCerts;

/// QUIC client implementation
pub struct QuicClient {
    endpoint: Mutex<Option<Endpoint>>,
    connection: Arc<RwLock<Option<Connection>>>,
    rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<ProtocolMessage>>>>,
    tx: mpsc::UnboundedSender<ProtocolMessage>,
    /// レスポンス受信タスクのハンドルを管理
    response_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl QuicClient {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        Ok(Self {
            endpoint: Mutex::new(None),
            connection: Arc::new(RwLock::new(None)),
            rx: Arc::new(RwLock::new(Some(rx))),
            tx,
            response_tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Configure client with custom TLS configuration
    pub async fn configure_client() -> Result<ClientConfig> {
        let client_crypto_config = RustlsClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        let crypto = quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto_config)?;
        let mut client_config = ClientConfig::new(Arc::new(crypto));

        // Configure QUIC transport parameters optimized for real-time communication
        let mut transport_config = quinn::TransportConfig::default();

        // Optimize for low latency
        transport_config
            .max_idle_timeout(Some(std::time::Duration::from_secs(60).try_into().unwrap()));
        transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(10)));

        // Enable 0-RTT for faster reconnection
        transport_config.max_concurrent_uni_streams(0u32.into()); // Unlimited unidirectional streams
        transport_config.max_concurrent_bidi_streams(1000u32.into()); // Support many bidirectional streams

        // Optimize congestion control for real-time data
        transport_config.initial_rtt(std::time::Duration::from_millis(100));

        client_config.transport_config(Arc::new(transport_config));

        Ok(client_config)
    }

    // 双方向ストリームを使うため、start_receive_loopは不要になりました

    /// QUIC接続への参照を取得（チャネル用ストリーム開設に使用）
    pub fn connection(&self) -> &Arc<RwLock<Option<Connection>>> {
        &self.connection
    }
}

impl QuicClient {
    /// IPv6専用でサーバーアドレスを解析
    fn parse_server_address(addr: &str) -> Result<SocketAddr> {
        parse_ipv6_address(addr)
    }

    pub async fn receive(&self) -> Result<ProtocolMessage> {
        let mut rx_guard = self.rx.write().await;
        if let Some(rx) = rx_guard.as_mut() {
            rx.recv()
                .await
                .context("Failed to receive message from channel")
        } else {
            Err(anyhow::anyhow!("Receiver not available"))
        }
    }

    pub async fn connect(&self, url: &str) -> Result<()> {
        // Parse URL (IPv6 only)
        let addr = Self::parse_server_address(url)?;

        let client_config = Self::configure_client().await?;

        // IPv6専用でバインド
        let bind_addr: SocketAddr = "[::]:0".parse().unwrap();

        let mut endpoint = Endpoint::client(bind_addr)?;
        endpoint.set_default_client_config(client_config);

        let connection = endpoint
            .connect(addr, "localhost")?
            .await
            .context("Failed to establish QUIC connection")?;

        info!("Connected to QUIC server at {} (IPv6)", addr);

        // Endpoint を保存（drop されると UDP ソケットが閉じて接続が切れる）
        *self.endpoint.lock().await = Some(endpoint);

        // accept_bi ループ用に connection をクローン
        let connection_for_loop = connection.clone();
        *self.connection.write().await = Some(connection);

        // サーバー発信ストリームを受け付けるバックグラウンドタスクを起動
        let tx = self.tx.clone();
        let task = tokio::spawn(async move {
            client_accept_bi_loop(connection_for_loop, tx).await;
        });
        self.response_tasks.lock().await.push(task);

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        // すべてのレスポンス受信タスクをキャンセル
        let mut tasks = self.response_tasks.lock().await;
        for task in tasks.drain(..) {
            task.abort();
        }

        // 接続をクローズ
        let mut connection_guard = self.connection.write().await;
        if let Some(connection) = connection_guard.take() {
            connection.close(quinn::VarInt::from_u32(0), b"client disconnect");
        }

        // Endpoint をクリーンアップ
        self.endpoint.lock().await.take();

        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        let connection_guard = self.connection.read().await;
        if let Some(connection) = connection_guard.as_ref() {
            connection.close_reason().is_none()
        } else {
            false
        }
    }
}

/// QUICサーバー実装
pub struct QuicServer {
    server: Arc<ProtocolServer>,
    endpoint: Option<Endpoint>,
}

impl QuicServer {
    pub fn new(server: Arc<ProtocolServer>) -> Self {
        Self {
            server,
            endpoint: None,
        }
    }

    /// QUIC/TLS 1.3用の自己署名証明書を生成（本番環境使用に最適化）
    pub fn generate_self_signed_cert()
    -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        let subject_alt_names = vec![
            "localhost".to_string(),
            "*.unison.svc.cluster.local".to_string(),
            "dev.chronista.club".to_string(),
        ];

        let cert_key = rcgen::generate_simple_self_signed(subject_alt_names)?;
        let cert_der_bytes = cert_key.cert.der().to_vec();
        let private_key_der_bytes = cert_key.key_pair.serialize_der();

        Ok((
            vec![CertificateDer::from(cert_der_bytes)],
            PrivateKeyDer::try_from(private_key_der_bytes).unwrap(),
        ))
    }

    /// 外部ファイルから証明書を読み込み（本番環境デプロイ用）
    pub fn load_cert_from_files(
        cert_path: &str,
        key_path: &str,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        let cert_pem = std::fs::read_to_string(cert_path)?;
        let key_der = std::fs::read(key_path)?;

        let cert_chain = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse certificate")?;
        let certs = cert_chain;

        // Convert to owned data for static lifetime
        let key_der_owned = key_der.clone();
        let private_key = PrivateKeyDer::try_from(key_der_owned.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;

        Ok((certs, private_key.clone_key()))
    }

    /// 埋め込みアセットから証明書を読み込み（rust-embed）
    pub fn load_cert_embedded() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        // Try to load embedded certificate files
        let cert_data = EmbeddedCerts::get("cert.pem")
            .ok_or_else(|| anyhow::anyhow!("Embedded cert.pem not found"))?;
        let key_data = EmbeddedCerts::get("private_key.der")
            .ok_or_else(|| anyhow::anyhow!("Embedded private_key.der not found"))?;

        // Parse certificate
        let cert_pem = std::str::from_utf8(&cert_data.data)?;
        let cert_chain = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse embedded certificate")?;
        let certs = cert_chain;

        // Load private key (already in DER format) - clone to own the data
        let key_data_owned = key_data.data.to_vec();
        let private_key = PrivateKeyDer::try_from(key_data_owned.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to parse embedded private key: {}", e))?;

        info!("Loaded embedded certificate from rust-embed");
        Ok((certs, private_key.clone_key()))
    }

    /// Automatically load certificate with fallback priority:
    /// 1. External files (assets/certs/)
    /// 2. Embedded certificates (rust-embed)
    /// 3. Generated self-signed certificate
    pub fn load_cert_auto() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        // Priority 1: External files
        if std::path::Path::new(DEFAULT_CERT_PATH).exists()
            && std::path::Path::new(DEFAULT_KEY_PATH).exists()
        {
            info!("Loading certificate from external files");
            return Self::load_cert_from_files(DEFAULT_CERT_PATH, DEFAULT_KEY_PATH);
        }

        // Priority 2: Embedded certificates (rust-embed使用)
        if let Ok(result) = Self::load_cert_embedded() {
            return Ok(result);
        }

        // Priority 3: Generate self-signed certificate
        info!("Generating self-signed certificate (no certificate files found)");
        Self::generate_self_signed_cert()
    }

    /// Configure server with TLS (using auto certificate detection)
    pub async fn configure_server() -> Result<ServerConfig> {
        let (certs, private_key) = Self::load_cert_auto()?;

        let rustls_server_config = RustlsServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, private_key)
            .map_err(|e| anyhow::anyhow!("Failed to configure TLS: {}", e))?;

        let crypto = quinn::crypto::rustls::QuicServerConfig::try_from(rustls_server_config)?;
        let mut server_config = ServerConfig::with_crypto(Arc::new(crypto));

        // Configure QUIC transport parameters optimized for real-time communication
        let mut transport_config = quinn::TransportConfig::default();

        // Optimize for low latency and high throughput
        transport_config
            .max_idle_timeout(Some(std::time::Duration::from_secs(60).try_into().unwrap()));
        transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(10)));

        // Support many concurrent streams for multiplexed communication
        transport_config.max_concurrent_uni_streams(0u32.into()); // Unlimited unidirectional streams
        transport_config.max_concurrent_bidi_streams(1000u32.into()); // Support many bidirectional streams

        // Optimize for protocol-level communication patterns
        transport_config.initial_rtt(std::time::Duration::from_millis(100));
        // Max UDP payload is handled automatically by QUIC

        server_config.transport_config(Arc::new(transport_config));

        Ok(server_config)
    }

    pub async fn bind(&mut self, addr: &str) -> Result<()> {
        // IPv6を優先的に使用し、IPv4もサポート
        let socket_addr = Self::parse_socket_addr(addr)?;

        let server_config = Self::configure_server().await?;
        let endpoint = Endpoint::server(server_config, socket_addr)?;

        info!("QUIC server bound to {} (IPv6)", socket_addr);
        self.endpoint = Some(endpoint);
        Ok(())
    }

    /// IPv6専用でソケットアドレスを解析
    fn parse_socket_addr(addr: &str) -> Result<SocketAddr> {
        parse_ipv6_address(addr)
    }

    /// バインド済みのローカルアドレスを取得
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.endpoint.as_ref().and_then(|ep| ep.local_addr().ok())
    }

    pub async fn start(&self) -> Result<()> {
        let endpoint = self
            .endpoint
            .as_ref()
            .context("Server not bound to an address")?;

        info!("QUIC server listening for connections");

        while let Some(connecting) = endpoint.accept().await {
            let connection = connecting.await?;
            let remote_addr = connection.remote_address();
            info!("New QUIC connection from: {}", remote_addr);

            let server = Arc::clone(&self.server);
            let ctx = Arc::new(ConnectionContext::new());
            tokio::spawn(async move {
                if let Err(e) = handle_connection(connection, server, ctx).await {
                    error!("Connection error: {}", e);
                }
            });
        }

        Ok(())
    }

    /// shutdown シグナルを受け付けるバージョンの start
    pub async fn start_with_shutdown(
        &self,
        mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<()> {
        let endpoint = self
            .endpoint
            .as_ref()
            .context("Server not bound to an address")?;

        info!("QUIC server listening for connections (with shutdown support)");

        loop {
            tokio::select! {
                connecting = endpoint.accept() => {
                    match connecting {
                        Some(connecting) => {
                            let connection = connecting.await?;
                            let remote_addr = connection.remote_address();
                            info!("New QUIC connection from: {}", remote_addr);

                            let server = Arc::clone(&self.server);
                            let ctx = Arc::new(ConnectionContext::new());
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(connection, server, ctx).await {
                                    error!("Connection error: {}", e);
                                }
                            });
                        }
                        None => {
                            info!("QUIC endpoint closed");
                            break;
                        }
                    }
                }
                _ = &mut shutdown_rx => {
                    info!("Shutdown signal received, stopping server");
                    endpoint.close(quinn::VarInt::from_u32(0), b"server shutdown");
                    break;
                }
            }
        }

        Ok(())
    }
}

/// クライアント側: サーバー発信の双方向ストリームを受け付けるループ
///
/// サーバーが `connection.open_bi()` で開いたストリーム（Identity 送信等）を
/// `accept_bi()` で受信し、ProtocolMessage に変換して tx チャネルに送る。
async fn client_accept_bi_loop(
    connection: Connection,
    tx: mpsc::UnboundedSender<ProtocolMessage>,
) {
    loop {
        match connection.accept_bi().await {
            Ok((_send_stream, mut recv_stream)) => {
                let tx = tx.clone();
                tokio::spawn(async move {
                    match read_typed_frame(&mut recv_stream).await {
                        Ok((FRAME_TYPE_PROTOCOL, frame_bytes)) => {
                            if let Ok(frame) = ProtocolFrame::from_bytes(&frame_bytes)
                                && let Ok(message) = ProtocolMessage::from_frame(&frame)
                            {
                                let _ = tx.send(message);
                            }
                        }
                        Ok((frame_type, _)) => {
                            warn!(
                                "Unexpected frame type in server-initiated stream: 0x{:02x}",
                                frame_type
                            );
                        }
                        Err(e) => {
                            warn!("Failed to read server-initiated stream: {}", e);
                        }
                    }
                });
            }
            Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                info!("Connection closed by server");
                break;
            }
            Err(e) => {
                warn!("Failed to accept server-initiated stream: {}", e);
                break;
            }
        }
    }
}

async fn handle_connection(
    connection: Connection,
    server: Arc<ProtocolServer>,
    ctx: Arc<ConnectionContext>,
) -> Result<()> {
    let remote_addr = connection.remote_address();

    // Identity Handshake: 接続直後にServerIdentityを送信
    let identity = server.build_identity().await;
    ctx.set_identity(identity.clone()).await;

    let identity_msg = identity.to_protocol_message();
    if let Ok(frame) = identity_msg.into_frame() {
        let frame_bytes = frame.to_bytes();
        match connection.open_bi().await {
            Ok((mut send_stream, _recv_stream)) => {
                if let Err(e) =
                    write_typed_frame(&mut send_stream, FRAME_TYPE_PROTOCOL, &frame_bytes).await
                {
                    warn!("Failed to send identity: {}", e);
                } else {
                    let _ = send_stream.finish();
                    info!("Identity sent to client");
                }
            }
            Err(e) => {
                warn!("Failed to open identity stream: {}", e);
            }
        }
    }

    // 接続イベントを送信
    server.emit_connection_event(super::server::ConnectionEvent::Connected {
        remote_addr,
        context: Arc::clone(&ctx),
    });

    loop {
        let connection_clone = connection.clone();
        match connection.accept_bi().await {
            Ok((send_stream, mut recv_stream)) => {
                let server = Arc::clone(&server);
                let connection = connection_clone;
                let ctx = Arc::clone(&ctx);

                tokio::spawn(async move {
                    // typed frame で読み取り（type tag 付き）
                    let request_result = match read_typed_frame(&mut recv_stream).await {
                        Ok((FRAME_TYPE_PROTOCOL, frame_bytes)) => {
                            ProtocolFrame::from_bytes(&frame_bytes)
                                .and_then(|frame| ProtocolMessage::from_frame(&frame))
                        }
                        Ok((frame_type, _)) => {
                            warn!("Unexpected frame type in handshake: 0x{:02x}", frame_type);
                            return;
                        }
                        Err(e) => {
                            error!("Failed to read handshake frame: {}", e);
                            return;
                        }
                    };

                    match request_result {
                        Ok(request) => {
                            // チャネルルーティング: __channel: プレフィックスをチェック
                            if let Some(channel_name) = request.method.strip_prefix("__channel:") {
                                let channel_name = channel_name.to_string();
                                if let Some(handler) =
                                    server.get_channel_handler(&channel_name).await
                                {
                                    // チャネル用のUnisonStreamを作成（ストリームは生きたまま）
                                    let stream = UnisonStream::from_streams(
                                        request.id,
                                        request.method.clone(),
                                        Arc::new(connection),
                                        send_stream,
                                        recv_stream,
                                    );
                                    if let Err(e) = handler(ctx, stream).await {
                                        error!(
                                            "Channel handler error for '{}': {}",
                                            channel_name, e
                                        );
                                    }
                                } else {
                                    warn!("No channel handler for: {}", channel_name);
                                }
                                return;
                            }

                            // 非チャネルメッセージはサポート外
                            warn!(
                                "Non-channel message received (method: {}). Use channels instead.",
                                request.method
                            );
                        }
                        Err(e) => {
                            warn!("Failed to parse message: {}", e);
                        }
                    }
                });
            }
            Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                info!("Client disconnected");
                server.emit_connection_event(super::server::ConnectionEvent::Disconnected {
                    remote_addr,
                });
                break;
            }
            Err(e) => {
                error!("Failed to accept stream: {}", e);
                server.emit_connection_event(super::server::ConnectionEvent::Disconnected {
                    remote_addr,
                });
                break;
            }
        }
    }

    Ok(())
}

/// 検証をスキップするカスタム証明書検証器（テスト専用）
#[derive(Debug)]
pub struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        use rustls::SignatureScheme;
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

/// Unison Stream - QUIC双方向ストリーム実装
pub struct UnisonStream {
    stream_id: u64,
    method: String,
    #[allow(dead_code)]
    connection: Arc<Connection>,
    send_stream: Arc<Mutex<Option<SendStream>>>,
    recv_stream: Arc<Mutex<Option<RecvStream>>>,
    is_active: Arc<AtomicBool>,
}

impl UnisonStream {
    pub async fn new(
        method: String,
        connection: Arc<Connection>,
        stream_id: Option<u64>,
    ) -> Result<Self> {
        static STREAM_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

        let id = stream_id.unwrap_or_else(|| STREAM_ID_COUNTER.fetch_add(1, Ordering::SeqCst));

        // Open bidirectional stream
        let (send_stream, recv_stream) = connection
            .open_bi()
            .await
            .context("Failed to open bidirectional stream")?;

        Ok(Self {
            stream_id: id,
            method,
            connection,
            send_stream: Arc::new(Mutex::new(Some(send_stream))),
            recv_stream: Arc::new(Mutex::new(Some(recv_stream))),
            is_active: Arc::new(AtomicBool::new(true)),
        })
    }

    /// 既存のストリームから作成（サーバー側）
    pub fn from_streams(
        stream_id: u64,
        method: String,
        connection: Arc<Connection>,
        send_stream: SendStream,
        recv_stream: RecvStream,
    ) -> Self {
        Self {
            stream_id,
            method,
            connection,
            send_stream: Arc::new(Mutex::new(Some(send_stream))),
            recv_stream: Arc::new(Mutex::new(Some(recv_stream))),
            is_active: Arc::new(AtomicBool::new(true)),
        }
    }

    /// ストリーム稼働状態の確認
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }
}

/// Typed フレーム受信結果
pub enum TypedFrame {
    /// ProtocolMessage フレーム (type tag 0x00)
    Protocol(ProtocolMessage),
    /// Raw bytes フレーム (type tag 0x01)
    Raw(Vec<u8>),
}

impl UnisonStream {
    /// ProtocolMessage を typed フレームとして送信（type tag 0x00）
    ///
    /// SystemStream::send() を経由せず、ProtocolMessage → into_frame() → write_typed_frame() で
    /// type tag 付き length-prefixed フレームとして送信する。チャネル通信で使用。
    pub async fn send_frame(&self, msg: &ProtocolMessage) -> Result<(), NetworkError> {
        if !self.is_active() {
            return Err(NetworkError::Connection("Stream is not active".to_string()));
        }

        let frame = msg.clone().into_frame()?;
        let frame_bytes = frame.to_bytes();

        let mut send_guard = self.send_stream.lock().await;
        if let Some(send_stream) = send_guard.as_mut() {
            write_typed_frame(send_stream, FRAME_TYPE_PROTOCOL, &frame_bytes)
                .await
                .map_err(|e| NetworkError::Quic(format!("Failed to send frame: {}", e)))?;
            Ok(())
        } else {
            Err(NetworkError::Connection(
                "Send stream is closed".to_string(),
            ))
        }
    }

    /// Raw bytes を typed フレームとして送信（type tag 0x01）
    ///
    /// rkyv/zstd をバイパスし、length-prefix + type tag + raw payload のみ。
    /// オーディオストリーミング等の最小オーバーヘッド通信に使用。
    pub async fn send_raw_frame(&self, data: &[u8]) -> Result<(), NetworkError> {
        if !self.is_active() {
            return Err(NetworkError::Connection("Stream is not active".to_string()));
        }

        let mut send_guard = self.send_stream.lock().await;
        if let Some(send_stream) = send_guard.as_mut() {
            write_typed_frame(send_stream, FRAME_TYPE_RAW, data)
                .await
                .map_err(|e| NetworkError::Quic(format!("Failed to send raw frame: {}", e)))?;
            Ok(())
        } else {
            Err(NetworkError::Connection(
                "Send stream is closed".to_string(),
            ))
        }
    }

    /// ストリームを閉じる（&self で呼べるバージョン、Arc 共有時に使用）
    pub async fn close_stream(&self) -> Result<(), NetworkError> {
        self.is_active.store(false, Ordering::SeqCst);

        if let Some(mut send_stream) = self.send_stream.lock().await.take() {
            send_stream
                .finish()
                .map_err(|e| NetworkError::Quic(format!("Failed to close send stream: {}", e)))?;
        }

        if let Some(mut recv_stream) = self.recv_stream.lock().await.take() {
            recv_stream.stop(quinn::VarInt::from_u32(0)).map_err(|e| {
                NetworkError::Quic(format!("Failed to close receive stream: {}", e))
            })?;
        }

        info!(
            "Stream {} closed for method '{}'",
            self.stream_id, self.method
        );
        Ok(())
    }

    /// ProtocolMessage のみを受信（後方互換）
    ///
    /// typed frame を読んで ProtocolMessage のみを返す。
    /// Raw bytes フレームが来た場合はエラーを返す。
    pub async fn recv_frame(&self) -> Result<ProtocolMessage, NetworkError> {
        match self.recv_typed_frame().await? {
            TypedFrame::Protocol(msg) => Ok(msg),
            TypedFrame::Raw(_) => Err(NetworkError::Protocol(
                "Expected protocol frame, got raw bytes".to_string(),
            )),
        }
    }

    /// Typed フレームを受信（ProtocolMessage or Raw bytes）
    ///
    /// type tag で振り分けて TypedFrame を返す。
    /// チャネルの recv ループで使用し、Protocol/Raw を適切なキューに振り分ける。
    pub async fn recv_typed_frame(&self) -> Result<TypedFrame, NetworkError> {
        if !self.is_active() {
            return Err(NetworkError::Connection("Stream is not active".to_string()));
        }

        let mut recv_guard = self.recv_stream.lock().await;
        if let Some(recv_stream) = recv_guard.as_mut() {
            let (frame_type, payload) = read_typed_frame(recv_stream).await.map_err(|e| {
                self.is_active.store(false, Ordering::SeqCst);
                NetworkError::Quic(format!("Failed to read frame: {}", e))
            })?;

            match frame_type {
                FRAME_TYPE_PROTOCOL => {
                    let frame = ProtocolFrame::from_bytes(&payload)?;
                    let message = ProtocolMessage::from_frame(&frame)?;
                    Ok(TypedFrame::Protocol(message))
                }
                FRAME_TYPE_RAW => Ok(TypedFrame::Raw(payload.to_vec())),
                _ => Err(NetworkError::Protocol(format!(
                    "Unknown frame type tag: 0x{:02x}",
                    frame_type
                ))),
            }
        } else {
            Err(NetworkError::Connection(
                "Receive stream is closed".to_string(),
            ))
        }
    }
}

