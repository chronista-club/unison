# Chrome DevTools MCP リファレンス

## 概要

Chrome DevTools MCP（Model Context Protocol）サーバーは、Claude CodeセッションでChromeブラウザを自動操作するためのツールセットです。Selenium やPlaywrightの代替として、よりシンプルでインタラクティブなブラウザ操作を可能にします。

## 利用可能なMCPツール

### mcp__chrome-devtools__new_page

新しいブラウザページまたはタブを開きます。

#### パラメータ

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| url | string | はい | 開くページのURL |

#### 使用例

```json
mcp__chrome-devtools__new_page
{
  "url": "https://example.com"
}
```

#### レスポンス例

```json
{
  "pageId": "1234567890",
  "status": "loaded"
}
```

### mcp__chrome-devtools__take_snapshot

現在のページのDOMスナップショットを取得します。要素のUID、テキスト内容、属性などが含まれます。

#### パラメータ

なし（現在のアクティブページを対象とします）

#### 使用例

```
mcp__chrome-devtools__take_snapshot
{}
```

#### レスポンス例

```
Page snapshot:
- Title: Example Domain
- URL: https://example.com

Elements:
[uid: 1_1] <html>
  [uid: 1_2] <head>
    [uid: 1_3] <title>Example Domain</title>
  [uid: 1_4] <body>
    [uid: 1_5] <h1>Example Domain</h1>
    [uid: 1_6] <p>This domain is for use in illustrative examples...</p>
    [uid: 1_7] <a href="https://www.iana.org/domains/example">More information...</a>
```

### mcp__chrome-devtools__click

指定したUID要素をクリックします。

#### パラメータ

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| uid | string | はい | クリックする要素のUID（take_snapshotで取得） |

#### 使用例

```json
mcp__chrome-devtools__click
{
  "uid": "1_7"
}
```

#### レスポンス例

```json
{
  "status": "clicked",
  "element": "a",
  "text": "More information..."
}
```

### mcp__chrome-devtools__take_screenshot

現在のページのスクリーンショットを撮影します。

#### パラメータ

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| fullPage | boolean | いいえ | ページ全体をキャプチャするか（デフォルト: false） |
| format | string | いいえ | 画像形式（"png" または "jpeg"、デフォルト: "png"） |

#### 使用例

```json
mcp__chrome-devtools__take_screenshot
{
  "fullPage": true,
  "format": "png"
}
```

#### レスポンス例

```
Screenshot saved as: screenshot_1234567890.png
Dimensions: 1920x1080
File size: 245KB
```

### mcp__chrome-devtools__navigate_page

ページをナビゲート（URLの変更、リロード、戻る/進む）します。

#### パラメータ

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| url | string | いいえ | 遷移先のURL（指定しない場合はリロード） |
| action | string | いいえ | "back", "forward", "reload"のいずれか |

#### 使用例

```json
mcp__chrome-devtools__navigate_page
{
  "action": "reload"
}
```

```json
mcp__chrome-devtools__navigate_page
{
  "url": "https://example.com/page2"
}
```

### mcp__chrome-devtools__input_text

テキストフィールドに文字を入力します。

#### パラメータ

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| uid | string | はい | 入力フィールドのUID |
| text | string | はい | 入力するテキスト |
| clear | boolean | いいえ | 入力前にフィールドをクリアするか（デフォルト: true） |

#### 使用例

```json
mcp__chrome-devtools__input_text
{
  "uid": "2_15",
  "text": "hello@example.com",
  "clear": true
}
```

### mcp__chrome-devtools__select_option

セレクトボックスでオプションを選択します。

#### パラメータ

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| uid | string | はい | セレクト要素のUID |
| value | string | はい/いいえ | 選択するオプションのvalue属性 |
| text | string | はい/いいえ | 選択するオプションのテキスト |
| index | number | はい/いいえ | 選択するオプションのインデックス（0始まり） |

※ value、text、indexのいずれか1つを指定

#### 使用例

```json
mcp__chrome-devtools__select_option
{
  "uid": "3_10",
  "text": "日本"
}
```

### mcp__chrome-devtools__get_cookies

現在のページのクッキーを取得します。

#### パラメータ

なし

#### 使用例

```
mcp__chrome-devtools__get_cookies
{}
```

#### レスポンス例

```json
[
  {
    "name": "session_id",
    "value": "abc123...",
    "domain": ".example.com",
    "path": "/",
    "expires": 1735689600,
    "httpOnly": true,
    "secure": true
  }
]
```

### mcp__chrome-devtools__execute_javascript

ページでJavaScriptコードを実行します。

#### パラメータ

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| code | string | はい | 実行するJavaScriptコード |

#### 使用例

```json
mcp__chrome-devtools__execute_javascript
{
  "code": "document.title"
}
```

```json
mcp__chrome-devtools__execute_javascript
{
  "code": "document.querySelectorAll('.item').length"
}
```

## 一般的なワークフロー

### 1. ログインフロー

```json
// 1. ログインページを開く
mcp__chrome-devtools__new_page
{ "url": "https://example.com/login" }

// 2. フォーム要素を確認
mcp__chrome-devtools__take_snapshot

// 3. ユーザー名を入力
mcp__chrome-devtools__input_text
{ "uid": "form_username", "text": "user@example.com" }

// 4. パスワードを入力
mcp__chrome-devtools__input_text
{ "uid": "form_password", "text": "password123" }

// 5. ログインボタンをクリック
mcp__chrome-devtools__click
{ "uid": "btn_login" }

// 6. ログイン成功を確認
mcp__chrome-devtools__take_snapshot
```

### 2. データ収集フロー

```json
// 1. 一覧ページを開く
mcp__chrome-devtools__new_page
{ "url": "https://example.com/products" }

// 2. ページ構造を確認
mcp__chrome-devtools__take_snapshot

// 3. 商品数を確認
mcp__chrome-devtools__execute_javascript
{ "code": "document.querySelectorAll('.product-item').length" }

// 4. 次のページへ
mcp__chrome-devtools__click
{ "uid": "pagination_next" }
```

### 3. フォーム送信フロー

```json
// 1. フォームページを開く
mcp__chrome-devtools__new_page
{ "url": "https://example.com/contact" }

// 2. フォーム要素を確認
mcp__chrome-devtools__take_snapshot

// 3. 各フィールドに入力
mcp__chrome-devtools__input_text
{ "uid": "input_name", "text": "山田太郎" }

mcp__chrome-devtools__input_text
{ "uid": "input_email", "text": "yamada@example.com" }

mcp__chrome-devtools__select_option
{ "uid": "select_category", "text": "技術的な質問" }

// 4. 送信前のスクリーンショット
mcp__chrome-devtools__take_screenshot

// 5. 送信
mcp__chrome-devtools__click
{ "uid": "btn_submit" }
```

## 高度な使用例

### 動的コンテンツの待機

```javascript
// ローディングが終わるまで待つ
mcp__chrome-devtools__execute_javascript
{
  "code": "new Promise(resolve => {
    const checkLoaded = setInterval(() => {
      if (!document.querySelector('.loading')) {
        clearInterval(checkLoaded);
        resolve('loaded');
      }
    }, 500);
  })"
}
```

### 複数タブの操作

```json
// タブ1を開く
mcp__chrome-devtools__new_page
{ "url": "https://example.com/page1" }

// タブ2を開く（新しいタブで）
mcp__chrome-devtools__new_page
{ "url": "https://example.com/page2" }

// タブ間の切り替えは現在のMCPでは制限あり
// 各タブで独立した操作を行う
```

### エラーハンドリング

```javascript
// エラーメッセージの確認
mcp__chrome-devtools__execute_javascript
{
  "code": "document.querySelector('.error-message')?.textContent || 'No error'"
}

// コンソールエラーの確認（制限あり）
mcp__chrome-devtools__execute_javascript
{
  "code": "window.__errors = []; window.addEventListener('error', e => window.__errors.push(e.message)); 'Error listener added'"
}
```

## 制限事項と回避策

### 1. ファイルアップロード
- **制限**: ファイル選択ダイアログを直接操作できない
- **回避策**: APIエンドポイントを使用するか、ドラッグ&ドロップ可能なUIを実装

### 2. アラート/確認ダイアログ
- **制限**: ネイティブダイアログの操作が困難
- **回避策**: カスタムモーダルを使用するか、事前にダイアログを無効化

### 3. iframeの操作
- **制限**: クロスオリジンiframe内の要素にアクセスできない
- **回避策**: 親フレームから可能な操作に限定

### 4. ダウンロード
- **制限**: ダウンロードファイルの内容を直接確認できない
- **回避策**: ダウンロードURLを取得してfetch/curlで確認

## パフォーマンス最適化

### 1. スナップショットの最適化
```javascript
// 特定の要素のみを対象にする（将来的な機能）
mcp__chrome-devtools__take_snapshot
{
  "selector": "#main-content"  // 未実装の例
}
```

### 2. 並列処理の考慮
- 複数のページ操作は順次実行
- 各操作の間に適切な待機時間を設ける

### 3. リソースの管理
- 長時間のセッションでは定期的にページをリロード
- 不要なタブは閉じる

## デバッグテクニック

### 1. 要素が見つからない場合
```
1. take_snapshotで現在の状態を確認
2. JavaScriptで要素の存在を確認
3. 待機処理を追加
4. セレクタを変更
```

### 2. クリックが効かない場合
```javascript
// 要素の状態を確認
mcp__chrome-devtools__execute_javascript
{
  "code": "(() => {
    const el = document.querySelector('[uid=\"1_5\"]');
    return {
      visible: el.offsetWidth > 0,
      disabled: el.disabled,
      clickable: window.getComputedStyle(el).pointerEvents !== 'none'
    };
  })()"
}
```

### 3. ページ遷移の確認
```javascript
// 現在のURLを確認
mcp__chrome-devtools__execute_javascript
{
  "code": "window.location.href"
}
```

## ベストプラクティス

1. **明示的な待機**
   - 動的コンテンツは複数回スナップショットを取る
   - JavaScriptでの状態確認を活用

2. **エラーハンドリング**
   - 各操作後に成功を確認
   - 予期しない状態への対処を準備

3. **ログ記録**
   - 重要な操作前後でスクリーンショットを取る
   - 実行したコマンドと結果を記録

4. **再現性の確保**
   - 初期状態を明確にする
   - 環境依存を最小化

5. **段階的なテスト**
   - 小さな単位でテストを構築
   - 各ステップを個別に検証