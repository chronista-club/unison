mod common;

use unison::network::{MessageType, ProtocolMessage};
use unison::packet::UnisonPacket;

/// Request型 ProtocolMessage の into_frame → to_bytes → from_bytes → from_frame 往復
#[test]
fn test_integ_request_round_trip() {
    let msg = ProtocolMessage::new_with_json(
        42,
        "test.method".to_string(),
        MessageType::Request,
        serde_json::json!({"key": "value"}),
    )
    .unwrap();

    let frame = msg.clone().into_frame().unwrap();
    let bytes = frame.to_bytes();

    let restored_frame = UnisonPacket::from_bytes(&bytes).unwrap();
    let restored = ProtocolMessage::from_frame(&restored_frame).unwrap();

    assert_eq!(restored.id, msg.id);
    assert_eq!(restored.method, msg.method);
    assert_eq!(restored.msg_type, msg.msg_type);
    assert_eq!(restored.payload, msg.payload);
}

/// Response型の往復
#[test]
fn test_integ_response_round_trip() {
    let msg = ProtocolMessage::new_with_json(
        99,
        "test.response".to_string(),
        MessageType::Response,
        serde_json::json!({"status": "ok", "data": [1, 2, 3]}),
    )
    .unwrap();

    let frame = msg.clone().into_frame().unwrap();
    let bytes = frame.to_bytes();

    let restored_frame = UnisonPacket::from_bytes(&bytes).unwrap();
    let restored = ProtocolMessage::from_frame(&restored_frame).unwrap();

    assert_eq!(restored.id, msg.id);
    assert_eq!(restored.method, msg.method);
    assert_eq!(restored.msg_type, msg.msg_type);
    assert_eq!(restored.payload, msg.payload);
}

/// Event型の往復
#[test]
fn test_integ_event_round_trip() {
    let msg = common::make_event(
        "system.notify",
        serde_json::json!({"level": "info", "msg": "hello"}),
    );

    let frame = msg.clone().into_frame().unwrap();
    let bytes = frame.to_bytes();

    let restored_frame = UnisonPacket::from_bytes(&bytes).unwrap();
    let restored = ProtocolMessage::from_frame(&restored_frame).unwrap();

    assert_eq!(restored.id, msg.id);
    assert_eq!(restored.method, msg.method);
    assert_eq!(restored.msg_type, msg.msg_type);
    assert_eq!(restored.payload, msg.payload);
}

/// Error型の往復
#[test]
fn test_integ_error_round_trip() {
    let msg = ProtocolMessage::new_with_json(
        7,
        "test.error".to_string(),
        MessageType::Error,
        serde_json::json!({"code": 404, "message": "not found"}),
    )
    .unwrap();

    let frame = msg.clone().into_frame().unwrap();
    let bytes = frame.to_bytes();

    let restored_frame = UnisonPacket::from_bytes(&bytes).unwrap();
    let restored = ProtocolMessage::from_frame(&restored_frame).unwrap();

    assert_eq!(restored.id, msg.id);
    assert_eq!(restored.method, msg.method);
    assert_eq!(restored.msg_type, msg.msg_type);
    assert_eq!(restored.payload, msg.payload);
}

/// ネストしたJSON、配列、Unicode文字列での往復
#[test]
fn test_integ_complex_json_round_trip() {
    let complex_payload = serde_json::json!({
        "user": {
            "name": "田中太郎",
            "tags": ["admin", "developer"],
            "profile": {
                "bio": "こんにちは世界 🌍",
                "scores": [100, 200, 300]
            }
        },
        "metadata": {
            "nested": {
                "deep": {
                    "value": true
                }
            }
        },
        "emoji": "🎉🚀✨",
        "empty_array": [],
        "null_field": null
    });

    let msg = common::make_request("complex.method", complex_payload);

    let frame = msg.clone().into_frame().unwrap();
    let bytes = frame.to_bytes();

    let restored_frame = UnisonPacket::from_bytes(&bytes).unwrap();
    let restored = ProtocolMessage::from_frame(&restored_frame).unwrap();

    assert_eq!(restored.id, msg.id);
    assert_eq!(restored.method, msg.method);
    assert_eq!(restored.msg_type, msg.msg_type);

    // JSON値として比較（キー順序に依存しない）
    let original_value: serde_json::Value = serde_json::from_str(&msg.payload).unwrap();
    let restored_value: serde_json::Value = serde_json::from_str(&restored.payload).unwrap();
    assert_eq!(original_value, restored_value);
}

/// 2KB超のペイロードが自動圧縮されてから往復
#[test]
fn test_integ_large_payload_compression_round_trip() {
    // 圧縮閾値（2048バイト）を超えるペイロードを生成
    let large_data: Vec<serde_json::Value> = (0..200)
        .map(|i| {
            serde_json::json!({
                "index": i,
                "value": format!("item_{:04}", i),
                "description": "This is a test entry for compression verification"
            })
        })
        .collect();

    let msg = common::make_request("bulk.data", serde_json::json!({"items": large_data}));

    // ペイロードが閾値を超えていることを確認
    assert!(
        msg.payload.len() > 2048,
        "payload should exceed compression threshold"
    );

    let frame = msg.clone().into_frame().unwrap();

    // 圧縮が適用されていることを確認
    let header = frame.header().unwrap();
    assert!(header.is_compressed(), "large payload should be compressed");

    let bytes = frame.to_bytes();

    let restored_frame = UnisonPacket::from_bytes(&bytes).unwrap();
    let restored = ProtocolMessage::from_frame(&restored_frame).unwrap();

    assert_eq!(restored.id, msg.id);
    assert_eq!(restored.method, msg.method);
    assert_eq!(restored.msg_type, msg.msg_type);

    let original_value: serde_json::Value = serde_json::from_str(&msg.payload).unwrap();
    let restored_value: serde_json::Value = serde_json::from_str(&restored.payload).unwrap();
    assert_eq!(original_value, restored_value);
}

/// 空ペイロード（`{}`）での往復
#[test]
fn test_integ_empty_payload_round_trip() {
    let msg = common::make_message("empty.method", MessageType::Request, serde_json::json!({}));

    let frame = msg.clone().into_frame().unwrap();
    let bytes = frame.to_bytes();

    let restored_frame = UnisonPacket::from_bytes(&bytes).unwrap();
    let restored = ProtocolMessage::from_frame(&restored_frame).unwrap();

    assert_eq!(restored.id, msg.id);
    assert_eq!(restored.method, msg.method);
    assert_eq!(restored.msg_type, msg.msg_type);
    assert_eq!(restored.payload, msg.payload);
}
