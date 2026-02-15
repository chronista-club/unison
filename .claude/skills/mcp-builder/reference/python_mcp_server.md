# Python MCPサーバー実装ガイド

## 概要

このドキュメントでは、MCP Python SDKを使用してMCPサーバーを実装するためのPython固有のベストプラクティスと例を提供します。サーバーのセットアップ、ツール登録パターン、Pydanticを使用した入力検証、エラーハンドリング、および完全な動作例について説明します。

---

## クイックリファレンス

### 主要なインポート
```python
from mcp.server.fastmcp import FastMCP
from pydantic import BaseModel, Field, field_validator, ConfigDict
from typing import Optional, List, Dict, Any
from enum import Enum
import httpx
```

### サーバーの初期化
```python
mcp = FastMCP("service_mcp")
```

### ツール登録パターン
```python
@mcp.tool(name="tool_name", annotations={...})
async def tool_function(params: InputModel) -> str:
    # 実装
    pass
```

---

## MCP Python SDKとFastMCP

公式のMCP Python SDKは、MCPサーバー構築用の高レベルフレームワークであるFastMCPを提供します。以下の機能を備えています：
- 関数シグネチャとドキュメント文字列からの自動的な説明とinputSchema生成
- 入力検証のためのPydanticモデル統合
- `@mcp.tool`を使用したデコレーターベースのツール登録

**完全なSDKドキュメントを読み込むには、WebFetchを使用して以下を参照してください：**
`https://raw.githubusercontent.com/modelcontextprotocol/python-sdk/main/README.md`

## サーバー命名規則

Python MCPサーバーは以下の命名パターンに従う必要があります：
- **形式**: `{service}_mcp`（小文字でアンダースコア区切り）
- **例**: `github_mcp`、`jira_mcp`、`stripe_mcp`

名前は以下の特徴を持つべきです：
- 汎用的（特定の機能に紐付かない）
- 統合されているサービス/APIを説明的に表現
- タスクの説明から容易に推測可能
- バージョン番号や日付を含まない

## ツールの実装

### ツールの命名

ツール名にはsnake_caseを使用し（例：「search_users」、「create_project」、「get_channel_info」）、明確でアクション指向の名前を付けます。

**命名の競合を避ける**: サービスコンテキストを含めて重複を防ぎます：
- 単なる「send_message」ではなく「slack_send_message」を使用
- 単なる「create_issue」ではなく「github_create_issue」を使用
- 単なる「list_tasks」ではなく「asana_list_tasks」を使用

### FastMCPを使用したツール構造

ツールは`@mcp.tool`デコレーターを使用して定義し、入力検証にはPydanticモデルを使用します：

```python
from pydantic import BaseModel, Field, ConfigDict
from mcp.server.fastmcp import FastMCP

# MCPサーバーを初期化
mcp = FastMCP("example_mcp")

# 入力検証用のPydanticモデルを定義
class ServiceToolInput(BaseModel):
    '''サービスツール操作の入力モデル。'''
    model_config = ConfigDict(
        str_strip_whitespace=True,  # 文字列から空白を自動削除
        validate_assignment=True,    # 代入時に検証
        extra='forbid'              # 余分なフィールドを禁止
    )

    param1: str = Field(..., description="最初のパラメータの説明（例：'user123'、'project-abc'）", min_length=1, max_length=100)
    param2: Optional[int] = Field(default=None, description="制約付きのオプション整数パラメータ", ge=0, le=1000)
    tags: Optional[List[str]] = Field(default_factory=list, description="適用するタグのリスト", max_items=10)

@mcp.tool(
    name="service_tool_name",
    annotations={
        "title": "人間が読みやすいツールタイトル",
        "readOnlyHint": True,     # ツールは環境を変更しない
        "destructiveHint": False,  # ツールは破壊的な操作を行わない
        "idempotentHint": True,    # 繰り返し呼び出しても追加の効果はない
        "openWorldHint": False     # ツールは外部エンティティとやり取りしない
    }
)
async def service_tool_name(params: ServiceToolInput) -> str:
    '''ツールの説明は自動的に'description'フィールドになります。

    このツールはサービス上で特定の操作を実行します。処理前にServiceToolInput
    Pydanticモデルを使用してすべての入力を検証します。

    Args:
        params (ServiceToolInput): 以下を含む検証済み入力パラメータ：
            - param1 (str): 最初のパラメータの説明
            - param2 (Optional[int]): デフォルト値付きのオプションパラメータ
            - tags (Optional[List[str]]): タグのリスト

    Returns:
        str: 操作結果を含むJSON形式のレスポンス
    '''
    # 実装をここに記述
    pass
```

## Pydantic v2の主要機能

- ネストされた`Config`クラスの代わりに`model_config`を使用
- 非推奨の`validator`の代わりに`field_validator`を使用
- 非推奨の`dict()`の代わりに`model_dump()`を使用
- バリデータには`@classmethod`デコレーターが必要
- バリデータメソッドには型ヒントが必要

```python
from pydantic import BaseModel, Field, field_validator, ConfigDict

class CreateUserInput(BaseModel):
    model_config = ConfigDict(
        str_strip_whitespace=True,
        validate_assignment=True
    )

    name: str = Field(..., description="ユーザーのフルネーム", min_length=1, max_length=100)
    email: str = Field(..., description="ユーザーのメールアドレス", pattern=r'^[\w\.-]+@[\w\.-]+\.\w+$')
    age: int = Field(..., description="ユーザーの年齢", ge=0, le=150)

    @field_validator('email')
    @classmethod
    def validate_email(cls, v: str) -> str:
        if not v.strip():
            raise ValueError("メールアドレスは空にできません")
        return v.lower()
```

## レスポンス形式オプション

柔軟性のために複数の出力形式をサポート：

```python
from enum import Enum

class ResponseFormat(str, Enum):
    '''ツールレスポンスの出力形式。'''
    MARKDOWN = "markdown"
    JSON = "json"

class UserSearchInput(BaseModel):
    query: str = Field(..., description="検索クエリ")
    response_format: ResponseFormat = Field(
        default=ResponseFormat.MARKDOWN,
        description="出力形式：人間が読みやすい'markdown'またはマシン読み取り可能な'json'"
    )
```

**Markdown形式**：
- わかりやすさのためにヘッダー、リスト、フォーマットを使用
- タイムスタンプを人間が読みやすい形式に変換（例：エポックの代わりに「2024-01-15 10:30:00 UTC」）
- IDを括弧内に表示名と共に表示（例：「@john.doe (U123456)」）
- 冗長なメタデータを省略（例：すべてのサイズではなく、1つのプロフィール画像URLのみを表示）
- 関連情報を論理的にグループ化

**JSON形式**：
- プログラム処理に適した完全で構造化されたデータを返す
- 利用可能なすべてのフィールドとメタデータを含む
- 一貫したフィールド名と型を使用

## ページネーションの実装

リソースをリストするツールの場合：

```python
class ListInput(BaseModel):
    limit: Optional[int] = Field(default=20, description="返す最大結果数", ge=1, le=100)
    offset: Optional[int] = Field(default=0, description="ページネーションのためにスキップする結果数", ge=0)

async def list_items(params: ListInput) -> str:
    # ページネーションを使用してAPIリクエストを実行
    data = await api_request(limit=params.limit, offset=params.offset)

    # ページネーション情報を返す
    response = {
        "total": data["total"],
        "count": len(data["items"]),
        "offset": params.offset,
        "items": data["items"],
        "has_more": data["total"] > params.offset + len(data["items"]),
        "next_offset": params.offset + len(data["items"]) if data["total"] > params.offset + len(data["items"]) else None
    }
    return json.dumps(response, indent=2)
```

## 文字数制限と切り捨て

過度に大きなレスポンスを防ぐためにCHARACTER_LIMIT定数を追加：

```python
# モジュールレベル
CHARACTER_LIMIT = 25000  # レスポンスサイズの最大文字数

async def search_tool(params: SearchInput) -> str:
    result = generate_response(data)

    # 文字数制限をチェックし、必要に応じて切り捨て
    if len(result) > CHARACTER_LIMIT:
        # データを切り捨てて通知を追加
        truncated_data = data[:max(1, len(data) // 2)]
        response["data"] = truncated_data
        response["truncated"] = True
        response["truncation_message"] = (
            f"レスポンスは{len(data)}から{len(truncated_data)}アイテムに切り捨てられました。"
            f"より多くの結果を見るには'offset'パラメータを使用するか、フィルターを追加してください。"
        )
        result = json.dumps(response, indent=2)

    return result
```

## エラーハンドリング

明確で実行可能なエラーメッセージを提供：

```python
def _handle_api_error(e: Exception) -> str:
    '''すべてのツール間で一貫したエラーフォーマット。'''
    if isinstance(e, httpx.HTTPStatusError):
        if e.response.status_code == 404:
            return "エラー：リソースが見つかりません。IDが正しいことを確認してください。"
        elif e.response.status_code == 403:
            return "エラー：権限が拒否されました。このリソースへのアクセス権がありません。"
        elif e.response.status_code == 429:
            return "エラー：レート制限を超えました。さらにリクエストを行う前にお待ちください。"
        return f"エラー：APIリクエストがステータス{e.response.status_code}で失敗しました"
    elif isinstance(e, httpx.TimeoutException):
        return "エラー：リクエストがタイムアウトしました。もう一度お試しください。"
    return f"エラー：予期しないエラーが発生しました：{type(e).__name__}"
```

## 共有ユーティリティ

共通機能を再利用可能な関数に抽出：

```python
# 共有APIリクエスト関数
async def _make_api_request(endpoint: str, method: str = "GET", **kwargs) -> dict:
    '''すべてのAPI呼び出しで再利用可能な関数。'''
    async with httpx.AsyncClient() as client:
        response = await client.request(
            method,
            f"{API_BASE_URL}/{endpoint}",
            timeout=30.0,
            **kwargs
        )
        response.raise_for_status()
        return response.json()
```

## Async/Awaitベストプラクティス

ネットワークリクエストとI/O操作には常にasync/awaitを使用：

```python
# 良い例：非同期ネットワークリクエスト
async def fetch_data(resource_id: str) -> dict:
    async with httpx.AsyncClient() as client:
        response = await client.get(f"{API_URL}/resource/{resource_id}")
        response.raise_for_status()
        return response.json()

# 悪い例：同期リクエスト
def fetch_data(resource_id: str) -> dict:
    response = requests.get(f"{API_URL}/resource/{resource_id}")  # ブロッキング
    return response.json()
```

## 型ヒント

全体を通して型ヒントを使用：

```python
from typing import Optional, List, Dict, Any

async def get_user(user_id: str) -> Dict[str, Any]:
    data = await fetch_user(user_id)
    return {"id": data["id"], "name": data["name"]}
```

## ツールのドキュメント文字列

すべてのツールには、明示的な型情報を含む包括的なドキュメント文字列が必要です：

```python
async def search_users(params: UserSearchInput) -> str:
    '''
    名前、メール、またはチームでExampleシステムのユーザーを検索します。

    このツールはExampleプラットフォーム内のすべてのユーザープロファイルを検索し、
    部分一致と様々な検索フィルターをサポートします。ユーザーの作成や変更は
    行いません。既存のユーザーの検索のみを行います。

    Args:
        params (UserSearchInput): 以下を含む検証済み入力パラメータ：
            - query (str): 名前/メールと一致させる検索文字列（例："john"、"@example.com"、"team:marketing"）
            - limit (Optional[int]): 返す最大結果数、1-100の間（デフォルト：20）
            - offset (Optional[int]): ページネーションのためにスキップする結果数（デフォルト：0）

    Returns:
        str: 以下のスキーマを持つ検索結果を含むJSON形式の文字列：

        成功レスポンス：
        {
            "total": int,           # 見つかった一致の総数
            "count": int,           # このレスポンスの結果数
            "offset": int,          # 現在のページネーションオフセット
            "users": [
                {
                    "id": str,      # ユーザーID（例："U123456789"）
                    "name": str,    # フルネーム（例："John Doe"）
                    "email": str,   # メールアドレス（例："john@example.com"）
                    "team": str     # チーム名（例："Marketing"） - オプション
                }
            ]
        }

        エラーレスポンス：
        "エラー：<エラーメッセージ>" または "'<query>'に一致するユーザーが見つかりません"

    Examples:
        - 使用する場合："マーケティングチームのメンバーをすべて見つける" -> query="team:marketing"でparams
        - 使用する場合："Johnのアカウントを検索する" -> query="john"でparams
        - 使用しない場合：ユーザーを作成する必要がある（代わりにexample_create_userを使用）
        - 使用しない場合：ユーザーIDがあり完全な詳細が必要（代わりにexample_get_userを使用）

    Error Handling:
        - 入力検証エラーはPydanticモデルによって処理される
        - リクエストが多すぎる場合は"エラー：レート制限を超えました"を返す（429ステータス）
        - APIキーが無効な場合は"エラー：無効なAPI認証"を返す（401ステータス）
        - 結果のフォーマット済みリストまたは"'query'に一致するユーザーが見つかりません"を返す
    '''
```

## 完全な例

以下は完全なPython MCPサーバーの例です：

```python
#!/usr/bin/env python3
'''
ExampleサービスのMCPサーバー。

このサーバーは、ユーザー検索、プロジェクト管理、データエクスポート機能を含む、
Example APIとやり取りするためのツールを提供します。
'''

from typing import Optional, List, Dict, Any
from enum import Enum
import httpx
from pydantic import BaseModel, Field, field_validator, ConfigDict
from mcp.server.fastmcp import FastMCP

# MCPサーバーを初期化
mcp = FastMCP("example_mcp")

# 定数
API_BASE_URL = "https://api.example.com/v1"
CHARACTER_LIMIT = 25000  # レスポンスサイズの最大文字数

# Enum
class ResponseFormat(str, Enum):
    '''ツールレスポンスの出力形式。'''
    MARKDOWN = "markdown"
    JSON = "json"

# 入力検証用のPydanticモデル
class UserSearchInput(BaseModel):
    '''ユーザー検索操作の入力モデル。'''
    model_config = ConfigDict(
        str_strip_whitespace=True,
        validate_assignment=True
    )

    query: str = Field(..., description="名前/メールと一致させる検索文字列", min_length=2, max_length=200)
    limit: Optional[int] = Field(default=20, description="返す最大結果数", ge=1, le=100)
    offset: Optional[int] = Field(default=0, description="ページネーションのためにスキップする結果数", ge=0)
    response_format: ResponseFormat = Field(default=ResponseFormat.MARKDOWN, description="出力形式")

    @field_validator('query')
    @classmethod
    def validate_query(cls, v: str) -> str:
        if not v.strip():
            raise ValueError("クエリは空または空白のみにはできません")
        return v.strip()

# 共有ユーティリティ関数
async def _make_api_request(endpoint: str, method: str = "GET", **kwargs) -> dict:
    '''すべてのAPI呼び出しで再利用可能な関数。'''
    async with httpx.AsyncClient() as client:
        response = await client.request(
            method,
            f"{API_BASE_URL}/{endpoint}",
            timeout=30.0,
            **kwargs
        )
        response.raise_for_status()
        return response.json()

def _handle_api_error(e: Exception) -> str:
    '''すべてのツール間で一貫したエラーフォーマット。'''
    if isinstance(e, httpx.HTTPStatusError):
        if e.response.status_code == 404:
            return "エラー：リソースが見つかりません。IDが正しいことを確認してください。"
        elif e.response.status_code == 403:
            return "エラー：権限が拒否されました。このリソースへのアクセス権がありません。"
        elif e.response.status_code == 429:
            return "エラー：レート制限を超えました。さらにリクエストを行う前にお待ちください。"
        return f"エラー：APIリクエストがステータス{e.response.status_code}で失敗しました"
    elif isinstance(e, httpx.TimeoutException):
        return "エラー：リクエストがタイムアウトしました。もう一度お試しください。"
    return f"エラー：予期しないエラーが発生しました：{type(e).__name__}"

# ツール定義
@mcp.tool(
    name="example_search_users",
    annotations={
        "title": "Exampleユーザーを検索",
        "readOnlyHint": True,
        "destructiveHint": False,
        "idempotentHint": True,
        "openWorldHint": True
    }
)
async def example_search_users(params: UserSearchInput) -> str:
    '''名前、メール、またはチームでExampleシステムのユーザーを検索します。

    [上記の完全なドキュメント文字列]
    '''
    try:
        # 検証済みパラメータを使用してAPIリクエストを実行
        data = await _make_api_request(
            "users/search",
            params={
                "q": params.query,
                "limit": params.limit,
                "offset": params.offset
            }
        )

        users = data.get("users", [])
        total = data.get("total", 0)

        if not users:
            return f"'{params.query}'に一致するユーザーが見つかりません"

        # リクエストされた形式に基づいてレスポンスをフォーマット
        if params.response_format == ResponseFormat.MARKDOWN:
            lines = [f"# ユーザー検索結果：'{params.query}'", ""]
            lines.append(f"{total}人のユーザーが見つかりました（{len(users)}人を表示）")
            lines.append("")

            for user in users:
                lines.append(f"## {user['name']} ({user['id']})")
                lines.append(f"- **メール**: {user['email']}")
                if user.get('team'):
                    lines.append(f"- **チーム**: {user['team']}")
                lines.append("")

            return "\n".join(lines)

        else:
            # マシン読み取り可能なJSON形式
            import json
            response = {
                "total": total,
                "count": len(users),
                "offset": params.offset,
                "users": users
            }
            return json.dumps(response, indent=2)

    except Exception as e:
        return _handle_api_error(e)

if __name__ == "__main__":
    mcp.run()
```

---

## 高度なFastMCP機能

### コンテキストパラメータインジェクション

FastMCPは、ログ記録、進行状況レポート、リソース読み取り、ユーザーインタラクションなどの高度な機能のために、ツールに自動的に`Context`パラメータを注入できます：

```python
from mcp.server.fastmcp import FastMCP, Context

mcp = FastMCP("example_mcp")

@mcp.tool()
async def advanced_search(query: str, ctx: Context) -> str:
    '''ログ記録と進行状況のためのコンテキストアクセスを持つ高度なツール。'''

    # 長い操作の進行状況をレポート
    await ctx.report_progress(0.25, "検索を開始しています...")

    # デバッグのための情報をログ記録
    await ctx.log_info("クエリを処理中", {"query": query, "timestamp": datetime.now()})

    # 検索を実行
    results = await search_api(query)
    await ctx.report_progress(0.75, "結果をフォーマット中...")

    # サーバー設定にアクセス
    server_name = ctx.fastmcp.name

    return format_results(results)

@mcp.tool()
async def interactive_tool(resource_id: str, ctx: Context) -> str:
    '''ユーザーから追加の入力を要求できるツール。'''

    # 必要な時に機密情報を要求
    api_key = await ctx.elicit(
        prompt="APIキーを入力してください：",
        input_type="password"
    )

    # 提供されたキーを使用
    return await api_call(resource_id, api_key)
```

**コンテキスト機能：**
- `ctx.report_progress(progress, message)` - 長い操作の進行状況をレポート
- `ctx.log_info(message, data)` / `ctx.log_error()` / `ctx.log_debug()` - ログ記録
- `ctx.elicit(prompt, input_type)` - ユーザーから入力を要求
- `ctx.fastmcp.name` - サーバー設定にアクセス
- `ctx.read_resource(uri)` - MCPリソースを読み取り

### リソース登録

効率的でテンプレートベースのアクセスのためにデータをリソースとして公開：

```python
@mcp.resource("file://documents/{name}")
async def get_document(name: str) -> str:
    '''ドキュメントをMCPリソースとして公開。

    リソースは、複雑なパラメータを必要としない静的または準静的データに便利です。
    柔軟なアクセスのためにURIテンプレートを使用します。
    '''
    document_path = f"./docs/{name}"
    with open(document_path, "r") as f:
        return f.read()

@mcp.resource("config://settings/{key}")
async def get_setting(key: str, ctx: Context) -> str:
    '''設定をコンテキスト付きリソースとして公開。'''
    settings = await load_settings()
    return json.dumps(settings.get(key, {}))
```

**リソース vs ツールの使い分け：**
- **リソース**：シンプルなパラメータ（URIテンプレート）でのデータアクセス用
- **ツール**：検証とビジネスロジックを持つ複雑な操作用

### 構造化された出力タイプ

FastMCPは文字列以外の複数の戻り値型をサポート：

```python
from typing import TypedDict
from dataclasses import dataclass
from pydantic import BaseModel

# 構造化された戻り値用のTypedDict
class UserData(TypedDict):
    id: str
    name: str
    email: str

@mcp.tool()
async def get_user_typed(user_id: str) -> UserData:
    '''構造化データを返す - FastMCPがシリアル化を処理。'''
    return {"id": user_id, "name": "John Doe", "email": "john@example.com"}

# 複雑な検証用のPydanticモデル
class DetailedUser(BaseModel):
    id: str
    name: str
    email: str
    created_at: datetime
    metadata: Dict[str, Any]

@mcp.tool()
async def get_user_detailed(user_id: str) -> DetailedUser:
    '''Pydanticモデルを返す - 自動的にスキーマを生成。'''
    user = await fetch_user(user_id)
    return DetailedUser(**user)
```

### ライフスパン管理

リクエスト間で持続するリソースを初期化：

```python
from contextlib import asynccontextmanager

@asynccontextmanager
async def app_lifespan():
    '''サーバーの寿命の間生きるリソースを管理。'''
    # 接続を初期化、設定をロードなど
    db = await connect_to_database()
    config = load_configuration()

    # すべてのツールで利用可能にする
    yield {"db": db, "config": config}

    # シャットダウン時のクリーンアップ
    await db.close()

mcp = FastMCP("example_mcp", lifespan=app_lifespan)

@mcp.tool()
async def query_data(query: str, ctx: Context) -> str:
    '''コンテキストを通じてライフスパンリソースにアクセス。'''
    db = ctx.request_context.lifespan_state["db"]
    results = await db.query(query)
    return format_results(results)
```

### 複数のトランスポートオプション

FastMCPは異なるトランスポートメカニズムをサポート：

```python
# デフォルト：Stdioトランスポート（CLIツール用）
if __name__ == "__main__":
    mcp.run()

# HTTPトランスポート（Webサービス用）
if __name__ == "__main__":
    mcp.run(transport="streamable_http", port=8000)

# SSEトランスポート（リアルタイム更新用）
if __name__ == "__main__":
    mcp.run(transport="sse", port=8000)
```

**トランスポートの選択：**
- **Stdio**：コマンドラインツール、サブプロセス統合
- **HTTP**：Webサービス、リモートアクセス、複数クライアント
- **SSE**：リアルタイム更新、プッシュ通知

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

### Python固有のベストプラクティス

1. **型ヒントを使用**：関数パラメータと戻り値には常に型アノテーションを含める
2. **Pydanticモデル**：すべての入力検証用に明確なPydanticモデルを定義
3. **手動検証を避ける**：制約を使用してPydanticに入力検証を任せる
4. **適切なインポート**：インポートをグループ化（標準ライブラリ、サードパーティ、ローカル）
5. **エラーハンドリング**：特定の例外タイプを使用（汎用のExceptionではなくhttpx.HTTPStatusError）
6. **非同期コンテキストマネージャー**：クリーンアップが必要なリソースには`async with`を使用
7. **定数**：モジュールレベルの定数をUPPER_CASEで定義

## 品質チェックリスト

Python MCPサーバー実装を完成させる前に、以下を確認してください：

### 戦略的設計
- [ ] ツールは単なるAPIエンドポイントのラッパーではなく、完全なワークフローを可能にする
- [ ] ツール名は自然なタスク分割を反映している
- [ ] レスポンス形式はエージェントのコンテキスト効率を最適化している
- [ ] 適切な場所で人間が読みやすい識別子を使用している
- [ ] エラーメッセージは正しい使用法へとエージェントを導く

### 実装品質
- [ ] フォーカスされた実装：最も重要で価値のあるツールが実装されている
- [ ] すべてのツールに説明的な名前とドキュメントがある
- [ ] 類似の操作間で戻り値の型が一貫している
- [ ] すべての外部呼び出しにエラーハンドリングが実装されている
- [ ] サーバー名が形式に従っている：`{service}_mcp`
- [ ] すべてのネットワーク操作がasync/awaitを使用している
- [ ] 共通機能が再利用可能な関数に抽出されている
- [ ] エラーメッセージが明確で、実行可能で、教育的である
- [ ] 出力が適切に検証され、フォーマットされている

### ツール設定
- [ ] すべてのツールがデコレーターに'name'と'annotations'を実装している
- [ ] アノテーションが正しく設定されている（readOnlyHint、destructiveHint、idempotentHint、openWorldHint）
- [ ] すべてのツールがField()定義付きの入力検証にPydantic BaseModelを使用している
- [ ] すべてのPydantic Fieldが制約付きの明示的な型と説明を持っている
- [ ] すべてのツールが明示的な入力/出力型を含む包括的なドキュメント文字列を持っている
- [ ] ドキュメント文字列にdict/JSON戻り値用の完全なスキーマ構造が含まれている
- [ ] Pydanticモデルが入力検証を処理している（手動検証は不要）

### 高度な機能（該当する場合）
- [ ] ログ記録、進行状況、または誘導にコンテキストインジェクションを使用
- [ ] 適切なデータエンドポイント用にリソースを登録
- [ ] 永続的な接続用にライフスパン管理を実装
- [ ] 構造化された出力タイプを使用（TypedDict、Pydanticモデル）
- [ ] 適切なトランスポートを設定（stdio、HTTP、SSE）

### コード品質
- [ ] ファイルにPydanticインポートを含む適切なインポートが含まれている
- [ ] 該当する場合はページネーションが適切に実装されている
- [ ] 大きなレスポンスがCHARACTER_LIMITをチェックし、明確なメッセージで切り捨てる
- [ ] 潜在的に大きな結果セットにフィルタリングオプションが提供されている
- [ ] すべての非同期関数が`async def`で適切に定義されている
- [ ] HTTPクライアントの使用が適切なコンテキストマネージャーで非同期パターンに従っている
- [ ] コード全体で型ヒントが使用されている
- [ ] 定数がモジュールレベルでUPPER_CASEで定義されている

### テスト
- [ ] サーバーが正常に実行される：`python your_server.py --help`
- [ ] すべてのインポートが正しく解決される
- [ ] サンプルツール呼び出しが期待通りに動作する
- [ ] エラーシナリオが適切に処理される