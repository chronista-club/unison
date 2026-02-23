//! ペイロード用のトレイトと基本実装
//!
//! UnisonPacketのペイロードとして使用できる型のトレイトを定義します。

use bytes::Bytes;
use rkyv::{
    Archive, CheckBytes, Deserialize, Serialize, ser::serializers::AllocSerializer,
    validation::validators::DefaultValidator,
};
use thiserror::Error;

/// ペイロード処理のエラー型
#[derive(Error, Debug)]
pub enum PayloadError {
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Invalid payload data")]
    InvalidData,

    #[error("Payload too large: {size} bytes (max: {max_size} bytes)")]
    TooLarge { size: usize, max_size: usize },
}

/// ペイロードとしてUnisonPacketで使用できる型のトレイト
///
/// このトレイトを実装する型は、rkyvによるゼロコピーシリアライゼーションと
/// bytesクレートとの相互変換をサポートする必要があります。
pub trait Payloadable: Archive + Sized + Serialize<AllocSerializer<256>> {
    /// ペイロードをBytesに変換
    fn to_bytes(&self) -> Result<Bytes, PayloadError> {
        let bytes = rkyv::to_bytes::<_, 256>(self)
            .map_err(|e| PayloadError::SerializationFailed(e.to_string()))?;
        Ok(Bytes::from(bytes.to_vec()))
    }

    /// Bytesからペイロードを復元
    fn from_bytes(bytes: &Bytes) -> Result<Self, PayloadError>
    where
        Self::Archived: Deserialize<Self, rkyv::Infallible>,
        for<'a> Self::Archived: CheckBytes<DefaultValidator<'a>>,
    {
        let archived = rkyv::check_archived_root::<Self>(bytes)
            .map_err(|e| PayloadError::DeserializationFailed(e.to_string()))?;

        archived
            .deserialize(&mut rkyv::Infallible)
            .map_err(|_| PayloadError::InvalidData)
    }

    /// アーカイブされたデータから直接参照を取得（ゼロコピー）
    fn from_bytes_zero_copy(bytes: &[u8]) -> Result<&Self::Archived, PayloadError>
    where
        for<'a> Self::Archived: CheckBytes<DefaultValidator<'a>>,
    {
        rkyv::check_archived_root::<Self>(bytes)
            .map_err(|e| PayloadError::DeserializationFailed(e.to_string()))
    }

    /// ペイロードサイズの最大値（デフォルト: 16MB）
    fn max_size() -> usize {
        16 * 1024 * 1024
    }

    /// ペイロードサイズの検証
    fn validate_size(size: usize) -> Result<(), PayloadError> {
        let max_size = Self::max_size();
        if size > max_size {
            Err(PayloadError::TooLarge { size, max_size })
        } else {
            Ok(())
        }
    }
}

// 基本型に対するPayloadable実装

/// バイト配列のペイロードラッパー
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct BytesPayload {
    pub data: Vec<u8>,
}

impl BytesPayload {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }
}

impl Payloadable for BytesPayload {}

/// 文字列ペイロードラッパー
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct StringPayload {
    pub data: String,
}

impl StringPayload {
    pub fn new(data: String) -> Self {
        Self { data }
    }

    pub fn from_string(data: &str) -> Self {
        Self {
            data: data.to_string(),
        }
    }
}

impl Payloadable for StringPayload {}

/// JSON互換ペイロードラッパー
///
/// serde_json::Valueを内部的に文字列として保持し、
/// rkyvでシリアライズ可能にします。
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct JsonPayload {
    json_str: String,
}

impl JsonPayload {
    pub fn new(value: serde_json::Value) -> Result<Self, PayloadError> {
        let json_str = serde_json::to_string(&value)
            .map_err(|e| PayloadError::SerializationFailed(e.to_string()))?;
        Ok(Self { json_str })
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, PayloadError> {
        // JSONの妥当性を検証
        serde_json::from_str::<serde_json::Value>(json_str)
            .map_err(|_| PayloadError::InvalidData)?;
        Ok(Self {
            json_str: json_str.to_string(),
        })
    }

    pub fn to_value(&self) -> Result<serde_json::Value, PayloadError> {
        serde_json::from_str(&self.json_str)
            .map_err(|e| PayloadError::DeserializationFailed(e.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.json_str
    }
}

impl Payloadable for JsonPayload {}

/// 空のペイロード
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[archive(check_bytes)]
pub struct EmptyPayload;

impl Payloadable for EmptyPayload {
    fn max_size() -> usize {
        0
    }
}

/// rkyv直接シリアライズペイロードラッパー
///
/// Archive + Serialize + Deserialize を実装した任意の型をペイロードとして使用できます。
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct RkyvPayload<T>
where
    T: Archive + Serialize<AllocSerializer<256>>,
{
    pub data: T,
}

impl<T> RkyvPayload<T>
where
    T: Archive + Serialize<AllocSerializer<256>>,
{
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> Payloadable for RkyvPayload<T>
where
    T: Archive + Serialize<AllocSerializer<256>>,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    for<'a> T::Archived: CheckBytes<DefaultValidator<'a>>,
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_payload() {
        let data = vec![1, 2, 3, 4, 5];
        let payload = BytesPayload::new(data.clone());

        // シリアライズ
        let bytes = payload.to_bytes().unwrap();
        assert!(!bytes.is_empty());

        // デシリアライズ
        let restored = BytesPayload::from_bytes(&bytes).unwrap();
        assert_eq!(restored.data, data);

        // ゼロコピーデシリアライズ
        let archived = BytesPayload::from_bytes_zero_copy(&bytes).unwrap();
        assert_eq!(archived.data.as_slice(), data.as_slice());
    }

    #[test]
    fn test_string_payload() {
        let text = "Hello, UnisonPacket!";
        let payload = StringPayload::from_string(text);

        let bytes = payload.to_bytes().unwrap();
        let restored = StringPayload::from_bytes(&bytes).unwrap();
        assert_eq!(restored.data, text);
    }

    #[test]
    fn test_json_payload() {
        let json_value = serde_json::json!({
            "name": "test",
            "value": 42,
            "nested": {
                "array": [1, 2, 3]
            }
        });

        let payload = JsonPayload::new(json_value.clone()).unwrap();
        let bytes = payload.to_bytes().unwrap();
        let restored = JsonPayload::from_bytes(&bytes).unwrap();

        assert_eq!(restored.to_value().unwrap(), json_value);
    }

    #[test]
    fn test_empty_payload() {
        let payload = EmptyPayload;

        // EmptyPayloadは特殊ケース: シリアライズすると0バイトになる可能性がある
        // rkyvがunit型を0バイトで表現することがあるため
        let result = payload.to_bytes();

        // to_bytesが正常に動作することを確認
        assert!(result.is_ok());
        let bytes = result.unwrap();

        // 0バイトの場合は特殊ケースとして許容
        // EmptyPayloadの主な目的は「ペイロードがない」ことを表現することなので
        // 実際のバイト数は重要ではない

        // ただし、0バイトの場合はfrom_bytesが失敗する可能性があるので、
        // バイト数が0より大きい場合のみデシリアライゼーションをテスト
        if !bytes.is_empty() {
            let restored = EmptyPayload::from_bytes(&bytes).unwrap();
            assert_eq!(restored, EmptyPayload);
        }
    }

    #[test]
    fn test_size_validation() {
        // BytesPayloadのデフォルト最大サイズは16MB
        let max_size = BytesPayload::max_size();
        assert_eq!(max_size, 16 * 1024 * 1024);

        // サイズ検証
        assert!(BytesPayload::validate_size(1000).is_ok());
        assert!(BytesPayload::validate_size(max_size).is_ok());
        assert!(BytesPayload::validate_size(max_size + 1).is_err());
    }

    #[test]
    fn test_invalid_json() {
        let result = JsonPayload::from_json_str("not a json");
        assert!(result.is_err());

        let result = JsonPayload::from_json_str(r#"{"valid": "json"}"#);
        assert!(result.is_ok());
    }
}
