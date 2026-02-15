//! E2Eテスト: creo_syncスキーマから生成されるConnectionBuilder の検証
//!
//! チャネル型の生成（QuicBackedChannel, ConnectionBuilder）を検証。

use unison::codegen::CodeGenerator;
use unison::parser::TypeRegistry;
use unison::prelude::*;

#[test]
fn test_e2e_connection_builder_generation() {
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

    // QuicConnection構造体が生成されること
    assert!(
        code.contains("QuicConnection"),
        "Expected QuicConnection struct in generated code"
    );

    // ConnectionBuilderトレイトが生成されること
    assert!(
        code.contains("ConnectionBuilder"),
        "Expected ConnectionBuilder trait in generated code"
    );

    // build()メソッドが生成されること
    assert!(
        code.contains("build"),
        "Expected build() method in generated code"
    );

    // open_channelが使われること
    assert!(
        code.contains("open_channel"),
        "Expected open_channel calls in generated code"
    );

    // 5つのチャネルすべてがQuicConnection内にあること
    for channel in &["control", "events", "query", "messaging", "urgent"] {
        assert!(
            code.contains(channel),
            "Expected '{}' channel in QuicConnection",
            channel
        );
    }

    // QuicBackedChannel型が使われること
    assert!(
        code.contains("QuicBackedChannel"),
        "Expected QuicBackedChannel type in generated code"
    );
}

#[test]
fn test_e2e_channel_type_mapping() {
    let schema = r#"
        protocol "test-sync" version="1.0.0" {
            namespace "test.sync"

            channel "control" from="client" lifetime="persistent" {
                send "ControlCmd" {
                    field "action" type="string" required=#true
                }
                recv "ControlAck" {
                    field "status" type="string" required=#true
                }
            }

            channel "events" from="server" lifetime="persistent" {
                send "Event" {
                    field "event_type" type="string" required=#true
                    field "data" type="json"
                }
            }

            channel "query" from="client" lifetime="transient" {
                send "QueryRequest" {
                    field "method" type="string" required=#true
                }
                recv "QueryResponse" {
                    field "result" type="json"
                }
            }
        }
    "#;

    let parser = SchemaParser::new();
    let parsed = parser.parse(schema).unwrap();
    let type_registry = TypeRegistry::new();
    let generator = RustGenerator::new();
    let code = generator.generate(&parsed, &type_registry).unwrap();

    // インメモリConnection: 各チャネル型が正しいこと
    // control: persistent + send+recv → BidirectionalChannel
    assert!(
        code.contains("BidirectionalChannel"),
        "Expected BidirectionalChannel for control channel"
    );

    // events: server push → ReceiveChannel
    assert!(
        code.contains("ReceiveChannel"),
        "Expected ReceiveChannel for events channel"
    );

    // query: transient + send+recv → RequestChannel
    assert!(
        code.contains("RequestChannel"),
        "Expected RequestChannel for query channel"
    );

    // QuicConnection: 全チャネルがQuicBackedChannel
    assert!(
        code.contains("QuicBackedChannel"),
        "Expected QuicBackedChannel in QuicConnection"
    );

    // ConnectionBuilderトレイトとimpl
    assert!(
        code.contains("ConnectionBuilder"),
        "Expected ConnectionBuilder trait"
    );
}

#[test]
fn test_quic_backed_channel_imports() {
    let schema = r#"
        protocol "minimal" version="1.0.0" {
            namespace "test"

            channel "data" from="client" lifetime="persistent" {
                send "DataMsg" {
                    field "value" type="string" required=#true
                }
                recv "DataAck" {
                    field "ok" type="bool" required=#true
                }
            }
        }
    "#;

    let parser = SchemaParser::new();
    let parsed = parser.parse(schema).unwrap();
    let type_registry = TypeRegistry::new();
    let generator = RustGenerator::new();
    let code = generator.generate(&parsed, &type_registry).unwrap();

    // QuicBackedChannel importが含まれること
    assert!(
        code.contains("QuicBackedChannel"),
        "Expected QuicBackedChannel in imports"
    );
}
