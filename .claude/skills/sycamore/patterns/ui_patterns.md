# UI Patterns - Sycamore実装パターン集

UI/UXデザイナー視点で整理した、Sycamoreでの実践的なUIパターン集です。

## 🎨 デザインシステムの基礎

### カラーパレット

**tailwind.config.js**:
```javascript
module.exports = {
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#eff6ff',
          500: '#3b82f6',
          600: '#2563eb',
          700: '#1d4ed8',
        },
        success: '#10b981',
        warning: '#f59e0b',
        error: '#ef4444',
      }
    }
  }
}
```

### タイポグラフィ

```rust
#[component]
fn Typography() -> View {
    view! {
        div {
            h1(class="text-4xl font-bold mb-4") { "見出し1" }
            h2(class="text-3xl font-semibold mb-3") { "見出し2" }
            h3(class="text-2xl font-medium mb-2") { "見出し3" }
            p(class="text-base leading-relaxed") { "本文テキスト" }
            span(class="text-sm text-gray-600") { "補足テキスト" }
        }
    }
}
```

## 🔘 ボタンコンポーネント

### ベーシックボタン

```rust
use sycamore::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

#[component]
pub fn Button<G: Html>(
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: ReadSignal<bool>,
    on_click: impl Fn() + 'static,
    children: Children,
) -> View<G> {
    let base_classes = "rounded font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2";
    
    let variant_classes = match variant {
        ButtonVariant::Primary => "bg-blue-600 hover:bg-blue-700 text-white focus:ring-blue-500",
        ButtonVariant::Secondary => "bg-gray-200 hover:bg-gray-300 text-gray-900 focus:ring-gray-500",
        ButtonVariant::Danger => "bg-red-600 hover:bg-red-700 text-white focus:ring-red-500",
        ButtonVariant::Ghost => "hover:bg-gray-100 text-gray-700 focus:ring-gray-500",
    };
    
    let size_classes = match size {
        ButtonSize::Small => "px-3 py-1.5 text-sm",
        ButtonSize::Medium => "px-4 py-2 text-base",
        ButtonSize::Large => "px-6 py-3 text-lg",
    };
    
    let disabled_classes = if *disabled.get() {
        "opacity-50 cursor-not-allowed"
    } else {
        "cursor-pointer"
    };
    
    let classes = format!("{} {} {} {}", base_classes, variant_classes, size_classes, disabled_classes);
    
    view! {
        button(
            class=classes,
            disabled=*disabled.get(),
            on:click=move |_| {
                if !*disabled.get() {
                    on_click()
                }
            }
        ) {
            (children)
        }
    }
}
```

### ローディングボタン

```rust
#[component]
fn LoadingButton() -> View {
    let is_loading = create_signal(false);
    
    let handle_click = move || {
        is_loading.set(true);
        // 非同期処理シミュレーション
        // 実際はfetchなど
    };
    
    view! {
        button(
            class="btn-primary relative",
            disabled=*is_loading.get(),
            on:click=move |_| handle_click()
        ) {
            (if *is_loading.get() {
                view! {
                    span {
                        span(class="spinner mr-2") {}
                        "処理中..."
                    }
                }
            } else {
                view! { span { "送信" } }
            })
        }
    }
}
```

## 📝 フォームコンポーネント

### バリデーション付きInput

```rust
#[derive(Clone)]
pub struct ValidationRule {
    pub validate: fn(&str) -> Option<String>,
}

#[component]
pub fn ValidatedInput(
    label: &'static str,
    placeholder: &'static str,
    value: Signal<String>,
    rules: Vec<ValidationRule>,
) -> View {
    let error = create_signal(None::<String>);
    let is_touched = create_signal(false);
    
    let validate = move || {
        if !*is_touched.get() {
            return;
        }
        
        let val = value.get();
        for rule in &rules {
            if let Some(err) = (rule.validate)(&val) {
                error.set(Some(err));
                return;
            }
        }
        error.set(None);
    };
    
    view! {
        div(class="mb-4") {
            label(class="block text-sm font-medium text-gray-700 mb-1") {
                (label)
            }
            
            input(
                type="text",
                class=if error.get().is_some() {
                    "w-full px-3 py-2 border border-red-500 rounded focus:outline-none focus:ring-2 focus:ring-red-500"
                } else {
                    "w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
                },
                placeholder=placeholder,
                bind:value=value,
                on:blur=move |_| {
                    is_touched.set(true);
                    validate();
                },
                on:input=move |_| {
                    if *is_touched.get() {
                        validate();
                    }
                }
            )
            
            (if let Some(err) = error.get().as_ref() {
                view! {
                    p(class="mt-1 text-sm text-red-600") {
                        (err)
                    }
                }
            } else {
                view! {}
            })
        }
    }
}

// 使用例
fn example_form() -> View {
    let email = create_signal(String::new());
    
    let email_rules = vec![
        ValidationRule {
            validate: |val| {
                if val.is_empty() {
                    Some("メールアドレスは必須です".to_string())
                } else {
                    None
                }
            }
        },
        ValidationRule {
            validate: |val| {
                if !val.contains('@') {
                    Some("有効なメールアドレスを入力してください".to_string())
                } else {
                    None
                }
            }
        },
    ];
    
    view! {
        ValidatedInput(
            label="メールアドレス",
            placeholder="example@email.com",
            value=email,
            rules=email_rules
        )
    }
}
```

## 💬 通知・トースト

### トースト通知システム

```rust
use std::time::Duration;

#[derive(Clone, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Clone)]
pub struct Toast {
    pub id: usize,
    pub message: String,
    pub toast_type: ToastType,
}

#[component]
pub fn ToastContainer() -> View {
    let toasts = create_signal(Vec::<Toast>::new());
    
    let add_toast = move |message: String, toast_type: ToastType| {
        let id = toasts.get().len();
        toasts.update(|t| {
            t.push(Toast {
                id,
                message,
                toast_type,
            })
        });
        
        // 3秒後に自動削除
        spawn_local(async move {
            sleep(Duration::from_secs(3)).await;
            toasts.update(|t| t.retain(|toast| toast.id != id));
        });
    };
    
    view! {
        div(class="fixed top-4 right-4 z-50 space-y-2") {
            Indexed(
                iterable=toasts,
                view=|toast| {
                    let type_classes = match toast.toast_type {
                        ToastType::Success => "bg-green-500",
                        ToastType::Error => "bg-red-500",
                        ToastType::Warning => "bg-yellow-500",
                        ToastType::Info => "bg-blue-500",
                    };
                    
                    view! {
                        div(
                            class=format!(
                                "px-6 py-3 rounded-lg shadow-lg text-white {} animate-slide-in",
                                type_classes
                            )
                        ) {
                            p { (toast.message) }
                        }
                    }
                }
            )
        }
    }
}
```

## 🎴 カードコンポーネント

```rust
#[component]
pub fn Card(
    title: &'static str,
    subtitle: Option<&'static str>,
    children: Children,
) -> View {
    view! {
        div(class="bg-white rounded-lg shadow-md hover:shadow-lg transition-shadow duration-300 overflow-hidden") {
            div(class="p-6") {
                h3(class="text-xl font-bold text-gray-900 mb-2") {
                    (title)
                }
                
                (if let Some(sub) = subtitle {
                    view! {
                        p(class="text-sm text-gray-600 mb-4") { (sub) }
                    }
                } else {
                    view! {}
                })
                
                div(class="mt-4") {
                    (children)
                }
            }
        }
    }
}
```

## 📊 データ表示パターン

### テーブル

```rust
#[derive(Clone)]
pub struct TableColumn {
    pub header: String,
    pub accessor: fn(&Row) -> String,
}

#[component]
pub fn Table<T: Clone>(
    columns: Vec<TableColumn>,
    data: ReadSignal<Vec<T>>,
) -> View {
    view! {
        div(class="overflow-x-auto") {
            table(class="min-w-full divide-y divide-gray-200") {
                thead(class="bg-gray-50") {
                    tr {
                        Indexed(
                            iterable=create_signal(columns.clone()),
                            view=|col| view! {
                                th(class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider") {
                                    (col.header)
                                }
                            }
                        )
                    }
                }
                
                tbody(class="bg-white divide-y divide-gray-200") {
                    Indexed(
                        iterable=data,
                        view=move |row| view! {
                            tr(class="hover:bg-gray-50") {
                                Indexed(
                                    iterable=create_signal(columns.clone()),
                                    view=move |col| {
                                        let value = (col.accessor)(&row);
                                        view! {
                                            td(class="px-6 py-4 whitespace-nowrap text-sm text-gray-900") {
                                                (value)
                                            }
                                        }
                                    }
                                )
                            }
                        }
                    )
                }
            }
        }
    }
}
```

### グリッドレイアウト

```rust
#[component]
pub fn GridLayout(
    items: ReadSignal<Vec<String>>,
) -> View {
    view! {
        div(class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6") {
            Indexed(
                iterable=items,
                view=|item| view! {
                    Card(title=&item) {
                        p { "カード内容" }
                    }
                }
            )
        }
    }
}
```

## 🎭 モーダル

```rust
#[component]
pub fn Modal(
    is_open: Signal<bool>,
    title: &'static str,
    children: Children,
) -> View {
    view! {
        (if *is_open.get() {
            view! {
                div(
                    class="fixed inset-0 z-50 overflow-y-auto",
                    on:click=move |_| is_open.set(false)
                ) {
                    // オーバーレイ
                    div(class="fixed inset-0 bg-black bg-opacity-50 transition-opacity") {}
                    
                    // モーダルコンテンツ
                    div(class="flex items-center justify-center min-h-screen p-4") {
                        div(
                            class="relative bg-white rounded-lg shadow-xl max-w-lg w-full transform transition-all",
                            on:click=|e| e.stop_propagation()
                        ) {
                            // ヘッダー
                            div(class="px-6 py-4 border-b") {
                                h3(class="text-lg font-semibold") { (title) }
                                button(
                                    class="absolute top-4 right-4 text-gray-400 hover:text-gray-600",
                                    on:click=move |_| is_open.set(false)
                                ) {
                                    "✕"
                                }
                            }
                            
                            // ボディ
                            div(class="px-6 py-4") {
                                (children)
                            }
                            
                            // フッター
                            div(class="px-6 py-4 border-t bg-gray-50 flex justify-end gap-2") {
                                Button(
                                    variant=ButtonVariant::Secondary,
                                    size=ButtonSize::Medium,
                                    disabled=create_signal(false),
                                    on_click=move || is_open.set(false)
                                ) {
                                    "キャンセル"
                                }
                                Button(
                                    variant=ButtonVariant::Primary,
                                    size=ButtonSize::Medium,
                                    disabled=create_signal(false),
                                    on_click=move || {
                                        // 処理
                                        is_open.set(false)
                                    }
                                ) {
                                    "確定"
                                }
                            }
                        }
                    }
                }
            }
        } else {
            view! {}
        })
    }
}
```

## 🎨 アニメーションパターン

### CSS定義（globals.css）

```css
@keyframes slide-in {
    from {
        transform: translateX(100%);
        opacity: 0;
    }
    to {
        transform: translateX(0);
        opacity: 1;
    }
}

@keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
}

@keyframes scale-in {
    from {
        transform: scale(0.9);
        opacity: 0;
    }
    to {
        transform: scale(1);
        opacity: 1;
    }
}

.animate-slide-in {
    animation: slide-in 0.3s ease-out;
}

.animate-fade-in {
    animation: fade-in 0.2s ease-in;
}

.animate-scale-in {
    animation: scale-in 0.2s ease-out;
}

.spinner {
    display: inline-block;
    width: 1em;
    height: 1em;
    border: 2px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
}

@keyframes spin {
    to { transform: rotate(360deg); }
}
```

## 🌈 アクセシビリティパターン

### スクリーンリーダー対応

```rust
#[component]
fn AccessibleButton() -> View {
    view! {
        button(
            aria-label="メニューを開く",
            aria-expanded="false",
            role="button"
        ) {
            span(aria-hidden="true") { "☰" }
        }
    }
}
```

### キーボードナビゲーション

```rust
#[component]
fn KeyboardAccessible() -> View {
    view! {
        div(
            tabindex="0",
            on:keydown=|e| {
                if e.key() == "Enter" || e.key() == " " {
                    // アクション実行
                }
            }
        ) {
            "キーボードでアクセス可能"
        }
    }
}
```

## 📱 レスポンシブパターン

```rust
#[component]
fn ResponsiveNav() -> View {
    let is_mobile_menu_open = create_signal(false);
    
    view! {
        nav(class="bg-white shadow-lg") {
            div(class="max-w-7xl mx-auto px-4") {
                div(class="flex justify-between items-center h-16") {
                    // ロゴ
                    div(class="flex-shrink-0") {
                        h1(class="text-xl font-bold") { "Logo" }
                    }
                    
                    // デスクトップメニュー
                    div(class="hidden md:flex space-x-8") {
                        a(href="#", class="text-gray-700 hover:text-blue-600") { "Home" }
                        a(href="#", class="text-gray-700 hover:text-blue-600") { "About" }
                        a(href="#", class="text-gray-700 hover:text-blue-600") { "Contact" }
                    }
                    
                    // モバイルメニューボタン
                    button(
                        class="md:hidden",
                        on:click=move |_| is_mobile_menu_open.update(|v| *v = !*v)
                    ) {
                        "☰"
                    }
                }
                
                // モバイルメニュー
                (if *is_mobile_menu_open.get() {
                    view! {
                        div(class="md:hidden pb-4") {
                            a(href="#", class="block py-2 text-gray-700") { "Home" }
                            a(href="#", class="block py-2 text-gray-700") { "About" }
                            a(href="#", class="block py-2 text-gray-700") { "Contact" }
                        }
                    }
                } else {
                    view! {}
                })
            }
        }
    }
}
```

## 🔗 次のステップ

- **[Component Library](component_library.md)** - コンポーネントライブラリ設計
- **[Animations](animations.md)** - 高度なアニメーション
- **[Accessibility](accessibility.md)** - アクセシビリティ詳細

---

**参考**: これらのパターンは、Material Design、Tailwind UI、Ant Designなどの優れたデザインシステムの原則に基づいています。
