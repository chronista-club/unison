//! Codec: アプリケーションメッセージのシリアライゼーション抽象化
//!
//! ## 概要
//!
//! `Codec` マーカートレイトと `Encodable<C>` / `Decodable<C>` トレイトペアで、
//! JSON / protobuf (buffa) 等のフォーマットを型安全に差し替え可能にする。
//!
//! ## 使い方
//!
//! ```rust,ignore
//! // JSON (デフォルト) — serde::Serialize/Deserialize な型はすべて使える
//! let channel: UnisonChannel<JsonCodec> = ...;
//! let resp: MyResponse = channel.request("method", &my_request).await?;
//!
//! // Protobuf — buffa::Message な型はすべて使える
//! let channel: UnisonChannel<ProtoCodec> = ...;
//! let resp: proto::Ack = channel.request("subscribe", &proto::Subscribe { ... }).await?;
//! ```

use thiserror::Error;

/// buffa 生成型（.proto → Rust）
pub mod proto {
    pub mod creo_sync {
        include!(concat!(env!("OUT_DIR"), "/creo_sync.rs"));
    }
}

/// Codec エラー型
#[derive(Error, Debug)]
pub enum CodecError {
    #[error("Encode error: {0}")]
    Encode(String),
    #[error("Decode error: {0}")]
    Decode(String),
}

/// Codec マーカートレイト
///
/// `UnisonChannel<C: Codec>` の型パラメータとして使用。
/// 実際の encode/decode は `Encodable<C>` / `Decodable<C>` が担う。
pub trait Codec: Send + Sync + 'static {}

/// `C: Codec` に対して、自身をバイト列にエンコードできることを表す
pub trait Encodable<C: ?Sized> {
    fn encode(&self) -> Result<Vec<u8>, CodecError>;
}

/// `C: Codec` に対して、バイト列から自身をデコードできることを表す
pub trait Decodable<C: ?Sized>: Sized {
    fn decode(bytes: &[u8]) -> Result<Self, CodecError>;
}

// ============================================================
// JsonCodec
// ============================================================

/// JSON Codec — serde ベースのシリアライゼーション
pub struct JsonCodec;

impl Codec for JsonCodec {}

impl<T: serde::Serialize> Encodable<JsonCodec> for T {
    fn encode(&self) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(self).map_err(|e| CodecError::Encode(e.to_string()))
    }
}

impl<T: serde::de::DeserializeOwned> Decodable<JsonCodec> for T {
    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        serde_json::from_slice(bytes).map_err(|e| CodecError::Decode(e.to_string()))
    }
}

// ============================================================
// ProtoCodec
// ============================================================

/// Protobuf Codec — buffa ベースのシリアライゼーション
pub struct ProtoCodec;

impl Codec for ProtoCodec {}

impl<T: buffa::Message> Encodable<ProtoCodec> for T {
    fn encode(&self) -> Result<Vec<u8>, CodecError> {
        Ok(self.encode_to_vec())
    }
}

impl<T: buffa::Message + Default> Decodable<ProtoCodec> for T {
    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        T::decode_from_slice(bytes).map_err(|e| CodecError::Decode(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_codec_value_roundtrip() {
        let value = serde_json::json!({
            "name": "test",
            "count": 42,
            "nested": { "items": [1, 2, 3] }
        });

        let encoded = Encodable::<JsonCodec>::encode(&value).unwrap();
        let decoded: serde_json::Value = Decodable::<JsonCodec>::decode(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_json_codec_typed_roundtrip() {
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct Ping {
            message: String,
        }

        let ping = Ping {
            message: "hello".into(),
        };

        let encoded = Encodable::<JsonCodec>::encode(&ping).unwrap();
        let decoded: Ping = Decodable::<JsonCodec>::decode(&encoded).unwrap();
        assert_eq!(ping, decoded);
    }

    #[test]
    fn test_json_codec_decode_error() {
        let result = <serde_json::Value as Decodable<JsonCodec>>::decode(b"not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_proto_codec_roundtrip() {
        use proto::creo_sync::Subscribe;

        let msg = Subscribe {
            category: "test".into(),
            tags: "a,b".into(),
            ..Default::default()
        };

        let encoded = Encodable::<ProtoCodec>::encode(&msg).unwrap();
        let decoded: Subscribe = Decodable::<ProtoCodec>::decode(&encoded).unwrap();
        assert_eq!(decoded.category, "test");
        assert_eq!(decoded.tags, "a,b");
    }

    #[test]
    fn test_proto_codec_decode_error() {
        use proto::creo_sync::Subscribe;

        // 不正なバイト列
        let result = <Subscribe as Decodable<ProtoCodec>>::decode(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn test_proto_codec_large_message() {
        use proto::creo_sync::MemoryEvent;

        let msg = MemoryEvent {
            event_type: "created".into(),
            memory_id: "mem_abc123".into(),
            category: "x".repeat(10_000),
            from: "creo-lead".into(),
            timestamp: "2026-03-28T12:00:00Z".into(),
            ..Default::default()
        };

        let encoded = Encodable::<ProtoCodec>::encode(&msg).unwrap();
        let decoded: MemoryEvent = Decodable::<ProtoCodec>::decode(&encoded).unwrap();
        assert_eq!(decoded.category.len(), 10_000);
        assert_eq!(decoded.event_type, "created");
    }

    #[test]
    fn test_proto_codec_empty_message() {
        use proto::creo_sync::Subscribe;

        // デフォルト（全フィールド空文字列）のメッセージ
        let msg = Subscribe::default();
        let encoded = Encodable::<ProtoCodec>::encode(&msg).unwrap();
        // proto3 のデフォルト値はゼロバイトにエンコードされる
        assert!(encoded.is_empty());
        let decoded: Subscribe = Decodable::<ProtoCodec>::decode(&encoded).unwrap();
        assert_eq!(decoded.category, "");
    }

    #[test]
    fn test_json_and_proto_encode_different_bytes() {
        // 同じ論理データでも JsonCodec と ProtoCodec では異なるバイト列になる
        use proto::creo_sync::Ack;

        let ack = Ack {
            status: "ok".into(),
            channel_ref: "ch-1".into(),
            ..Default::default()
        };

        let proto_bytes = Encodable::<ProtoCodec>::encode(&ack).unwrap();

        let json_value = serde_json::json!({"status": "ok", "channel_ref": "ch-1"});
        let json_bytes = Encodable::<JsonCodec>::encode(&json_value).unwrap();

        // 異なるフォーマットなのでバイト列は一致しない
        assert_ne!(proto_bytes, json_bytes);
        // protobuf のほうがコンパクト
        assert!(proto_bytes.len() < json_bytes.len());
    }

    #[test]
    fn test_json_codec_empty_bytes_decode_error() {
        let result = <serde_json::Value as Decodable<JsonCodec>>::decode(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_codec_null_roundtrip() {
        let value = serde_json::Value::Null;
        let encoded = Encodable::<JsonCodec>::encode(&value).unwrap();
        let decoded: serde_json::Value = Decodable::<JsonCodec>::decode(&encoded).unwrap();
        assert_eq!(decoded, serde_json::Value::Null);
    }

    #[test]
    fn test_proto_codec_truncated_bytes_decode_error() {
        use proto::creo_sync::Subscribe;

        let msg = Subscribe {
            category: "test-category".into(),
            tags: "a,b,c".into(),
            ..Default::default()
        };
        let encoded = Encodable::<ProtoCodec>::encode(&msg).unwrap();

        // 末尾を切り落とした truncated バイト列
        let truncated = &encoded[..encoded.len() / 2];
        let result = <Subscribe as Decodable<ProtoCodec>>::decode(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn test_proto_codec_all_fields_roundtrip() {
        use proto::creo_sync::MemoryEvent;

        let msg = MemoryEvent {
            event_type: "updated".into(),
            memory_id: "mem_xyz789".into(),
            category: "architecture".into(),
            from: "worker-1".into(),
            timestamp: "2026-03-28T15:30:00Z".into(),
            ..Default::default()
        };

        let encoded = Encodable::<ProtoCodec>::encode(&msg).unwrap();
        let decoded: MemoryEvent = Decodable::<ProtoCodec>::decode(&encoded).unwrap();
        assert_eq!(decoded.event_type, "updated");
        assert_eq!(decoded.memory_id, "mem_xyz789");
        assert_eq!(decoded.category, "architecture");
        assert_eq!(decoded.from, "worker-1");
        assert_eq!(decoded.timestamp, "2026-03-28T15:30:00Z");
    }

    #[test]
    fn test_codec_error_display() {
        let enc_err = CodecError::Encode("test encode error".to_string());
        assert!(enc_err.to_string().contains("test encode error"));

        let dec_err = CodecError::Decode("test decode error".to_string());
        assert!(dec_err.to_string().contains("test decode error"));
    }
}
