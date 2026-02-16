use super::CodeGenerator;
use crate::parser::{
    DefaultValue, Enum, Field, FieldType, Message, Method, MethodMessage, ParsedSchema, Protocol,
    Service, Stream, TypeRegistry,
};
use anyhow::Result;
use convert_case::{Case, Casing};

#[derive(Default)]
pub struct TypeScriptGenerator;

impl TypeScriptGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl CodeGenerator for TypeScriptGenerator {
    fn generate(&self, schema: &ParsedSchema, type_registry: &TypeRegistry) -> Result<String> {
        let mut code = String::new();

        // インポート文を追加
        code.push_str(&self.generate_imports());
        code.push('\n');

        // 列挙型を生成
        for enum_def in &schema.enums {
            code.push_str(&self.generate_enum(enum_def));
            code.push_str("\n\n");
        }

        // メッセージをインターフェースとして生成
        for message in &schema.messages {
            code.push_str(&self.generate_message(message, type_registry));
            code.push_str("\n\n");
        }

        // プロトコル固有のコードを生成
        if let Some(protocol) = &schema.protocol {
            code.push_str(&self.generate_protocol(protocol, type_registry));
        }

        Ok(code)
    }
}

impl TypeScriptGenerator {
    fn generate_imports(&self) -> String {
        r#"// Auto-generated TypeScript definitions
// DO NOT EDIT MANUALLY

export type Timestamp = string; // ISO-8601 format
export type UUID = string;
export type LanguageCode = string; // ISO 639-1 format
"#
        .to_string()
    }

    fn generate_protocol(&self, protocol: &Protocol, type_registry: &TypeRegistry) -> String {
        let mut code = String::new();

        // プロトコルのネームスペースコメントを生成
        if let Some(namespace) = &protocol.namespace {
            code.push_str(&format!("// Namespace: {}\n", namespace));
            code.push_str(&format!("// Version: {}\n\n", protocol.version));
        }

        // プロトコルの列挙型を生成
        for enum_def in &protocol.enums {
            code.push_str(&self.generate_enum(enum_def));
            code.push_str("\n\n");
        }

        // プロトコルのメッセージを生成
        for message in &protocol.messages {
            code.push_str(&self.generate_message(message, type_registry));
            code.push_str("\n\n");
        }

        // サービスクライアントを生成
        for service in &protocol.services {
            code.push_str(&self.generate_service(service, type_registry));
            code.push_str("\n\n");
        }

        code
    }

    fn generate_enum(&self, enum_def: &Enum) -> String {
        let name = &enum_def.name;
        let values: Vec<String> = enum_def
            .values
            .iter()
            .map(|v| format!("  {} = '{}',", v.to_case(Case::Pascal), v))
            .collect();

        format!("export enum {} {{\n{}\n}}", name, values.join("\n"))
    }

    fn generate_message(&self, message: &Message, type_registry: &TypeRegistry) -> String {
        // インラインメッセージはスキップ
        if message.name.starts_with("_inline_") {
            return String::new();
        }

        let name = &message.name;
        let fields: Vec<String> = message
            .fields
            .iter()
            .map(|f| self.generate_field(f, type_registry))
            .collect();

        format!("export interface {} {{\n{}\n}}", name, fields.join("\n"))
    }

    fn generate_field(&self, field: &Field, type_registry: &TypeRegistry) -> String {
        let name = &field.name;
        let ts_type = self.field_type_to_typescript(&field.field_type(), type_registry);

        let optional = if !field.required { "?" } else { "" };

        let mut field_def = format!("  {}{}: {};", name, optional, ts_type);

        // 制約とデフォルト値のJSDocコメントを追加
        let mut comments = Vec::new();

        if let Some(default) = &field.default() {
            comments.push(format!(
                "@default {}",
                self.default_value_to_string(default)
            ));
        }

        if let (Some(min), Some(max)) = (field.constraints().min, field.constraints().max) {
            comments.push(format!("@minimum {} @maximum {}", min, max));
        }

        if let Some(pattern) = &field.constraints().pattern {
            comments.push(format!("@pattern {}", pattern));
        }

        if !comments.is_empty() {
            let comment = format!("  /** {} */\n", comments.join(" "));
            field_def = format!("{}{}", comment, field_def);
        }

        field_def
    }

    #[allow(clippy::only_used_in_recursion)]
    fn field_type_to_typescript(
        &self,
        field_type: &FieldType,
        type_registry: &TypeRegistry,
    ) -> String {
        match field_type {
            FieldType::String => "string".to_string(),
            FieldType::Int | FieldType::Float => "number".to_string(),
            FieldType::Bool => "boolean".to_string(),
            FieldType::Json | FieldType::Object => "any".to_string(),
            FieldType::Array(inner) => {
                format!("{}[]", self.field_type_to_typescript(inner, type_registry))
            }
            FieldType::Map(_, value) => {
                format!(
                    "Record<string, {}>",
                    self.field_type_to_typescript(value, type_registry)
                )
            }
            FieldType::Enum(values) => values
                .iter()
                .map(|v| format!("'{}'", v))
                .collect::<Vec<_>>()
                .join(" | "),
            FieldType::Custom(name) => {
                type_registry.get_typescript_type(name).unwrap_or_else(|| {
                    // snake_caseをTypeScriptの型用にPascalCaseへ変換
                    if name == "timestamp" {
                        "Timestamp".to_string()
                    } else if name == "uuid" {
                        "UUID".to_string()
                    } else if name == "language_code" {
                        "LanguageCode".to_string()
                    } else {
                        name.to_case(Case::Pascal)
                    }
                })
            }
        }
    }

    fn default_value_to_string(&self, default: &DefaultValue) -> String {
        match default {
            DefaultValue::String(s) => format!("'{}'", s),
            DefaultValue::Int(i) => i.to_string(),
            DefaultValue::Float(f) => f.to_string(),
            DefaultValue::Bool(b) => b.to_string(),
            DefaultValue::Null => "null".to_string(),
            DefaultValue::Array(_) => "[]".to_string(),
            DefaultValue::Object(_) => "{}".to_string(),
        }
    }

    fn generate_service(&self, service: &Service, type_registry: &TypeRegistry) -> String {
        let service_name = format!("{}Service", service.name);
        let client_name = format!("{}Client", service.name);

        let mut code = String::new();

        // インラインメッセージのリクエスト/レスポンス型を生成
        code.push_str(&self.generate_inline_types(service, type_registry));

        // サービスインターフェースを生成
        code.push_str(&format!("export interface {} {{\n", service_name));

        for method in &service.methods {
            code.push_str(&self.generate_service_method(method, type_registry));
        }

        for stream in &service.streams {
            code.push_str(&self.generate_service_stream(stream, type_registry));
        }

        code.push_str("}\n\n");

        // クライアントクラスを生成
        code.push_str(&format!("export class {} {{\n", client_name));
        code.push_str("  constructor(private readonly transport: WebSocketTransport) {}\n\n");

        for method in &service.methods {
            code.push_str(&self.generate_client_method(method, type_registry));
        }

        for stream in &service.streams {
            code.push_str(&self.generate_client_stream(stream, type_registry));
        }

        code.push_str("}\n");

        code
    }

    fn generate_inline_types(&self, service: &Service, type_registry: &TypeRegistry) -> String {
        let mut code = String::new();

        for method in &service.methods {
            if let Some(request) = &method.request {
                let type_name = format!("{}Request", method.name.to_case(Case::Pascal));
                code.push_str(&self.generate_inline_message(&type_name, request, type_registry));
                code.push_str("\n\n");
            }

            if let Some(response) = &method.response {
                let type_name = format!("{}Response", method.name.to_case(Case::Pascal));
                code.push_str(&self.generate_inline_message(&type_name, response, type_registry));
                code.push_str("\n\n");
            }
        }

        for stream in &service.streams {
            if let Some(request) = &stream.request {
                let type_name = format!("{}Request", stream.name.to_case(Case::Pascal));
                code.push_str(&self.generate_inline_message(&type_name, request, type_registry));
                code.push_str("\n\n");
            }

            if let Some(response) = &stream.response {
                let type_name = format!("{}Response", stream.name.to_case(Case::Pascal));
                code.push_str(&self.generate_inline_message(&type_name, response, type_registry));
                code.push_str("\n\n");
            }
        }

        code
    }

    fn generate_inline_message(
        &self,
        name: &str,
        message: &MethodMessage,
        type_registry: &TypeRegistry,
    ) -> String {
        let fields: Vec<String> = message
            .fields
            .iter()
            .map(|f| self.generate_field(f, type_registry))
            .collect();

        format!("export interface {} {{\n{}\n}}", name, fields.join("\n"))
    }

    fn generate_service_method(&self, method: &Method, _type_registry: &TypeRegistry) -> String {
        let name = method.name.to_case(Case::Camel);
        let request_type = self.get_method_type_name(&method.request, &method.name, "Request");
        let response_type = self.get_method_type_name(&method.response, &method.name, "Response");

        format!(
            "  {}(request: {}): Promise<{}>;\n",
            name, request_type, response_type
        )
    }

    fn generate_service_stream(&self, stream: &Stream, _type_registry: &TypeRegistry) -> String {
        let name = stream.name.to_case(Case::Camel);
        let request_type = self.get_method_type_name(&stream.request, &stream.name, "Request");
        let response_type = self.get_method_type_name(&stream.response, &stream.name, "Response");

        format!(
            "  {}(request: {}): AsyncIterableIterator<{}>;\n",
            name, request_type, response_type
        )
    }

    fn generate_client_method(&self, method: &Method, _type_registry: &TypeRegistry) -> String {
        let name = method.name.to_case(Case::Camel);
        let request_type = self.get_method_type_name(&method.request, &method.name, "Request");
        let response_type = self.get_method_type_name(&method.response, &method.name, "Response");

        format!(
            r#"  async {}(request: {}): Promise<{}> {{
    return this.transport.request('{}', request);
  }}
"#,
            name, request_type, response_type, method.name
        )
    }

    fn generate_client_stream(&self, stream: &Stream, _type_registry: &TypeRegistry) -> String {
        let name = stream.name.to_case(Case::Camel);
        let request_type = self.get_method_type_name(&stream.request, &stream.name, "Request");
        let response_type = self.get_method_type_name(&stream.response, &stream.name, "Response");

        format!(
            r#"  async *{}(request: {}): AsyncIterableIterator<{}> {{
    yield* this.transport.stream('{}', request);
  }}
"#,
            name, request_type, response_type, stream.name
        )
    }

    fn get_method_type_name(
        &self,
        message: &Option<MethodMessage>,
        method_name: &str,
        suffix: &str,
    ) -> String {
        if message.is_some() {
            // MethodMessage は常にインラインなので、メソッド名ベースの型名を生成
            format!("{}{}", method_name.to_case(Case::Pascal), suffix)
        } else {
            "void".to_string()
        }
    }
}

// WebSocketトランスポートインターフェース（生成されたファイルに含まれる）
impl TypeScriptGenerator {
    pub fn generate_transport_interface() -> String {
        r#"// WebSocket Transport Interface
export interface WebSocketTransport {
  request<TRequest, TResponse>(method: string, request: TRequest): Promise<TResponse>;
  stream<TRequest, TResponse>(method: string, request: TRequest): AsyncIterableIterator<TResponse>;
  connect(url: string): Promise<void>;
  disconnect(): Promise<void>;
  isConnected(): boolean;
}

// Basic WebSocket transport implementation
export class WebSocketTransportImpl implements WebSocketTransport {
  private ws: WebSocket | null = null;
  private requestId = 0;
  private pendingRequests = new Map<number, {
    resolve: (value: any) => void;
    reject: (error: Error) => void;
  }>();
  private streamHandlers = new Map<number, (data: any) => void>();

  async connect(url: string): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(url);

      this.ws.onopen = () => resolve();
      this.ws.onerror = (error) => reject(error);
      this.ws.onmessage = (event) => this.handleMessage(event);
      this.ws.onclose = () => this.handleClose();
    });
  }

  async disconnect(): Promise<void> {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  async request<TRequest, TResponse>(method: string, request: TRequest): Promise<TResponse> {
    if (!this.isConnected()) {
      throw new Error('WebSocket not connected');
    }

    const id = ++this.requestId;
    const message = {
      id,
      method,
      type: 'request',
      payload: request,
    };

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(id, { resolve, reject });
      this.ws!.send(JSON.stringify(message));
    });
  }

  async *stream<TRequest, TResponse>(method: string, request: TRequest): AsyncIterableIterator<TResponse> {
    if (!this.isConnected()) {
      throw new Error('WebSocket not connected');
    }

    const id = ++this.requestId;
    const message = {
      id,
      method,
      type: 'stream',
      payload: request,
    };

    const queue: TResponse[] = [];
    let resolve: ((value: IteratorResult<TResponse>) => void) | null = null;
    let done = false;

    this.streamHandlers.set(id, (data: any) => {
      if (data.type === 'stream_end') {
        done = true;
        this.streamHandlers.delete(id);
        if (resolve) {
          resolve({ done: true, value: undefined });
        }
      } else if (data.type === 'stream_data') {
        const response = data.payload as TResponse;
        if (resolve) {
          resolve({ done: false, value: response });
          resolve = null;
        } else {
          queue.push(response);
        }
      } else if (data.type === 'error') {
        done = true;
        this.streamHandlers.delete(id);
        if (resolve) {
          resolve({ done: true, value: undefined });
        }
      }
    });

    this.ws!.send(JSON.stringify(message));

    while (!done) {
      if (queue.length > 0) {
        yield queue.shift()!;
      } else {
        await new Promise<IteratorResult<TResponse>>((r) => {
          resolve = r;
        }).then((result) => {
          if (!result.done) {
            return result.value;
          }
        });
      }
    }
  }

  private handleMessage(event: MessageEvent): void {
    try {
      const data = JSON.parse(event.data);

      if (data.type === 'response') {
        const handler = this.pendingRequests.get(data.id);
        if (handler) {
          this.pendingRequests.delete(data.id);
          if (data.error) {
            handler.reject(new Error(data.error));
          } else {
            handler.resolve(data.payload);
          }
        }
      } else if (data.type === 'stream_data' || data.type === 'stream_end' || data.type === 'error') {
        const handler = this.streamHandlers.get(data.id);
        if (handler) {
          handler(data);
        }
      }
    } catch (error) {
      console.error('Failed to parse WebSocket message:', error);
    }
  }

  private handleClose(): void {
    // Reject all pending requests
    for (const [id, handler] of this.pendingRequests) {
      handler.reject(new Error('WebSocket connection closed'));
    }
    this.pendingRequests.clear();
    this.streamHandlers.clear();
  }
}
"#.to_string()
    }
}
