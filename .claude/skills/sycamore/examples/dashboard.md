# Dashboard UI Example

Vantage MCP Web Console向けの実践的なダッシュボードUI実装例です。

## 📊 ダッシュボード構成

```
Dashboard
├── Header（ヘッダー・ナビゲーション）
├── Sidebar（サイドバーメニュー）
├── Main Content
│   ├── Stats Cards（統計カード）
│   ├── Process List（プロセス一覧）
│   └── Activity Log（アクティビティログ）
└── Footer
```

## 🎨 完全実装

### src/main.rs

```rust
use sycamore::prelude::*;

mod components;
use components::*;

#[component]
fn App() -> View {
    // グローバル状態
    let processes = create_signal(vec![
        Process {
            id: "proc-1".to_string(),
            name: "Web Server".to_string(),
            status: ProcessStatus::Running,
            cpu: 45.2,
            memory: 256,
        },
        Process {
            id: "proc-2".to_string(),
            name: "Database".to_string(),
            status: ProcessStatus::Running,
            cpu: 23.5,
            memory: 512,
        },
        Process {
            id: "proc-3".to_string(),
            name: "Worker".to_string(),
            status: ProcessStatus::Stopped,
            cpu: 0.0,
            memory: 0,
        },
    ]);
    
    let sidebar_open = create_signal(true);
    
    view! {
        div(class="min-h-screen bg-gray-100") {
            Header(sidebar_open=sidebar_open) {}
            
            div(class="flex") {
                Sidebar(is_open=sidebar_open) {}
                
                main(
                    class=if *sidebar_open.get() {
                        "flex-1 p-6 ml-64 transition-all duration-300"
                    } else {
                        "flex-1 p-6 transition-all duration-300"
                    }
                ) {
                    DashboardContent(processes=processes) {}
                }
            }
        }
    }
}

fn main() {
    sycamore::render(App);
}
```

### src/components/header.rs

```rust
use sycamore::prelude::*;

#[component]
pub fn Header(sidebar_open: Signal<bool>) -> View {
    view! {
        header(class="bg-white shadow-md fixed top-0 left-0 right-0 z-40") {
            div(class="flex items-center justify-between px-6 py-4") {
                div(class="flex items-center gap-4") {
                    // サイドバートグル
                    button(
                        class="text-gray-600 hover:text-gray-900 focus:outline-none",
                        on:click=move |_| sidebar_open.update(|v| *v = !*v),
                        aria-label="Toggle sidebar"
                    ) {
                        (if *sidebar_open.get() {
                            "☰"
                        } else {
                            "☰"
                        })
                    }
                    
                    // ロゴ
                    div(class="flex items-center gap-2") {
                        div(class="w-8 h-8 bg-blue-600 rounded flex items-center justify-center text-white font-bold") {
                            "V"
                        }
                        h1(class="text-xl font-bold text-gray-900") {
                            "Vantage MCP"
                        }
                    }
                }
                
                // ヘッダーアクション
                div(class="flex items-center gap-4") {
                    // 通知
                    button(class="relative p-2 text-gray-600 hover:text-gray-900") {
                        span(class="text-xl") { "🔔" }
                        span(class="absolute top-1 right-1 w-2 h-2 bg-red-500 rounded-full") {}
                    }
                    
                    // ユーザーメニュー
                    button(class="flex items-center gap-2 px-3 py-2 rounded hover:bg-gray-100") {
                        div(class="w-8 h-8 bg-gray-300 rounded-full flex items-center justify-center") {
                            "👤"
                        }
                        span(class="text-sm font-medium") { "Admin" }
                    }
                }
            }
        }
    }
}
```

### src/components/sidebar.rs

```rust
use sycamore::prelude::*;

#[component]
pub fn Sidebar(is_open: ReadSignal<bool>) -> View {
    let current_page = create_signal("dashboard".to_string());
    
    let menu_items = vec![
        ("dashboard", "📊", "ダッシュボード"),
        ("processes", "⚙️", "プロセス"),
        ("logs", "📝", "ログ"),
        ("settings", "⚙️", "設定"),
    ];
    
    view! {
        aside(
            class=if *is_open.get() {
                "fixed left-0 top-16 bottom-0 w-64 bg-white shadow-lg transform translate-x-0 transition-transform duration-300 z-30"
            } else {
                "fixed left-0 top-16 bottom-0 w-64 bg-white shadow-lg transform -translate-x-full transition-transform duration-300 z-30"
            }
        ) {
            nav(class="p-4") {
                Indexed(
                    iterable=create_signal(menu_items),
                    view=move |(id, icon, label)| {
                        let is_active = create_memo(move || {
                            *current_page.get() == id
                        });
                        
                        view! {
                            button(
                                class=if *is_active.get() {
                                    "w-full flex items-center gap-3 px-4 py-3 mb-2 rounded-lg bg-blue-50 text-blue-600 font-medium"
                                } else {
                                    "w-full flex items-center gap-3 px-4 py-3 mb-2 rounded-lg hover:bg-gray-100 text-gray-700"
                                },
                                on:click=move |_| current_page.set(id.to_string())
                            ) {
                                span(class="text-xl") { (icon) }
                                span { (label) }
                            }
                        }
                    }
                )
            }
        }
    }
}
```

### src/components/dashboard_content.rs

```rust
use sycamore::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Failed,
}

#[derive(Clone)]
pub struct Process {
    pub id: String,
    pub name: String,
    pub status: ProcessStatus,
    pub cpu: f64,
    pub memory: usize,
}

#[component]
pub fn DashboardContent(processes: ReadSignal<Vec<Process>>) -> View {
    // 統計計算
    let stats = create_memo(move || {
        let procs = processes.get();
        let running = procs.iter().filter(|p| p.status == ProcessStatus::Running).count();
        let stopped = procs.iter().filter(|p| p.status == ProcessStatus::Stopped).count();
        let total_cpu = procs.iter().map(|p| p.cpu).sum::<f64>();
        let total_memory = procs.iter().map(|p| p.memory).sum::<usize>();
        
        (running, stopped, total_cpu, total_memory)
    });
    
    view! {
        div(class="space-y-6 mt-16") {
            // ページタイトル
            h2(class="text-2xl font-bold text-gray-900") {
                "ダッシュボード"
            }
            
            // 統計カード
            div(class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6") {
                StatCard(
                    title="実行中",
                    value=create_memo(move || stats.get().0.to_string()),
                    icon="▶️",
                    color="blue"
                ) {}
                
                StatCard(
                    title="停止中",
                    value=create_memo(move || stats.get().1.to_string()),
                    icon="⏸️",
                    color="gray"
                ) {}
                
                StatCard(
                    title="CPU使用率",
                    value=create_memo(move || format!("{:.1}%", stats.get().2)),
                    icon="📊",
                    color="green"
                ) {}
                
                StatCard(
                    title="メモリ使用量",
                    value=create_memo(move || format!("{} MB", stats.get().3)),
                    icon="💾",
                    color="purple"
                ) {}
            }
            
            // プロセス一覧
            div(class="bg-white rounded-lg shadow-md p-6") {
                h3(class="text-lg font-semibold mb-4") {
                    "プロセス一覧"
                }
                
                ProcessTable(processes=processes) {}
            }
        }
    }
}

#[component]
fn StatCard(
    title: &'static str,
    value: ReadSignal<String>,
    icon: &'static str,
    color: &'static str,
) -> View {
    let bg_color = match color {
        "blue" => "bg-blue-500",
        "gray" => "bg-gray-500",
        "green" => "bg-green-500",
        "purple" => "bg-purple-500",
        _ => "bg-gray-500",
    };
    
    view! {
        div(class="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow") {
            div(class="flex items-center justify-between") {
                div {
                    p(class="text-sm text-gray-600 mb-1") { (title) }
                    p(class="text-3xl font-bold text-gray-900") {
                        (value.get())
                    }
                }
                
                div(class=format!("w-12 h-12 {} rounded-full flex items-center justify-center text-2xl", bg_color)) {
                    (icon)
                }
            }
        }
    }
}

#[component]
fn ProcessTable(processes: ReadSignal<Vec<Process>>) -> View {
    view! {
        div(class="overflow-x-auto") {
            table(class="min-w-full") {
                thead {
                    tr(class="border-b") {
                        th(class="text-left py-3 px-4 text-sm font-medium text-gray-700") { "プロセス名" }
                        th(class="text-left py-3 px-4 text-sm font-medium text-gray-700") { "ステータス" }
                        th(class="text-left py-3 px-4 text-sm font-medium text-gray-700") { "CPU" }
                        th(class="text-left py-3 px-4 text-sm font-medium text-gray-700") { "メモリ" }
                        th(class="text-left py-3 px-4 text-sm font-medium text-gray-700") { "アクション" }
                    }
                }
                
                tbody {
                    Indexed(
                        iterable=processes,
                        view=|process| {
                            let status_badge = match process.status {
                                ProcessStatus::Running => ("bg-green-100 text-green-800", "実行中"),
                                ProcessStatus::Stopped => ("bg-gray-100 text-gray-800", "停止中"),
                                ProcessStatus::Failed => ("bg-red-100 text-red-800", "失敗"),
                            };
                            
                            view! {
                                tr(class="border-b hover:bg-gray-50") {
                                    td(class="py-3 px-4") {
                                        div(class="flex items-center gap-2") {
                                            span(class="font-medium") { (process.name) }
                                        }
                                    }
                                    
                                    td(class="py-3 px-4") {
                                        span(class=format!("px-2 py-1 rounded-full text-xs font-medium {}", status_badge.0)) {
                                            (status_badge.1)
                                        }
                                    }
                                    
                                    td(class="py-3 px-4 text-sm text-gray-600") {
                                        (format!("{:.1}%", process.cpu))
                                    }
                                    
                                    td(class="py-3 px-4 text-sm text-gray-600") {
                                        (format!("{} MB", process.memory))
                                    }
                                    
                                    td(class="py-3 px-4") {
                                        div(class="flex gap-2") {
                                            (if process.status == ProcessStatus::Running {
                                                view! {
                                                    button(class="px-3 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600") {
                                                        "停止"
                                                    }
                                                }
                                            } else {
                                                view! {
                                                    button(class="px-3 py-1 text-sm bg-green-500 text-white rounded hover:bg-green-600") {
                                                        "開始"
                                                    }
                                                }
                                            })
                                            
                                            button(class="px-3 py-1 text-sm bg-gray-500 text-white rounded hover:bg-gray-600") {
                                                "詳細"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    )
                }
            }
        }
    }
}
```

## 🎨 スタイリング (globals.css)

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer components {
    .btn-primary {
        @apply px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors;
    }
    
    .btn-secondary {
        @apply px-4 py-2 bg-gray-200 text-gray-900 rounded hover:bg-gray-300 transition-colors;
    }
}

/* アニメーション */
@keyframes pulse {
    0%, 100% {
        opacity: 1;
    }
    50% {
        opacity: 0.5;
    }
}

.animate-pulse {
    animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}
```

## 📱 レスポンシブ対応

このダッシュボードは以下のブレークポイントで最適化されています：

- **Mobile (< 768px)**: シングルカラム、サイドバーはオーバーレイ
- **Tablet (768px - 1024px)**: 2カラムグリッド
- **Desktop (> 1024px)**: 4カラムグリッド、サイドバー固定

## 🚀 実行方法

```bash
# プロジェクトディレクトリで
trunk serve
```

→ `http://localhost:8080` でダッシュボードにアクセス

## 🔄 API統合の追加

実際のVantage MCPと統合する場合：

```rust
use wasm_bindgen_futures::spawn_local;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ProcessResponse {
    processes: Vec<Process>,
}

async fn fetch_processes() -> Result<Vec<Process>, String> {
    let response = reqwest::get("http://localhost:12700/api/processes")
        .await
        .map_err(|e| e.to_string())?;
    
    let data: ProcessResponse = response
        .json()
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(data.processes)
}

// コンポーネントで使用
#[component]
fn ProcessList() -> View {
    let processes = create_signal(Vec::new());
    let loading = create_signal(true);
    
    // 初回読み込み
    spawn_local(async move {
        match fetch_processes().await {
            Ok(procs) => {
                processes.set(procs);
                loading.set(false);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                loading.set(false);
            }
        }
    });
    
    view! {
        (if *loading.get() {
            view! { div { "読み込み中..." } }
        } else {
            view! { ProcessTable(processes=processes) {} }
        })
    }
}
```

## 🎯 次のステップ

1. **リアルタイム更新**: WebSocketでプロセス状態の自動更新
2. **フィルタリング**: プロセス一覧のフィルター・ソート機能
3. **詳細ビュー**: 個別プロセスの詳細情報表示
4. **グラフ**: CPU/メモリ使用率のチャート表示

---

このダッシュボードは、UI/UXのベストプラクティスに基づいて設計されており、実際のVantage MCP Web Consoleの基盤として使用できます。
