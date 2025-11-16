# Node/TypeScript MCPサーバー実装ガイド

## 概要

このドキュメントでは、MCP TypeScript SDKを使用してMCPサーバーを実装するためのNode/TypeScript固有のベストプラクティスと例を提供します。プロジェクト構造、サーバーセットアップ、ツール登録パターン、Zodを使用した入力検証、エラーハンドリング、および完全な動作例について説明します。

---

## クイックリファレンス

### 主要なインポート
```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import axios, { AxiosError } from "axios";
```

### サーバーの初期化
```typescript
const server = new McpServer({
  name: "service-mcp-server",
  version: "1.0.0"
});
```

### ツール登録パターン
```typescript
server.registerTool("tool_name", {...config}, async (params) => {
  // 実装
});
```

---

## MCP TypeScript SDK

公式のMCP TypeScript SDKは以下を提供します：
- サーバー初期化用の`McpServer`クラス
- ツール登録用の`registerTool`メソッド
- ランタイム入力検証用のZodスキーマ統合
- 型安全なツールハンドラ実装

完全な詳細については、リファレンスのMCP SDKドキュメントを参照してください。

## サーバー命名規則

Node/TypeScript MCPサーバーは以下の命名パターンに従う必要があります：
- **形式**: `{service}-mcp-server`（小文字でハイフン区切り）
- **例**: `github-mcp-server`、`jira-mcp-server`、`stripe-mcp-server`

名前は以下の特徴を持つべきです：
- 汎用的（特定の機能に紐付かない）
- 統合されているサービス/APIを説明的に表現
- タスクの説明から容易に推測可能
- バージョン番号や日付を含まない

## プロジェクト構造

Node/TypeScript MCPサーバー用に以下の構造を作成します：

```
{service}-mcp-server/
├── package.json
├── tsconfig.json
├── README.md
├── src/
│   ├── index.ts          # McpServer初期化を含むメインエントリーポイント
│   ├── types.ts          # TypeScript型定義とインターフェース
│   ├── tools/            # ツール実装（ドメインごとに1ファイル）
│   ├── services/         # APIクライアントと共有ユーティリティ
│   ├── schemas/          # Zod検証スキーマ
│   └── constants.ts      # 共有定数（API_URL、CHARACTER_LIMITなど）
└── dist/                 # ビルドされたJavaScriptファイル（エントリーポイント：dist/index.js）
```

## ツールの実装

### ツールの命名

ツール名にはsnake_caseを使用し（例：「search_users」、「create_project」、「get_channel_info」）、明確でアクション指向の名前を付けます。

**命名の競合を避ける**: サービスコンテキストを含めて重複を防ぎます：
- 単なる「send_message」ではなく「slack_send_message」を使用
- 単なる「create_issue」ではなく「github_create_issue」を使用
- 単なる「list_tasks」ではなく「asana_list_tasks」を使用

### ツール構造

ツールは以下の要件で`registerTool`メソッドを使用して登録されます：
- ランタイム入力検証と型安全性のためにZodスキーマを使用
- `description`フィールドは明示的に提供する必要があります - JSDocコメントは自動的に抽出されません
- `title`、`description`、`inputSchema`、`annotations`を明示的に提供
- `inputSchema`はZodスキーマオブジェクトである必要があります（JSONスキーマではなく）
- すべてのパラメータと戻り値を明示的に型付け

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";

const server = new McpServer({
  name: "example-mcp",
  version: "1.0.0"
});

// 入力検証用のZodスキーマ
const UserSearchInputSchema = z.object({
  query: z.string()
    .min(2, "クエリは最低2文字必要です")
    .max(200, "クエリは200文字を超えることはできません")
    .describe("名前/メールと一致させる検索文字列"),
  limit: z.number()
    .int()
    .min(1)
    .max(100)
    .default(20)
    .describe("返す最大結果数"),
  offset: z.number()
    .int()
    .min(0)
    .default(0)
    .describe("ページネーションのためにスキップする結果数"),
  response_format: z.nativeEnum(ResponseFormat)
    .default(ResponseFormat.MARKDOWN)
    .describe("出力形式：人間が読みやすい'markdown'またはマシン読み取り可能な'json'")
}).strict();

// Zodスキーマからの型定義
type UserSearchInput = z.infer<typeof UserSearchInputSchema>;

server.registerTool(
  "example_search_users",
  {
    title: "Exampleユーザーを検索",
    description: `名前、メール、またはチームでExampleシステムのユーザーを検索します。

このツールはExampleプラットフォーム内のすべてのユーザープロファイルを検索し、部分一致と様々な検索フィルターをサポートします。ユーザーの作成や変更は行いません。既存のユーザーの検索のみを行います。

引数：
  - query (string): 名前/メールと一致させる検索文字列
  - limit (number): 返す最大結果数、1-100の間（デフォルト：20）
  - offset (number): ページネーションのためにスキップする結果数（デフォルト：0）
  - response_format ('markdown' | 'json'): 出力形式（デフォルト：'markdown'）

戻り値：
  JSON形式の場合：以下のスキーマを持つ構造化データ：
  {
    "total": number,           // 見つかった一致の総数
    "count": number,           // このレスポンスの結果数
    "offset": number,          // 現在のページネーションオフセット
    "users": [
      {
        "id": string,          // ユーザーID（例："U123456789"）
        "name": string,        // フルネーム（例："John Doe"）
        "email": string,       // メールアドレス
        "team": string,        // チーム名（オプション）
        "active": boolean      // ユーザーがアクティブかどうか
      }
    ],
    "has_more": boolean,       // さらに結果が利用可能かどうか
    "next_offset": number      // 次のページのオフセット（has_moreがtrueの場合）
  }

例：
  - 使用する場合："マーケティングチームのメンバーをすべて見つける" -> query="team:marketing"でparams
  - 使用する場合："Johnのアカウントを検索する" -> query="john"でparams
  - 使用しない場合：ユーザーを作成する必要がある（代わりにexample_create_userを使用）

エラーハンドリング：
  - リクエストが多すぎる場合は"エラー：レート制限を超えました"を返す（429ステータス）
  - 検索結果が空の場合は"'<query>'に一致するユーザーが見つかりません"を返す`,
    inputSchema: UserSearchInputSchema,
    annotations: {
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: true
    }
  },
  async (params: UserSearchInput) => {
    try {
      // 入力検証はZodスキーマによって処理されます
      // 検証済みパラメータを使用してAPIリクエストを実行
      const data = await makeApiRequest<any>(
        "users/search",
        "GET",
        undefined,
        {
          q: params.query,
          limit: params.limit,
          offset: params.offset
        }
      );

      const users = data.users || [];
      const total = data.total || 0;

      if (!users.length) {
        return {
          content: [{
            type: "text",
            text: `'${params.query}'に一致するユーザーが見つかりません`
          }]
        };
      }

      // リクエストされた形式に基づいてレスポンスをフォーマット
      let result: string;

      if (params.response_format === ResponseFormat.MARKDOWN) {
        // 人間が読みやすいMarkdown形式
        const lines: string[] = [`# ユーザー検索結果：'${params.query}'`, ""];
        lines.push(`${total}人のユーザーが見つかりました（${users.length}人を表示）`);
        lines.push("");

        for (const user of users) {
          lines.push(`## ${user.name} (${user.id})`);
          lines.push(`- **メール**: ${user.email}`);
          if (user.team) {
            lines.push(`- **チーム**: ${user.team}`);
          }
          lines.push("");
        }

        result = lines.join("\n");

      } else {
        // マシン読み取り可能なJSON形式
        const response: any = {
          total,
          count: users.length,
          offset: params.offset,
          users: users.map((user: any) => ({
            id: user.id,
            name: user.name,
            email: user.email,
            ...(user.team ? { team: user.team } : {}),
            active: user.active ?? true
          }))
        };

        // さらに結果がある場合はページネーション情報を追加
        if (total > params.offset + users.length) {
          response.has_more = true;
          response.next_offset = params.offset + users.length;
        }

        result = JSON.stringify(response, null, 2);
      }

      return {
        content: [{
          type: "text",
          text: result
        }]
      };
    } catch (error) {
      return {
        content: [{
          type: "text",
          text: handleApiError(error)
        }]
      };
    }
  }
);
```

## 入力検証用のZodスキーマ

Zodはランタイム型検証を提供します：

```typescript
import { z } from "zod";

// 検証付きの基本スキーマ
const CreateUserSchema = z.object({
  name: z.string()
    .min(1, "名前は必須です")
    .max(100, "名前は100文字を超えることはできません"),
  email: z.string()
    .email("無効なメール形式です"),
  age: z.number()
    .int("年齢は整数である必要があります")
    .min(0, "年齢は負にできません")
    .max(150, "年齢は150を超えることはできません")
}).strict();  // 余分なフィールドを禁止するために.strict()を使用

// Enum
enum ResponseFormat {
  MARKDOWN = "markdown",
  JSON = "json"
}

const SearchSchema = z.object({
  response_format: z.nativeEnum(ResponseFormat)
    .default(ResponseFormat.MARKDOWN)
    .describe("出力形式")
});

// デフォルト付きのオプションフィールド
const PaginationSchema = z.object({
  limit: z.number()
    .int()
    .min(1)
    .max(100)
    .default(20)
    .describe("返す最大結果数"),
  offset: z.number()
    .int()
    .min(0)
    .default(0)
    .describe("スキップする結果数")
});
```

## レスポンス形式オプション

柔軟性のために複数の出力形式をサポート：

```typescript
enum ResponseFormat {
  MARKDOWN = "markdown",
  JSON = "json"
}

const inputSchema = z.object({
  query: z.string(),
  response_format: z.nativeEnum(ResponseFormat)
    .default(ResponseFormat.MARKDOWN)
    .describe("出力形式：人間が読みやすい'markdown'またはマシン読み取り可能な'json'")
});
```

**Markdown形式**：
- わかりやすさのためにヘッダー、リスト、フォーマットを使用
- タイムスタンプを人間が読みやすい形式に変換
- IDを括弧内に表示名と共に表示
- 冗長なメタデータを省略
- 関連情報を論理的にグループ化

**JSON形式**：
- プログラム処理に適した完全で構造化されたデータを返す
- 利用可能なすべてのフィールドとメタデータを含む
- 一貫したフィールド名と型を使用

## ページネーションの実装

リソースをリストするツールの場合：

```typescript
const ListSchema = z.object({
  limit: z.number().int().min(1).max(100).default(20),
  offset: z.number().int().min(0).default(0)
});

async function listItems(params: z.infer<typeof ListSchema>) {
  const data = await apiRequest(params.limit, params.offset);

  const response = {
    total: data.total,
    count: data.items.length,
    offset: params.offset,
    items: data.items,
    has_more: data.total > params.offset + data.items.length,
    next_offset: data.total > params.offset + data.items.length
      ? params.offset + data.items.length
      : undefined
  };

  return JSON.stringify(response, null, 2);
}
```

## 文字数制限と切り捨て

過度に大きなレスポンスを防ぐためにCHARACTER_LIMIT定数を追加：

```typescript
// constants.tsのモジュールレベル
export const CHARACTER_LIMIT = 25000;  // レスポンスサイズの最大文字数

async function searchTool(params: SearchInput) {
  let result = generateResponse(data);

  // 文字数制限をチェックし、必要に応じて切り捨て
  if (result.length > CHARACTER_LIMIT) {
    const truncatedData = data.slice(0, Math.max(1, data.length / 2));
    response.data = truncatedData;
    response.truncated = true;
    response.truncation_message =
      `レスポンスは${data.length}から${truncatedData.length}アイテムに切り捨てられました。` +
      `より多くの結果を見るには'offset'パラメータを使用するか、フィルターを追加してください。`;
    result = JSON.stringify(response, null, 2);
  }

  return result;
}
```

## エラーハンドリング

明確で実行可能なエラーメッセージを提供：

```typescript
import axios, { AxiosError } from "axios";

function handleApiError(error: unknown): string {
  if (error instanceof AxiosError) {
    if (error.response) {
      switch (error.response.status) {
        case 404:
          return "エラー：リソースが見つかりません。IDが正しいことを確認してください。";
        case 403:
          return "エラー：権限が拒否されました。このリソースへのアクセス権がありません。";
        case 429:
          return "エラー：レート制限を超えました。さらにリクエストを行う前にお待ちください。";
        default:
          return `エラー：APIリクエストがステータス${error.response.status}で失敗しました`;
      }
    } else if (error.code === "ECONNABORTED") {
      return "エラー：リクエストがタイムアウトしました。もう一度お試しください。";
    }
  }
  return `エラー：予期しないエラーが発生しました：${error instanceof Error ? error.message : String(error)}`;
}
```

## 共有ユーティリティ

共通機能を再利用可能な関数に抽出：

```typescript
// 共有APIリクエスト関数
async function makeApiRequest<T>(
  endpoint: string,
  method: "GET" | "POST" | "PUT" | "DELETE" = "GET",
  data?: any,
  params?: any
): Promise<T> {
  try {
    const response = await axios({
      method,
      url: `${API_BASE_URL}/${endpoint}`,
      data,
      params,
      timeout: 30000,
      headers: {
        "Content-Type": "application/json",
        "Accept": "application/json"
      }
    });
    return response.data;
  } catch (error) {
    throw error;
  }
}
```

## Async/Awaitベストプラクティス

ネットワークリクエストとI/O操作には常にasync/awaitを使用：

```typescript
// 良い例：非同期ネットワークリクエスト
async function fetchData(resourceId: string): Promise<ResourceData> {
  const response = await axios.get(`${API_URL}/resource/${resourceId}`);
  return response.data;
}

// 悪い例：Promiseチェーン
function fetchData(resourceId: string): Promise<ResourceData> {
  return axios.get(`${API_URL}/resource/${resourceId}`)
    .then(response => response.data);  // 読みにくく保守しにくい
}
```

## TypeScriptベストプラクティス

1. **厳密なTypeScriptを使用**: tsconfig.jsonで厳密モードを有効化
2. **インターフェースを定義**: すべてのデータ構造に明確なインターフェース定義を作成
3. **`any`を避ける**: `any`の代わりに適切な型または`unknown`を使用
4. **ランタイム検証にZod**: 外部データを検証するためにZodスキーマを使用
5. **型ガード**: 複雑な型チェック用の型ガード関数を作成
6. **エラーハンドリング**: 適切なエラー型チェックでtry-catchを常に使用
7. **Null安全性**: オプショナルチェーン(`?.`)とnullish合体演算子(`??`)を使用

```typescript
// 良い例：Zodとインターフェースで型安全
interface UserResponse {
  id: string;
  name: string;
  email: string;
  team?: string;
  active: boolean;
}

const UserSchema = z.object({
  id: z.string(),
  name: z.string(),
  email: z.string().email(),
  team: z.string().optional(),
  active: z.boolean()
});

type User = z.infer<typeof UserSchema>;

async function getUser(id: string): Promise<User> {
  const data = await apiCall(`/users/${id}`);
  return UserSchema.parse(data);  // ランタイム検証
}

// 悪い例：anyを使用
async function getUser(id: string): Promise<any> {
  return await apiCall(`/users/${id}`);  // 型安全性なし
}
```

## パッケージ設定

### package.json

```json
{
  "name": "{service}-mcp-server",
  "version": "1.0.0",
  "description": "{Service} API統合用のMCPサーバー",
  "type": "module",
  "main": "dist/index.js",
  "scripts": {
    "start": "node dist/index.js",
    "dev": "tsx watch src/index.ts",
    "build": "tsc",
    "clean": "rm -rf dist"
  },
  "engines": {
    "node": ">=18"
  },
  "dependencies": {
    "@modelcontextprotocol/sdk": "^1.6.1",
    "axios": "^1.7.9",
    "zod": "^3.23.8"
  },
  "devDependencies": {
    "@types/node": "^22.10.0",
    "tsx": "^4.19.2",
    "typescript": "^5.7.2"
  }
}
```

### tsconfig.json

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "Node16",
    "moduleResolution": "Node16",
    "lib": ["ES2022"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "allowSyntheticDefaultImports": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

## 完全な例

```typescript
#!/usr/bin/env node
/**
 * ExampleサービスのMCPサーバー。
 *
 * このサーバーは、ユーザー検索、プロジェクト管理、データエクスポート機能を含む、
 * Example APIとやり取りするためのツールを提供します。
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import axios, { AxiosError } from "axios";

// 定数
const API_BASE_URL = "https://api.example.com/v1";
const CHARACTER_LIMIT = 25000;

// Enum
enum ResponseFormat {
  MARKDOWN = "markdown",
  JSON = "json"
}

// Zodスキーマ
const UserSearchInputSchema = z.object({
  query: z.string()
    .min(2, "クエリは最低2文字必要です")
    .max(200, "クエリは200文字を超えることはできません")
    .describe("名前/メールと一致させる検索文字列"),
  limit: z.number()
    .int()
    .min(1)
    .max(100)
    .default(20)
    .describe("返す最大結果数"),
  offset: z.number()
    .int()
    .min(0)
    .default(0)
    .describe("ページネーションのためにスキップする結果数"),
  response_format: z.nativeEnum(ResponseFormat)
    .default(ResponseFormat.MARKDOWN)
    .describe("出力形式：人間が読みやすい'markdown'またはマシン読み取り可能な'json'")
}).strict();

type UserSearchInput = z.infer<typeof UserSearchInputSchema>;

// 共有ユーティリティ関数
async function makeApiRequest<T>(
  endpoint: string,
  method: "GET" | "POST" | "PUT" | "DELETE" = "GET",
  data?: any,
  params?: any
): Promise<T> {
  try {
    const response = await axios({
      method,
      url: `${API_BASE_URL}/${endpoint}`,
      data,
      params,
      timeout: 30000,
      headers: {
        "Content-Type": "application/json",
        "Accept": "application/json"
      }
    });
    return response.data;
  } catch (error) {
    throw error;
  }
}

function handleApiError(error: unknown): string {
  if (error instanceof AxiosError) {
    if (error.response) {
      switch (error.response.status) {
        case 404:
          return "エラー：リソースが見つかりません。IDが正しいことを確認してください。";
        case 403:
          return "エラー：権限が拒否されました。このリソースへのアクセス権がありません。";
        case 429:
          return "エラー：レート制限を超えました。さらにリクエストを行う前にお待ちください。";
        default:
          return `エラー：APIリクエストがステータス${error.response.status}で失敗しました`;
      }
    } else if (error.code === "ECONNABORTED") {
      return "エラー：リクエストがタイムアウトしました。もう一度お試しください。";
    }
  }
  return `エラー：予期しないエラーが発生しました：${error instanceof Error ? error.message : String(error)}`;
}

// MCPサーバーインスタンスを作成
const server = new McpServer({
  name: "example-mcp",
  version: "1.0.0"
});

// ツールを登録
server.registerTool(
  "example_search_users",
  {
    title: "Exampleユーザーを検索",
    description: `[上記の完全な説明]`,
    inputSchema: UserSearchInputSchema,
    annotations: {
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: true
    }
  },
  async (params: UserSearchInput) => {
    // 上記の実装
  }
);

// メイン関数
async function main() {
  // 必要に応じて環境変数を確認
  if (!process.env.EXAMPLE_API_KEY) {
    console.error("エラー：EXAMPLE_API_KEY環境変数が必要です");
    process.exit(1);
  }

  // トランスポートを作成
  const transport = new StdioServerTransport();

  // サーバーをトランスポートに接続
  await server.connect(transport);

  console.error("Example MCPサーバーがstdio経由で実行中");
}

// サーバーを実行
main().catch((error) => {
  console.error("サーバーエラー:", error);
  process.exit(1);
});
```

---

## 高度なMCP機能

### リソース登録

効率的でURIベースのアクセスのためにデータをリソースとして公開：

```typescript
import { ResourceTemplate } from "@modelcontextprotocol/sdk/types.js";

// URIテンプレート付きでリソースを登録
server.registerResource(
  {
    uri: "file://documents/{name}",
    name: "ドキュメントリソース",
    description: "名前でドキュメントにアクセス",
    mimeType: "text/plain"
  },
  async (uri: string) => {
    // URIからパラメータを抽出
    const match = uri.match(/^file:\/\/documents\/(.+)$/);
    if (!match) {
      throw new Error("無効なURI形式");
    }

    const documentName = match[1];
    const content = await loadDocument(documentName);

    return {
      contents: [{
        uri,
        mimeType: "text/plain",
        text: content
      }]
    };
  }
);

// 利用可能なリソースを動的にリスト
server.registerResourceList(async () => {
  const documents = await getAvailableDocuments();
  return {
    resources: documents.map(doc => ({
      uri: `file://documents/${doc.name}`,
      name: doc.name,
      mimeType: "text/plain",
      description: doc.description
    }))
  };
});
```

**リソース vs ツールの使い分け：**
- **リソース**：シンプルなURIベースのパラメータでのデータアクセス用
- **ツール**：検証とビジネスロジックを必要とする複雑な操作用
- **リソース**：データが比較的静的またはテンプレートベースの場合
- **ツール**：操作に副作用や複雑なワークフローがある場合

### 複数のトランスポートオプション

TypeScript SDKは異なるトランスポートメカニズムをサポート：

```typescript
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { SSEServerTransport } from "@modelcontextprotocol/sdk/server/sse.js";

// Stdioトランスポート（デフォルト - CLIツール用）
const stdioTransport = new StdioServerTransport();
await server.connect(stdioTransport);

// SSEトランスポート（リアルタイムWeb更新用）
const sseTransport = new SSEServerTransport("/message", response);
await server.connect(sseTransport);

// HTTPトランスポート（Webサービス用）
// HTTPフレームワーク統合に基づいて設定
```

**トランスポート選択ガイド：**
- **Stdio**：コマンドラインツール、サブプロセス統合、ローカル開発
- **HTTP**：Webサービス、リモートアクセス、複数の同時クライアント
- **SSE**：リアルタイム更新、サーバープッシュ通知、Webダッシュボード

### 通知サポート

サーバーの状態が変更されたときにクライアントに通知：

```typescript
// ツールリストが変更されたときに通知
server.notification({
  method: "notifications/tools/list_changed"
});

// リソースが変更されたときに通知
server.notification({
  method: "notifications/resources/list_changed"
});
```

通知は控えめに使用 - サーバー機能が本当に変更された場合のみ。

---

## コードベストプラクティス

### コードの構成可能性と再利用性

実装は構成可能性とコードの再利用を優先する必要があります：

1. **共通機能の抽出**：
   - 複数のツール間で使用される操作用に再利用可能なヘルパー関数を作成
   - コードを複製する代わりにHTTPリクエスト用の共有APIクライアントを構築
   - ユーティリティ関数にエラーハンドリングロジックを集約
   - 構成可能な専用関数にビジネスロジックを抽出
   - 共有のMarkdownまたはJSONフィールドの選択とフォーマット機能を抽出

2. **重複を避ける**：
   - ツール間で類似のコードをコピーペーストしない
   - 同様のロジックを2回書いている場合は、関数に抽出
   - ページネーション、フィルタリング、フィールド選択、フォーマットなどの一般的な操作は共有すべき
   - 認証/認可ロジックは集約すべき

## ビルドと実行

実行前に常にTypeScriptコードをビルド：

```bash
# プロジェクトをビルド
npm run build

# サーバーを実行
npm start

# 自動リロード付き開発
npm run dev
```

実装が完了したと見なす前に、`npm run build`が正常に完了することを常に確認してください。

## 品質チェックリスト

Node/TypeScript MCPサーバー実装を完成させる前に、以下を確認してください：

### 戦略的設計
- [ ] ツールは単なるAPIエンドポイントのラッパーではなく、完全なワークフローを可能にする
- [ ] ツール名は自然なタスク分割を反映している
- [ ] レスポンス形式はエージェントのコンテキスト効率を最適化している
- [ ] 適切な場所で人間が読みやすい識別子を使用している
- [ ] エラーメッセージは正しい使用法へとエージェントを導く

### 実装品質
- [ ] フォーカスされた実装：最も重要で価値のあるツールが実装されている
- [ ] すべてのツールが完全な設定で`registerTool`を使用して登録されている
- [ ] すべてのツールに`title`、`description`、`inputSchema`、`annotations`が含まれている
- [ ] アノテーションが正しく設定されている（readOnlyHint、destructiveHint、idempotentHint、openWorldHint）
- [ ] すべてのツールが`.strict()`強制でランタイム入力検証にZodスキーマを使用している
- [ ] すべてのZodスキーマが適切な制約と説明的なエラーメッセージを持っている
- [ ] すべてのツールが明示的な入力/出力型を含む包括的な説明を持っている
- [ ] 説明に戻り値の例と完全なスキーマドキュメントが含まれている
- [ ] エラーメッセージが明確で、実行可能で、教育的である

### TypeScript品質
- [ ] すべてのデータ構造にTypeScriptインターフェースが定義されている
- [ ] tsconfig.jsonで厳密なTypeScriptが有効になっている
- [ ] `any`型を使用していない - 代わりに`unknown`または適切な型を使用
- [ ] すべての非同期関数が明示的なPromise<T>戻り値型を持っている
- [ ] エラーハンドリングが適切な型ガードを使用している（例：`axios.isAxiosError`、`z.ZodError`）

### 高度な機能（該当する場合）
- [ ] 適切なデータエンドポイント用にリソースを登録
- [ ] 適切なトランスポートを設定（stdio、HTTP、SSE）
- [ ] 動的サーバー機能用に通知を実装
- [ ] SDKインターフェースで型安全

### プロジェクト設定
- [ ] Package.jsonに必要な依存関係がすべて含まれている
- [ ] ビルドスクリプトがdist/ディレクトリに動作するJavaScriptを生成する
- [ ] メインエントリーポイントがdist/index.jsとして適切に設定されている
- [ ] サーバー名が形式に従っている：`{service}-mcp-server`
- [ ] tsconfig.jsonが厳密モードで適切に設定されている

### コード品質
- [ ] 該当する場合はページネーションが適切に実装されている
- [ ] 大きなレスポンスがCHARACTER_LIMIT定数をチェックし、明確なメッセージで切り捨てる
- [ ] 潜在的に大きな結果セットにフィルタリングオプションが提供されている
- [ ] すべてのネットワーク操作がタイムアウトと接続エラーを適切に処理する
- [ ] 共通機能が再利用可能な関数に抽出されている
- [ ] 類似の操作間で戻り値の型が一貫している

### テストとビルド
- [ ] `npm run build`がエラーなしで正常に完了する
- [ ] dist/index.jsが作成され実行可能である
- [ ] サーバーが実行される：`node dist/index.js --help`
- [ ] すべてのインポートが正しく解決される
- [ ] サンプルツール呼び出しが期待通りに動作する