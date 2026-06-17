use dioxus::prelude::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::frontend::components::{ProjectsTab, IssuesTab, VaultTab, LogsTab, FeedbackModal, ChatTab};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProjectInfo {
    pub path: String,
    pub goal: String,
    pub pid: u32,
    pub status: String,
    pub spawned_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Issue {
    pub id: u32,
    pub title: String,
    pub body: String,
    pub status: String, // "open", "in-progress", "resolved", "failed"
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub is_user: bool,
    pub text: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HealthCheckResult {
    pub target: String,
    pub healthy: bool,
    pub message: String,
    pub checked_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FeedbackResponse {
    Submitted { title: String, url: String },
    PrefilledUrl { title: String, body: String, url: String },
}

#[derive(Clone, PartialEq, Debug)]
pub enum UpgradeProgress {
    Idle,
    Downloading,
    Installing,
    Restarting,
    Success,
    Failed(String),
}

async fn sleep_ms(ms: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(ms).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    }
}

#[cfg(target_arch = "wasm32")]
fn reload_page() {
    spawn(async move {
        let mut eval_js = document::eval("window.location.reload();");
        let _ = eval_js.recv::<()>().await;
    });
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    let mut active_tab = use_signal(|| "projects".to_string());
    let mut projects = use_signal(HashMap::new);
    let mut issues = use_signal(Vec::new);
    let mut logs = use_signal(String::new);
    let mut vault_notes = use_signal(Vec::new);
    let mut system_health = use_signal(Vec::new);
    let mut daemon_running = use_signal(|| false);
    let mut upgrade_available = use_signal(|| None::<(String, String)>);
    let mut show_feedback_modal = use_signal(|| false);
    let mut upgrade_progress = use_signal(|| UpgradeProgress::Idle);
    let mut chat_messages = use_signal(HashMap::<String, Vec<ChatMessage>>::new);
    let mut active_session_id = use_signal(|| None::<String>);
    let mut chat_sessions = use_signal(Vec::<ChatSession>::new);

    // Load chat history once on mount
    let _chat_init = use_future(move || async move {
        if let Ok(sessions) = crate::get_chat_sessions().await {
            chat_sessions.set(sessions);
        }
        if let Ok(Some(active_id)) = crate::get_active_session_id().await {
            active_session_id.set(Some(active_id.clone()));
            if let Ok(history) = crate::get_chat_history(active_id.clone()).await {
                let mut map = HashMap::new();
                map.insert(active_id, history);
                chat_messages.set(map);
            }
        }
    });

    // Poll data periodically
    let _fetch_future = use_future(move || async move {
        loop {
            if let Ok(p) = crate::get_projects().await {
                projects.set(p);
            }
            if let Ok(i) = crate::get_issues().await {
                issues.set(i);
            }
            if let Ok(l) = crate::get_logs().await {
                logs.set(l);
            }
            if let Ok(vn) = crate::get_vault_notes().await {
                vault_notes.set(vn);
            }
            if let Ok(sh) = crate::get_system_health().await {
                system_health.set(sh);
            }
            if let Ok(dr) = crate::get_daemon_status().await {
                daemon_running.set(dr);
            }
            if let Ok(upg) = crate::get_upgrade_status().await {
                upgrade_available.set(upg);
            }
            sleep_ms(3000).await;
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("assets/tailwind.css") }
        document::Link { rel: "stylesheet", href: "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" }

        div { class: "bg-slate-950 text-slate-100 h-screen overflow-hidden font-sans flex flex-col selection:bg-indigo-500 selection:text-white",
            // Header Bar
            header { class: "bg-slate-900/80 backdrop-blur-md border-b border-slate-800/80 px-6 py-4 flex items-center justify-between sticky top-0 z-50",
                div { class: "flex items-center gap-3",
                    span { class: "text-2xl", "🤖" }
                    h1 { class: "text-xl font-bold tracking-tight bg-gradient-to-r from-indigo-400 via-purple-400 to-indigo-300 bg-clip-text text-transparent",
                        "AGY Orchestrator Dashboard"
                    }
                    span { class: "text-[10px] bg-slate-800 text-slate-400 px-2 py-0.5 rounded-md font-mono font-semibold",
                        "v{env!(\"AGY_ORCHESTRATOR_VERSION\")}"
                    }
                    if let Some((tag_name, download_url)) = upgrade_available.read().clone() {
                        button {
                            class: "text-[10px] bg-indigo-600/20 hover:bg-indigo-600/40 text-indigo-300 border border-indigo-500/20 px-2.5 py-0.5 rounded-full font-bold animate-pulse active:scale-95 transition-all shadow shadow-indigo-900/40 cursor-pointer",
                            onclick: move |_| {
                                let url = download_url.clone();
                                let mut progress = upgrade_progress;
                                spawn(async move {
                                    progress.set(UpgradeProgress::Downloading);
                                    sleep_ms(500).await;
                                    progress.set(UpgradeProgress::Installing);
                                    match crate::trigger_remote_upgrade(url).await {
                                        Ok(_) => {
                                            progress.set(UpgradeProgress::Restarting);
                                            sleep_ms(4000).await;
                                            progress.set(UpgradeProgress::Success);
                                            #[cfg(target_arch = "wasm32")]
                                            reload_page();
                                        }
                                        Err(e) => {
                                            progress.set(UpgradeProgress::Failed(e.to_string()));
                                        }
                                    }
                                });
                            },
                            "Update to {tag_name} 🚀"
                        }
                    }
                }
                div { class: "flex items-center gap-4",
                    div { class: "flex items-center gap-2 bg-slate-950/60 border border-slate-800/80 px-3 py-1.5 rounded-full text-xs font-semibold",
                        span { class: "text-slate-400", "Daemon Status:" }
                        if *daemon_running.read() {
                            span { class: "text-emerald-400 flex items-center gap-1.5",
                                span { class: "h-2 w-2 rounded-full bg-emerald-400 animate-pulse" }
                                "RUNNING"
                            }
                        } else {
                            span { class: "text-rose-400 flex items-center gap-1.5",
                                span { class: "h-2 w-2 rounded-full bg-rose-400" }
                                "STOPPED"
                            }
                        }
                    }
                    button {
                        class: "px-4 py-1.5 rounded-full text-xs font-bold transition-all duration-200 active:scale-95 border "
                            .to_string() + if *daemon_running.read() {
                                "bg-rose-500/10 hover:bg-rose-500/20 text-rose-300 border-rose-500/20"
                            } else {
                                "bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-300 border-emerald-500/20"
                            },
                        onclick: move |_| async move {
                            if let Ok(new_status) = crate::toggle_daemon().await {
                                daemon_running.set(new_status);
                            }
                        },
                        if *daemon_running.read() { "Stop Daemon" } else { "Start Daemon" }
                    }
                }
            }

            // Main Body Area
            div { class: "flex-1 flex overflow-hidden",
                // Sidebar Navigation
                nav { class: "w-64 bg-slate-900/40 border-r border-slate-850 p-4 flex flex-col justify-between shrink-0",
                    div { class: "flex flex-col gap-2",
                        button {
                            class: "flex items-center gap-3 px-4 py-3 rounded-xl font-medium transition-all duration-200 "
                                .to_string() + if *active_tab.read() == "projects" {
                                    "bg-indigo-600/20 text-indigo-200 border-l-4 border-indigo-500"
                                } else {
                                    "hover:bg-slate-800/50 text-slate-400 hover:text-slate-200 border-l-4 border-transparent"
                                },
                            onclick: move |_| active_tab.set("projects".to_string()),
                            span { "📂" }
                            "Active Projects"
                        }
                        button {
                            class: "flex items-center gap-3 px-4 py-3 rounded-xl font-medium transition-all duration-200 "
                                .to_string() + if *active_tab.read() == "issues" {
                                    "bg-indigo-600/20 text-indigo-200 border-l-4 border-indigo-500"
                                } else {
                                    "hover:bg-slate-800/50 text-slate-400 hover:text-slate-200 border-l-4 border-transparent"
                                },
                            onclick: move |_| active_tab.set("issues".to_string()),
                            span { "📋" }
                            "Kanban Issues"
                        }
                        button {
                            class: "flex items-center gap-3 px-4 py-3 rounded-xl font-medium transition-all duration-200 "
                                .to_string() + if *active_tab.read() == "vault" {
                                    "bg-indigo-600/20 text-indigo-200 border-l-4 border-indigo-500"
                                } else {
                                    "hover:bg-slate-800/50 text-slate-400 hover:text-slate-200 border-l-4 border-transparent"
                                },
                            onclick: move |_| active_tab.set("vault".to_string()),
                            span { "🗂️" }
                            "Knowledge Vault"
                        }
                        button {
                            class: "flex items-center gap-3 px-4 py-3 rounded-xl font-medium transition-all duration-200 "
                                .to_string() + if *active_tab.read() == "logs" {
                                    "bg-indigo-600/20 text-indigo-200 border-l-4 border-indigo-500"
                                } else {
                                    "hover:bg-slate-800/50 text-slate-400 hover:text-slate-200 border-l-4 border-transparent"
                                },
                            onclick: move |_| active_tab.set("logs".to_string()),
                            span { "📟" }
                            "Live Logs"
                        }
                        button {
                            class: "flex items-center gap-3 px-4 py-3 rounded-xl font-medium transition-all duration-200 "
                                .to_string() + if *active_tab.read() == "chat" {
                                    "bg-indigo-600/20 text-indigo-200 border-l-4 border-indigo-500"
                                } else {
                                    "hover:bg-slate-800/50 text-slate-400 hover:text-slate-200 border-l-4 border-transparent"
                                },
                            onclick: move |_| {
                                active_tab.set("chat".to_string());
                                let mut msgs = chat_messages;
                                let mut sessions_sig = chat_sessions;
                                let mut active_sig = active_session_id;
                                spawn(async move {
                                    if let Ok(sessions) = crate::get_chat_sessions().await {
                                        sessions_sig.set(sessions);
                                    }
                                    match crate::get_active_session_id().await {
                                        Ok(Some(active_id)) => {
                                            active_sig.set(Some(active_id.clone()));
                                            if let Ok(history) = crate::get_chat_history(active_id.clone()).await {
                                                msgs.write().insert(active_id, history);
                                            }
                                        }
                                        Ok(None) => {
                                            active_sig.set(None);
                                        }
                                        Err(_) => {}
                                    }
                                });
                            },
                            span { "💬" }
                            "Chat Assistant"
                        }
                    }
                    button {
                        class: "flex items-center gap-3 px-4 py-3 rounded-xl font-medium transition-all duration-200 hover:bg-slate-800/40 text-slate-400 hover:text-slate-200 border border-slate-800/30 hover:border-slate-700/60 shadow shadow-slate-950/20 active:scale-95 mb-2 cursor-pointer",
                        onclick: move |_| show_feedback_modal.set(true),
                        span { "💬" }
                        "Report Feedback"
                    }
                }

                // Tab Content Panel
                main {
                    class: "flex-1 p-8 flex flex-col overflow-hidden h-full",
                    match active_tab.read().as_str() {
                        "projects" => rsx! {
                            ProjectsTab {
                                projects: projects,
                                system_health: system_health
                            }
                        },
                        "issues" => rsx! {
                            IssuesTab {
                                issues: issues
                            }
                        },
                        "vault" => rsx! {
                            VaultTab {
                                vault_notes: vault_notes,
                                projects: projects
                            }
                        },
                        "logs" => rsx! {
                            LogsTab {
                                logs: logs
                            }
                        },
                        "chat" => rsx! {
                            ChatTab {
                                messages: chat_messages,
                                issues: issues,
                                active_session_id: active_session_id,
                                chat_sessions: chat_sessions
                            }
                        },
                        _ => rsx! { div { "Unknown tab" } }
                    }
                }
            }
            FeedbackModal { show_modal: show_feedback_modal }
            if *upgrade_progress.read() != UpgradeProgress::Idle {
                div { class: "fixed inset-0 z-[100] flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm transition-all duration-300",
                    div { class: "w-full max-w-md bg-slate-900 border border-slate-800 rounded-2xl shadow-2xl p-6 flex flex-col gap-5 text-center animate-in fade-in-50 zoom-in-95",
                        match upgrade_progress.read().clone() {
                            UpgradeProgress::Idle => rsx! { div {} },
                            UpgradeProgress::Downloading => rsx! {
                                div { class: "flex flex-col items-center gap-4 py-4",
                                    div { class: "h-12 w-12 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin" }
                                    h3 { class: "text-lg font-bold text-slate-100", "패키지 다운로드 중..." }
                                    p { class: "text-xs text-slate-400 leading-relaxed", "GitHub로부터 최신 릴리즈 압축 패키지를 안전하게 내려받고 있습니다." }
                                }
                            },
                            UpgradeProgress::Installing => rsx! {
                                div { class: "flex flex-col items-center gap-4 py-4",
                                    div { class: "h-12 w-12 border-4 border-purple-500 border-t-transparent rounded-full animate-spin" }
                                    h3 { class: "text-lg font-bold text-slate-100", "업데이트 설치 중..." }
                                    p { class: "text-xs text-slate-400 leading-relaxed", "바이너리를 백업하고 신규 파일 압축 해제 및 무결성 검사를 수행하는 중입니다." }
                                }
                            },
                            UpgradeProgress::Restarting => rsx! {
                                div { class: "flex flex-col items-center gap-4 py-4",
                                    div { class: "h-12 w-12 border-4 border-emerald-500 border-t-transparent rounded-full animate-spin" }
                                    h3 { class: "text-lg font-bold text-emerald-400", "대시보드 재기동 중..." }
                                    p { class: "text-xs text-slate-400 leading-relaxed", "업그레이드가 성공적으로 완료되었습니다! 대시보드 서비스를 다시 로드하고 있습니다. 잠시만 기다려주세요." }
                                }
                            },
                            UpgradeProgress::Success => rsx! {
                                div { class: "flex flex-col items-center gap-4 py-4",
                                    span { class: "text-5xl", "🎉" }
                                    h3 { class: "text-lg font-bold text-emerald-400", "업그레이드 완료!" }
                                    p { class: "text-xs text-slate-300", "대시보드를 새로고침합니다..." }
                                }
                            },
                            UpgradeProgress::Failed(err) => rsx! {
                                div { class: "flex flex-col gap-4 py-2",
                                    span { class: "text-5xl", "⚠️" }
                                    h3 { class: "text-lg font-bold text-rose-400", "업그레이드 실패" }
                                    div { class: "bg-slate-950/60 border border-slate-800/80 rounded-xl p-3.5 text-left max-h-[150px] overflow-y-auto scrollbar-thin scrollbar-thumb-slate-800",
                                        p { class: "text-xs text-rose-300 font-mono whitespace-pre-wrap leading-relaxed", "{err}" }
                                    }
                                    p { class: "text-[11px] text-slate-500 leading-relaxed", "시스템이 이전 안전 상태로 자동 롤백되었습니다. 필요한 경우 CLI를 통해 수동으로 복구해 주세요." }
                                    div { class: "flex justify-end gap-3 mt-4 border-t border-slate-800 pt-4",
                                        button {
                                            class: "px-4 py-2 text-xs font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all",
                                            onclick: move |_| upgrade_progress.set(UpgradeProgress::Idle),
                                            "확인"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
