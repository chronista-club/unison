use super::TypeRegistry;
use std::collections::HashMap;

/// Parsed schema representation
#[derive(Debug, Default, Clone, knuffel::Decode)]
pub struct ParsedSchema {
    #[knuffel(child)]
    pub protocol: Option<Protocol>,

    #[knuffel(children(name = "import"))]
    pub imports: Vec<Import>,

    #[knuffel(children(name = "message"))]
    pub messages: Vec<Message>,

    #[knuffel(children(name = "enum"))]
    pub enums: Vec<Enum>,

    #[knuffel(children(name = "typedef"))]
    pub typedefs: Vec<TypeDef>,
}

/// Import definition
#[derive(Debug, Clone, knuffel::Decode)]
pub struct Import {
    #[knuffel(argument)]
    pub path: String,
}

/// Protocol definition
#[derive(Debug, Clone, knuffel::Decode)]
pub struct Protocol {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(property)]
    pub version: String,

    #[knuffel(child, unwrap(argument))]
    pub namespace: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub description: Option<String>,

    #[knuffel(children(name = "service"))]
    pub services: Vec<Service>,

    #[knuffel(children(name = "message"))]
    pub messages: Vec<Message>,

    #[knuffel(children(name = "enum"))]
    pub enums: Vec<Enum>,

    #[knuffel(children(name = "channel"))]
    pub channels: Vec<Channel>,
}

/// Channel開始者
#[derive(Debug, Clone, PartialEq, knuffel::DecodeScalar)]
pub enum ChannelFrom {
    #[knuffel(rename = "client")]
    Client,
    #[knuffel(rename = "server")]
    Server,
    #[knuffel(rename = "either")]
    Either,
}

/// Channelの寿命
#[derive(Debug, Clone, PartialEq, knuffel::DecodeScalar)]
pub enum ChannelLifetime {
    #[knuffel(rename = "transient")]
    Transient,
    #[knuffel(rename = "persistent")]
    Persistent,
}

/// Channel内のメッセージ定義（名前付き）
#[derive(Debug, Clone, knuffel::Decode)]
pub struct ChannelMessage {
    /// メッセージ名
    #[knuffel(argument)]
    pub name: String,

    /// フィールド定義
    #[knuffel(children(name = "field"))]
    pub fields: Vec<Field>,
}

/// Channel定義（Stream-First APIのプリミティブ）
#[derive(Debug, Clone, knuffel::Decode)]
pub struct Channel {
    /// チャネル名
    #[knuffel(argument)]
    pub name: String,

    /// 誰がStreamを開くか
    #[knuffel(property)]
    pub from: ChannelFrom,

    /// Streamの寿命
    #[knuffel(property)]
    pub lifetime: ChannelLifetime,

    /// 送信メッセージ型（opener視点）
    #[knuffel(child)]
    pub send: Option<ChannelMessage>,

    /// 受信メッセージ型（opener視点）
    #[knuffel(child)]
    pub recv: Option<ChannelMessage>,

    /// エラー型
    #[knuffel(child)]
    pub error: Option<ChannelMessage>,
}

/// Service definition
#[derive(Debug, Clone, knuffel::Decode)]
pub struct Service {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(child, unwrap(argument))]
    pub description: Option<String>,

    #[knuffel(children(name = "method"))]
    pub methods: Vec<Method>,

    #[knuffel(children(name = "stream"))]
    pub streams: Vec<Stream>,
}

/// RPC Method definition
#[derive(Debug, Clone, knuffel::Decode)]
pub struct Method {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(child, unwrap(argument))]
    pub description: Option<String>,

    #[knuffel(child)]
    pub request: Option<MethodMessage>,

    #[knuffel(child)]
    pub response: Option<MethodMessage>,
}

/// Method request/response definition (without name argument)
#[derive(Debug, Clone, knuffel::Decode)]
pub struct MethodMessage {
    #[knuffel(children(name = "field"))]
    pub fields: Vec<Field>,
}

/// Streaming endpoint definition
#[derive(Debug, Clone, knuffel::Decode)]
pub struct Stream {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(child)]
    pub request: Option<MethodMessage>,

    #[knuffel(child)]
    pub response: Option<MethodMessage>,
}

/// Message/struct definition
#[derive(Debug, Clone, knuffel::Decode)]
pub struct Message {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(child, unwrap(argument))]
    pub description: Option<String>,

    #[knuffel(children(name = "field"))]
    pub fields: Vec<Field>,
}

/// Field definition (KDL representation)
#[derive(Debug, Clone, knuffel::Decode)]
pub struct Field {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(property(name = "type"))]
    pub field_type_str: String,

    #[knuffel(property, default = false)]
    pub required: bool,

    #[knuffel(property(name = "default"))]
    pub default_str: Option<String>,

    #[knuffel(property)]
    pub min: Option<i64>,

    #[knuffel(property)]
    pub max: Option<i64>,

    #[knuffel(property)]
    pub min_length: Option<usize>,

    #[knuffel(property)]
    pub max_length: Option<usize>,

    #[knuffel(property)]
    pub pattern: Option<String>,

    #[knuffel(property)]
    pub description: Option<String>,
}

impl Field {
    /// フィールド型を取得
    pub fn field_type(&self) -> FieldType {
        self.parse_field_type(&self.field_type_str)
    }

    /// デフォルト値を取得
    pub fn default(&self) -> Option<DefaultValue> {
        self.default_str
            .as_ref()
            .and_then(|s| self.parse_default(s))
    }

    /// 制約を取得
    pub fn constraints(&self) -> Constraints {
        Constraints {
            min: self.min,
            max: self.max,
            min_length: self.min_length,
            max_length: self.max_length,
            pattern: self.pattern.clone(),
        }
    }

    fn parse_field_type(&self, type_str: &str) -> FieldType {
        match type_str {
            "string" => FieldType::String,
            "int" => FieldType::Int,
            "float" => FieldType::Float,
            "bool" => FieldType::Bool,
            "json" => FieldType::Json,
            "object" => FieldType::Object,
            _ => FieldType::Custom(type_str.to_string()),
        }
    }

    fn parse_default(&self, s: &str) -> Option<DefaultValue> {
        // 簡易的なパース実装
        if s == "null" {
            Some(DefaultValue::Null)
        } else if s == "true" {
            Some(DefaultValue::Bool(true))
        } else if s == "false" {
            Some(DefaultValue::Bool(false))
        } else if let Ok(i) = s.parse::<i64>() {
            Some(DefaultValue::Int(i))
        } else if let Ok(f) = s.parse::<f64>() {
            Some(DefaultValue::Float(f))
        } else {
            Some(DefaultValue::String(s.to_string()))
        }
    }
}

/// Field type
#[derive(Debug, Clone)]
pub enum FieldType {
    String,
    Int,
    Float,
    Bool,
    Json,
    Array(Box<FieldType>),
    Map(Box<FieldType>, Box<FieldType>),
    Enum(Vec<String>),
    Object,
    Custom(String),
}

/// Enum definition
#[derive(Debug, Clone, knuffel::Decode)]
pub struct Enum {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(child, unwrap(arguments))]
    pub values: Vec<String>,
}

/// Type definition
#[derive(Debug, Clone, knuffel::Decode)]
pub struct TypeDef {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(child, unwrap(argument))]
    pub base_type: String,

    #[knuffel(child, unwrap(argument))]
    pub rust_type: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub typescript_type: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub format: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub pattern: Option<String>,
}

/// Default value for fields
#[derive(Debug, Clone)]
pub enum DefaultValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<DefaultValue>),
    Object(HashMap<String, DefaultValue>),
    Null,
}

/// Field constraints
#[derive(Debug, Clone, Default)]
pub struct Constraints {
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
}

impl FieldType {
    /// Get the Rust type representation
    pub fn to_rust_type(&self, type_registry: &TypeRegistry) -> String {
        match self {
            FieldType::String => "String".to_string(),
            FieldType::Int => "i64".to_string(),
            FieldType::Float => "f64".to_string(),
            FieldType::Bool => "bool".to_string(),
            FieldType::Json => "serde_json::Value".to_string(),
            FieldType::Array(inner) => format!("Vec<{}>", inner.to_rust_type(type_registry)),
            FieldType::Map(key, value) => format!(
                "HashMap<{}, {}>",
                key.to_rust_type(type_registry),
                value.to_rust_type(type_registry)
            ),
            FieldType::Enum(_values) => {
                // This should be resolved to the actual enum name
                "String".to_string()
            }
            FieldType::Object => "serde_json::Value".to_string(),
            FieldType::Custom(name) => type_registry
                .get_rust_type(name)
                .unwrap_or_else(|| name.clone()),
        }
    }

    /// Get the TypeScript type representation
    pub fn to_typescript_type(&self, type_registry: &TypeRegistry) -> String {
        match self {
            FieldType::String => "string".to_string(),
            FieldType::Int | FieldType::Float => "number".to_string(),
            FieldType::Bool => "boolean".to_string(),
            FieldType::Json | FieldType::Object => "any".to_string(),
            FieldType::Array(inner) => format!("{}[]", inner.to_typescript_type(type_registry)),
            FieldType::Map(_, value) => format!(
                "Record<string, {}>",
                value.to_typescript_type(type_registry)
            ),
            FieldType::Enum(values) => values
                .iter()
                .map(|v| format!("'{}'", v))
                .collect::<Vec<_>>()
                .join(" | "),
            FieldType::Custom(name) => type_registry
                .get_typescript_type(name)
                .unwrap_or_else(|| name.clone()),
        }
    }
}
