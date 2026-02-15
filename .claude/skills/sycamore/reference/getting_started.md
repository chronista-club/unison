# Getting Started with Sycamore

Sycamoreを使ったWebアプリケーション開発の始め方を、環境構築から実装まで解説します。

## 📋 前提条件

- **Rust**: 1.70以上
- **wasm-pack**: WebAssemblyビルドツール
- **Trunk**: 推奨ビルドツール（または任意のWASMビルドツール）
- **Node.js**: (オプション) Tailwind CSSを使用する場合

## 🔧 環境構築

### 1. Rustのインストール

```bash
# rustupがない場合
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# WASMターゲット追加
rustup target add wasm32-unknown-unknown
```

### 2. Trunkのインストール

```bash
cargo install trunk
```

### 3. プロジェクト作成

```bash
# 新規プロジェクト作成
cargo new my-sycamore-app
cd my-sycamore-app
```

### 4. 依存関係の設定

**Cargo.toml**:
```toml
[package]
name = "my-sycamore-app"
version = "0.1.0"
edition = "2021"

[dependencies]
sycamore = "0.9"

[profile.release]
# WASMバイナリの最適化
opt-level = 'z'     # サイズ最適化
lto = true          # Link Time Optimization
codegen-units = 1   # コード生成ユニット削減
```

### 5. index.htmlの作成

**index.html**:
```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>My Sycamore App</title>
    <link data-trunk rel="rust" data-wasm-opt="z"/>
</head>
<body>
    <div id="app"></div>
</body>
</html>
```

### 6. メインコードの実装

**src/main.rs**:
```rust
use sycamore::prelude::*;

#[component]
fn App() -> View {
    let name = create_signal(String::from("World"));
    
    view! {
        div {
            h1 { "Hello, Sycamore!" }
            p { "Welcome, " (name.get()) "!" }
            
            input(bind:value=name, placeholder="Enter your name")
        }
    }
}

fn main() {
    sycamore::render(App);
}
```

### 7. 開発サーバー起動

```bash
trunk serve
```

→ ブラウザで `http://localhost:8080` を開く

### 8. プロダクションビルド

```bash
trunk build --release
```

→ `dist/` ディレクトリに最適化されたファイルが生成されます

## 🎨 Tailwind CSSの追加（オプション）

### 1. Tailwind CSSのセットアップ

```bash
# Node.jsプロジェクト初期化
npm init -y

# Tailwind CSSインストール
npm install -D tailwindcss
npx tailwindcss init
```

### 2. tailwind.config.js

```javascript
/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.rs",
    "./index.html",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
```

### 3. globals.cssの作成

**globals.css**:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

/* カスタムスタイル */
@layer components {
  .btn-primary {
    @apply px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors;
  }
}
```

### 4. index.htmlに追加

```html
<head>
    <!-- ... -->
    <link data-trunk rel="tailwind-css" href="./globals.css"/>
</head>
```

### 5. コンポーネントで使用

```rust
view! {
    button(class="btn-primary") {
        "Click me"
    }
}
```

## 📱 基本概念

### 1. コンポーネント

Sycamoreの基本単位は**コンポーネント**です：

```rust
#[component]
fn MyComponent() -> View {
    view! {
        div(class="my-component") {
            h1 { "Title" }
            p { "Content" }
        }
    }
}
```

### 2. リアクティビティ

**Signal**で状態を管理：

```rust
let count = create_signal(0);

// 読み取り
let value = *count.get();

// 更新
count.set(10);
count.update(|n| *n += 1);
```

### 3. イベントハンドリング

```rust
let count = create_signal(0);

view! {
    button(on:click=move |_| count.update(|n| *n += 1)) {
        "Count: " (count.get())
    }
}
```

### 4. 条件付きレンダリング

```rust
let show = create_signal(true);

view! {
    div {
        (if *show.get() {
            view! { p { "Visible" } }
        } else {
            view! { p { "Hidden" } }
        })
    }
}
```

### 5. リストレンダリング

```rust
let items = create_signal(vec!["Apple", "Banana", "Cherry"]);

view! {
    ul {
        Indexed(
            iterable=items,
            view=|item| view! {
                li { (item) }
            }
        )
    }
}
```

## 🏗️ プロジェクト構造

推奨されるディレクトリ構造：

```
my-sycamore-app/
├── Cargo.toml
├── index.html
├── globals.css              # Tailwind CSS
├── tailwind.config.js
├── src/
│   ├── main.rs
│   ├── app.rs              # メインアプリ
│   ├── components/         # 再利用可能コンポーネント
│   │   ├── mod.rs
│   │   ├── button.rs
│   │   ├── input.rs
│   │   └── card.rs
│   ├── pages/              # ページコンポーネント
│   │   ├── mod.rs
│   │   ├── home.rs
│   │   └── about.rs
│   ├── state/              # グローバル状態管理
│   │   └── mod.rs
│   └── utils/              # ユーティリティ
│       └── mod.rs
└── dist/                   # ビルド成果物（自動生成）
```

## 🎯 Hello World実装例

完全な動作例：

**src/main.rs**:
```rust
use sycamore::prelude::*;

#[component]
fn Counter() -> View {
    let count = create_signal(0);
    
    let increment = move |_| count.update(|n| *n += 1);
    let decrement = move |_| count.update(|n| *n -= 1);
    let reset = move |_| count.set(0);
    
    view! {
        div(class="flex flex-col items-center justify-center min-h-screen bg-gray-100") {
            div(class="bg-white p-8 rounded-lg shadow-lg") {
                h1(class="text-3xl font-bold mb-4 text-center") {
                    "Counter App"
                }
                
                div(class="text-6xl font-bold text-center mb-8 text-blue-600") {
                    (count.get())
                }
                
                div(class="flex gap-4") {
                    button(
                        class="btn-primary",
                        on:click=decrement
                    ) { "−" }
                    
                    button(
                        class="px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700",
                        on:click=reset
                    ) { "Reset" }
                    
                    button(
                        class="btn-primary",
                        on:click=increment
                    ) { "+" }
                }
            }
        }
    }
}

fn main() {
    sycamore::render(Counter);
}
```

## 🐛 トラブルシューティング

### よくある問題

**1. `trunk serve`が失敗する**

```bash
# Trunkの再インストール
cargo install trunk --force

# キャッシュクリア
trunk clean
```

**2. WASMビルドエラー**

```bash
# wasm32ターゲットの再追加
rustup target remove wasm32-unknown-unknown
rustup target add wasm32-unknown-unknown
```

**3. ホットリロードが動作しない**

`index.html`に正しいdata-trunk属性があるか確認：
```html
<link data-trunk rel="rust" data-wasm-opt="z"/>
```

**4. Tailwind CSSが適用されない**

```bash
# tailwind.config.jsのcontent設定を確認
# Trunk起動前にTailwindビルドを実行
npx tailwindcss -i ./globals.css -o ./dist/tailwind.css --watch
```

## 📚 次のステップ

- **[Reactivity](reactivity.md)** - リアクティビティシステムの詳細
- **[Components](components.md)** - コンポーネント設計パターン
- **[Styling](styling.md)** - スタイリング手法
- **[Examples](../examples/)** - 実践例

## 🔗 参考リンク

- [Sycamore Book](https://sycamore.dev/book/)
- [Trunk Documentation](https://trunkrs.dev/)
- [Tailwind CSS](https://tailwindcss.com/)
- [Rust WASM Book](https://rustwasm.github.io/book/)
