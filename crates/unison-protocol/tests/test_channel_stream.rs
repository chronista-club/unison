use tokio::sync::mpsc;

#[tokio::test]
async fn test_stream_sender_receiver() {
    let (tx, mut rx) = mpsc::channel::<String>(32);

    let sender = unison::network::channel::StreamSender::new(tx);
    sender.send("hello".to_string()).await.unwrap();
    sender.send("world".to_string()).await.unwrap();

    let msg1 = rx.recv().await.unwrap();
    assert_eq!(msg1, "hello");
    let msg2 = rx.recv().await.unwrap();
    assert_eq!(msg2, "world");
}

#[tokio::test]
async fn test_bidirectional_channel() {
    use unison::network::channel::BidirectionalChannel;

    let (client_tx, server_rx) = mpsc::channel::<String>(32);
    let (server_tx, client_rx) = mpsc::channel::<String>(32);

    let mut client = BidirectionalChannel {
        sender: unison::network::channel::StreamSender::new(client_tx),
        receiver: unison::network::channel::StreamReceiver::new(client_rx),
    };

    let mut server = BidirectionalChannel {
        sender: unison::network::channel::StreamSender::new(server_tx),
        receiver: unison::network::channel::StreamReceiver::new(server_rx),
    };

    // クライアント → サーバー
    client.sender.send("ping".to_string()).await.unwrap();
    let msg = server.receiver.recv().await.unwrap();
    assert_eq!(msg, "ping");

    // サーバー → クライアント
    server.sender.send("pong".to_string()).await.unwrap();
    let msg = client.receiver.recv().await.unwrap();
    assert_eq!(msg, "pong");
}
