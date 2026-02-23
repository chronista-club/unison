mod common;

use unison::codegen::{CodeGenerator, RustGenerator};
use unison::parser::{SchemaParser, TypeRegistry};

/// schemas/ping_pong.kdl を読み込み → parse → generate Rust → 構造検証
#[test]
fn test_integ_ping_pong_full_pipeline() {
    let schema = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/ping_pong.kdl"
    ))
    .unwrap();

    let parser = SchemaParser::new();
    let parsed = parser.parse(&schema).unwrap();
    let type_registry = TypeRegistry::new();
    let generator = RustGenerator::new();
    let code = generator.generate(&parsed, &type_registry).unwrap();

    assert!(code.contains("Ping"), "Expected Ping struct in generated code");
    assert!(
        code.contains("Pong"),
        "Expected Pong struct in generated code"
    );
    assert!(
        code.contains("UnisonChannel"),
        "Expected UnisonChannel reference in generated code"
    );
}

/// schemas/creo_sync.kdl → parse → generate → 5チャネル検証
#[test]
fn test_integ_creo_sync_full_pipeline() {
    let schema = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/creo_sync.kdl"
    ))
    .unwrap();

    let parser = SchemaParser::new();
    let parsed = parser.parse(&schema).unwrap();
    let type_registry = TypeRegistry::new();
    let generator = RustGenerator::new();
    let code = generator.generate(&parsed, &type_registry).unwrap();

    // 5チャネルのメッセージ型が生成されていること
    assert!(
        code.contains("Subscribe"),
        "Expected Subscribe from control channel"
    );
    assert!(
        code.contains("MemoryEvent"),
        "Expected MemoryEvent from events channel"
    );
    assert!(
        code.contains("Query"),
        "Expected Query from query channel"
    );
    assert!(
        code.contains("SendMessage"),
        "Expected SendMessage from messaging channel"
    );
    assert!(
        code.contains("Alert"),
        "Expected Alert from urgent channel"
    );
}

/// request/returns 構文の codegen 検証
#[test]
fn test_integ_request_returns_codegen() {
    let schema = r#"
        protocol "test" version="1.0.0" {
            namespace "test"
            channel "api" from="client" lifetime="persistent" {
                request "GetUser" {
                    field "user_id" type="string" required=#true
                    returns "UserResponse" {
                        field "name" type="string"
                        field "email" type="string"
                    }
                }
            }
        }
    "#;

    let parser = SchemaParser::new();
    let parsed = parser.parse(schema).unwrap();
    let type_registry = TypeRegistry::new();
    let generator = RustGenerator::new();
    let code = generator.generate(&parsed, &type_registry).unwrap();

    assert!(code.contains("GetUser"), "Expected GetUser struct");
    assert!(
        code.contains("UserResponse"),
        "Expected UserResponse struct"
    );
    assert!(code.contains("user_id"), "Expected user_id field");
    assert!(code.contains("name"), "Expected name field");
}

/// event 構文の codegen 検証
#[test]
fn test_integ_event_codegen() {
    let schema = r#"
        protocol "test" version="1.0.0" {
            namespace "test"
            channel "notifications" from="server" lifetime="persistent" {
                event "UserJoined" {
                    field "user_name" type="string" required=#true
                    field "timestamp" type="string"
                }
                event "UserLeft" {
                    field "user_name" type="string" required=#true
                }
            }
        }
    "#;

    let parser = SchemaParser::new();
    let parsed = parser.parse(schema).unwrap();
    let type_registry = TypeRegistry::new();
    let generator = RustGenerator::new();
    let code = generator.generate(&parsed, &type_registry).unwrap();

    assert!(code.contains("UserJoined"), "Expected UserJoined struct");
    assert!(code.contains("UserLeft"), "Expected UserLeft struct");
    assert!(code.contains("user_name"), "Expected user_name field");
}

/// 不正KDLでのパースエラー伝播
#[test]
fn test_integ_invalid_kdl_parse_error() {
    let invalid_schema = r#"
        protocol "test" version="1.0.0" {
            this is not valid kdl {{{}}}
        }
    "#;

    let parser = SchemaParser::new();
    let result = parser.parse(invalid_schema);
    assert!(result.is_err());
}
