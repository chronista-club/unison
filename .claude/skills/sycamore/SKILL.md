# Sycamore - Rust Reactive WebUI Framework

**Sycamore**は、Rustで記述されたfine-grained reactivityベースのWebUIフレームワークです。仮想DOMを使用せず、SolidJS風のAPIで高性能なWebアプリケーションを構築できます。

## 🎯 スキルの目的

このスキルは、**技術的な実装**と**UI/UXデザイン**の両面から、Sycamoreを使った優れたユーザー体験を持つアプリケーション開発を支援します。

## 🌟 Sycamoreの特徴

### 技術的特徴

- **Fine-grained Reactivity**: 変更のあった部分のみを更新（仮想DOM不要）
- **WebAssembly**: Rustの性能をブラウザで発揮
- **型安全性**: コンパイル時エラー検出
- **SSR & Hydration**: サーバーサイドレンダリング対応
- **軽量**: 小さなバンドルサイズ

### UI/UX的特徴

- **即時フィードバック**: fine-grained reactivityによる遅延のない反応
- **スムーズなアニメーション**: 細粒度の更新で自然な動き
- **パフォーマンス**: WASMによる高速レンダリング
- **アクセシビリティ**: 標準Web技術との互換性

## 📚 ドキュメント構成

```
sycamore/
├── SKILL.md                  # このファイル（概要）
├── reference/                # リファレンス
│   ├── getting_started.md    # セットアップと基本
│   ├── reactivity.md         # リアクティビティシステム
│   ├── components.md         # コンポーネント設計
│   ├── routing.md            # ルーティング
│   └── styling.md            # スタイリング手法
├── patterns/                 # デザインパターン
│   ├── ui_patterns.md        # UIパターン集
│   ├── component_library.md  # コンポーネントライブラリ設計
│   ├── animations.md         # アニメーション実装
│   └── accessibility.md      # アクセシビリティ
└── examples/                 # 実践例
    ├── counter.md            # シンプルなカウンター
    ├── todo_app.md           # TodoMVCアプリ
    └── dashboard.md          # ダッシュボードUI
```

## 🎨 UI/UXデザイン原則

### 1. レスポンシブフィードバック

Sycamoreのfine-grained reactivityを活用し、ユーザーアクションへの**即座のフィードバック**を実現：

```rust
#[component]
fn Button<G: Html>(text: &str, on_click: impl Fn() + 'static) -> View<G> {
    let is_pressed = create_signal(false);
    
    view! {
        button(
            class=if *is_pressed.get() { "pressed" } else { "normal" },
            on:mousedown=move |_| is_pressed.set(true),
            on:mouseup=move |_| is_pressed.set(false),
            on:click=move |_| on_click()
        ) {
            (text)
        }
    }
}
```

**UX原則**: ユーザーは操作に対する即座の視覚的フィードバックを期待します。

### 2. アニメーションとトランジション

状態変化を**視覚的に滑らか**に表現：

```rust
view! {
    div(class="transition-all duration-300 ease-in-out") {
        (if *show.get() {
            view! { div(class="fade-in") { "Content" } }
        } else {
            view! {}
        })
    }
}
```

**UX原則**: 突然の変化はユーザーを混乱させます。トランジションで文脈を維持。

### 3. アクセシビリティファースト

セマンティックHTMLと適切なARIA属性：

```rust
view! {
    button(
        aria-label="メニューを開く",
        aria-expanded=*is_open.get()
    ) {
        "メニュー"
    }
}
```

**UX原則**: すべてのユーザーがアクセス可能なインターフェース。

## 🚀 クイックスタート

### 1. プロジェクト作成

```bash
# Trunk（推奨ビルドツール）をインストール
cargo install trunk

# 新規プロジェクト作成
cargo new my-sycamore-app
cd my-sycamore-app
```

### 2. 依存関係追加

```toml
[dependencies]
sycamore = "0.9"
```

### 3. Hello World

```rust
use sycamore::prelude::*;

#[component]
fn App() -> View {
    view! {
        div(class="app") {
            h1 { "Hello, Sycamore!" }
            p { "Fine-grained reactivityで構築されたアプリ" }
        }
    }
}

fn main() {
    sycamore::render(|| view! { App {} });
}
```

### 4. ビルド & 実行

```bash
trunk serve
```

→ `http://localhost:8080` でアプリが起動

## 🎨 スタイリング戦略

### Tailwind CSS統合

Sycamoreは**Tailwind CSS**との統合が優れています：

**1. セットアップ**

`index.html`:
```html
<link data-trunk rel="tailwind-css" href="./globals.css" />
```

`globals.css`:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

`tailwind.config.js`:
```javascript
module.exports = {
  content: ['./src/**/*.rs', './index.html'],
  theme: { extend: {} },
  plugins: [],
}
```

**2. コンポーネントで使用**

```rust
view! {
    div(class="flex items-center justify-center min-h-screen bg-gray-100") {
        h1(class="text-4xl font-bold text-blue-600") {
            "Beautiful UI with Tailwind"
        }
    }
}
```

### デザインシステム構築

再利用可能なコンポーネントライブラリ：

```rust
// design_system/button.rs
#[component]
pub fn Button(
    variant: ButtonVariant,
    size: ButtonSize,
    children: Children,
) -> View {
    let classes = match (variant, size) {
        (ButtonVariant::Primary, ButtonSize::Medium) => 
            "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
        // ... 他のバリエーション
    };
    
    view! {
        button(class=classes) {
            (children)
        }
    }
}
```

## 💡 UI/UXベストプラクティス

### 1. ローディング状態の明示

```rust
#[component]
fn DataView() -> View {
    let data = create_resource(fetch_data);
    
    view! {
        div {
            (match data.get().as_ref() {
                Some(Ok(data)) => view! {
                    // データ表示
                },
                Some(Err(e)) => view! {
                    div(class="error") { "エラー: " (e) }
                },
                None => view! {
                    div(class="loading") {
                        div(class="spinner") {}
                        "読み込み中..."
                    }
                }
            })
        }
    }
}
```

### 2. エラーハンドリングのUX

```rust
let error_message = create_signal(None::<String>);

view! {
    (if let Some(msg) = error_message.get().as_ref() {
        view! {
            div(
                class="fixed top-4 right-4 bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                role="alert"
            ) {
                span(class="block sm:inline") { (msg) }
                button(
                    class="ml-4",
                    on:click=move |_| error_message.set(None)
                ) { "✕" }
            }
        }
    } else {
        view! {}
    })
}
```

### 3. フォームバリデーション

```rust
#[component]
fn ValidatedInput() -> View {
    let value = create_signal(String::new());
    let error = create_memo(move || {
        let v = value.get();
        if v.is_empty() {
            Some("必須項目です")
        } else if v.len() < 3 {
            Some("3文字以上入力してください")
        } else {
            None
        }
    });
    
    view! {
        div(class="form-group") {
            input(
                class=if error.get().is_some() { "error" } else { "" },
                bind:value=value,
                aria-invalid=error.get().is_some(),
                aria-describedby="error-msg"
            )
            (if let Some(msg) = error.get().as_ref() {
                view! {
                    span(id="error-msg", class="error-message") { (msg) }
                }
            } else {
                view! {}
            })
        }
    }
}
```

## 🔄 リアクティビティパターン

### Signals（基本の状態管理）

```rust
let count = create_signal(0);

// 読み取り
let current = count.get();

// 更新
count.set(5);
count.update(|n| *n += 1);
```

### Memos（派生状態）

```rust
let count = create_signal(0);
let doubled = create_memo(move || *count.get() * 2);

// doubledは自動的にcountの変更を追跡
```

### Effects（副作用）

```rust
create_effect(move || {
    println!("Count changed to: {}", count.get());
});
```

## 📱 レスポンシブデザイン

```rust
#[component]
fn ResponsiveLayout() -> View {
    view! {
        div(class="container mx-auto px-4") {
            div(class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4") {
                Card { title: "Card 1" }
                Card { title: "Card 2" }
                Card { title: "Card 3" }
            }
        }
    }
}
```

## 🎭 アニメーションライブラリ

### CSSトランジション

```css
.fade-enter {
    opacity: 0;
}
.fade-enter-active {
    opacity: 1;
    transition: opacity 300ms ease-in;
}
.fade-exit {
    opacity: 1;
}
.fade-exit-active {
    opacity: 0;
    transition: opacity 300ms ease-out;
}
```

### Rustでのアニメーション制御

```rust
#[component]
fn AnimatedBox() -> View {
    let visible = create_signal(true);
    let animation_class = create_memo(move || {
        if *visible.get() {
            "fade-enter-active"
        } else {
            "fade-exit-active"
        }
    });
    
    view! {
        div(class=*animation_class.get()) {
            "Animated content"
        }
    }
}
```

## 🌐 SSRとハイドレーション

```rust
// SSRモード
#[cfg(feature = "ssr")]
fn render_to_string(app: impl FnOnce() -> View) -> String {
    sycamore::render_to_string(app)
}

// クライアントサイドハイドレーション
#[cfg(not(feature = "ssr"))]
fn main() {
    sycamore::hydrate(|| view! { App {} });
}
```

## 📦 コンポーネントライブラリ設計

### Atomic Design原則

```
atoms/          # 基本要素（Button, Input, Icon）
molecules/      # 組み合わせ（SearchBox, FormField）
organisms/      # 複雑な構造（Header, Card, Modal）
templates/      # ページレイアウト
pages/          # 完全なページ
```

### 例：原子（Atom）コンポーネント

```rust
// atoms/button.rs
#[derive(Props)]
pub struct ButtonProps {
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub disabled: bool,
    pub on_click: Box<dyn Fn()>,
    pub children: Children,
}

#[component]
pub fn Button(props: ButtonProps) -> View {
    let classes = compute_button_classes(props.variant, props.size);
    
    view! {
        button(
            class=classes,
            disabled=props.disabled,
            on:click=move |_| (props.on_click)()
        ) {
            (props.children)
        }
    }
}
```

## 🎯 Vantage MCPでの活用

### Web Console UI

```rust
#[component]
fn ProcessDashboard() -> View {
    let processes = create_signal(Vec::new());
    
    view! {
        div(class="dashboard") {
            Header {}
            ProcessList(processes=processes) {}
            Footer {}
        }
    }
}
```

## 🔗 関連リソース

- [Sycamore公式サイト](https://sycamore.dev/)
- [Sycamore Book](https://sycamore.dev/book/)
- [GitHub Repository](https://github.com/sycamore-rs/sycamore)
- [Examples](https://examples.sycamore.dev/)
- [Discord Community](https://discord.gg/sycamore)

## 📖 次のステップ

1. **[Getting Started](reference/getting_started.md)** - 環境構築と基本概念
2. **[Component Design](reference/components.md)** - コンポーネント設計パターン
3. **[UI Patterns](patterns/ui_patterns.md)** - 実践的なUIパターン集
4. **[Examples](examples/)** - 実装サンプル

---

**Last Updated**: 2025-11-03  
**Version**: 0.9.x対応
