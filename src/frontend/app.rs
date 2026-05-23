use dioxus::prelude::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

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
pub struct HealthCheckResult {
    pub target: String,
    pub healthy: bool,
    pub message: String,
    pub checked_at: String,
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

#[allow(non_snake_case)]
pub fn App() -> Element {
    let mut active_tab = use_signal(|| "projects".to_string());
    let mut projects = use_signal(HashMap::new);
    let mut issues = use_signal(Vec::new);
    let mut logs = use_signal(String::new);
    let mut vault_notes = use_signal(Vec::new);
    let mut system_health = use_signal(Vec::new);
    let mut daemon_running = use_signal(|| false);

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
            sleep_ms(3000).await;
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
        document::Link { rel: "stylesheet", href: "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" }

        div { class: "bg-slate-950 text-slate-100 min-h-screen font-sans flex flex-col selection:bg-indigo-500 selection:text-white",
            // Header Bar
            header { class: "bg-slate-900/80 backdrop-blur-md border-b border-slate-800/80 px-6 py-4 flex items-center justify-between sticky top-0 z-50",
                div { class: "flex items-center gap-3",
                    span { class: "text-2xl", "🤖" }
                    h1 { class: "text-xl font-bold tracking-tight bg-gradient-to-r from-indigo-400 via-purple-400 to-indigo-300 bg-clip-text text-transparent",
                        "AGY Orchestrator Dashboard"
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
                nav { class: "w-64 bg-slate-900/40 border-r border-slate-850 p-4 flex flex-col gap-2 shrink-0",
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
                }

                // Tab Content Panel
                main { class: "flex-1 overflow-y-auto p-8",
                    match active_tab.read().as_str() {
                        "projects" => rsx! {
                            ProjectsTab {
                                projects: projects.clone(),
                                system_health: system_health.clone()
                            }
                        },
                        "issues" => rsx! {
                            IssuesTab {
                                issues: issues.clone()
                            }
                        },
                        "vault" => rsx! {
                            VaultTab {
                                vault_notes: vault_notes.clone(),
                                projects: projects.clone()
                            }
                        },
                        "logs" => rsx! {
                            LogsTab {
                                logs: logs.clone()
                            }
                        },
                        _ => rsx! { div { "Unknown tab" } }
                    }
                }
            }
        }
    }
}

#[component]
fn ProjectsTab(
    projects: Signal<HashMap<String, ProjectInfo>>,
    system_health: Signal<Vec<HealthCheckResult>>
) -> Element {
    let mut show_modal = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut path = use_signal(String::new);
    let mut goal = use_signal(String::new);
    let mut status_msg = use_signal(String::new);

    let projects_map = projects.read();

    // 1. Build parent-child relationships
    let mut root_projects = Vec::new();
    let mut sub_projects: HashMap<String, Vec<(String, ProjectInfo)>> = HashMap::new();

    for (name, info) in projects_map.iter() {
        if name.contains("_sub_") {
            if let Some(sub_idx) = name.rfind("_sub_") {
                let parent_name = name[..sub_idx].to_string();
                if projects_map.contains_key(&parent_name) {
                    sub_projects.entry(parent_name).or_default().push((name.clone(), info.clone()));
                    continue;
                }
            }
        }
        root_projects.push((name.clone(), info.clone()));
    }

    // Sort to ensure stable rendering order
    root_projects.sort_by(|a, b| a.0.cmp(&b.0));
    for subs in sub_projects.values_mut() {
        subs.sort_by(|a, b| a.0.cmp(&b.0));
    }

    rsx! {
        div { class: "flex flex-col gap-6",
            div { class: "flex items-center justify-between",
                div {
                    h2 { class: "text-2xl font-bold text-slate-100", "Projects & Agent Targets" }
                    p { class: "text-sm text-slate-400 mt-1", "Monitor background agent statuses and trigger new autonomous tasks." }
                }
                button {
                    class: "px-4 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-semibold transition-all duration-200 shadow-lg shadow-indigo-900/40 hover:shadow-indigo-900/60 active:scale-95 flex items-center gap-2",
                    onclick: move |_| show_modal.set(true),
                    span { class: "text-lg", "+" }
                    "New Task"
                }
            }

            // Projects List
            if root_projects.is_empty() {
                div { class: "bg-slate-900/30 border border-slate-850 rounded-2xl p-12 text-center flex flex-col items-center gap-3",
                    span { class: "text-4xl", "📭" }
                    h3 { class: "font-semibold text-slate-300 text-lg", "No projects registered" }
                    p { class: "text-slate-500 text-sm max-w-sm", "Spawn your first project target to start executing agent tasks." }
                }
            } else {
                div { class: "grid grid-cols-1 xl:grid-cols-2 gap-6",
                    for (proj_name, info) in root_projects.iter() {
                        div { class: "bg-slate-900/50 backdrop-blur-md border border-slate-800/80 rounded-2xl p-6 flex flex-col justify-between hover:border-slate-700/60 transition-all duration-200 shadow-lg shadow-slate-950/20",
                            div { class: "flex flex-col gap-4",
                                div { class: "flex items-start justify-between",
                                    div { class: "flex flex-col gap-1",
                                        h3 { class: "text-lg font-bold text-slate-200", "{proj_name}" }
                                        span { class: "text-xs font-mono text-slate-500", "PID: {info.pid}" }
                                    }
                                    // Status Badge
                                    span {
                                        class: "px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wider "
                                            .to_string() + match info.status.as_str() {
                                                "running" => "bg-sky-500/10 text-sky-400 border border-sky-500/20 animate-pulse",
                                                "completed" => "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                                "failed" => "bg-rose-500/10 text-rose-400 border border-rose-500/20",
                                                _ => "bg-slate-800 text-slate-400"
                                            },
                                        "{info.status}"
                                    }
                                }

                                div { class: "bg-slate-950/60 border border-slate-850 rounded-xl p-4 flex flex-col gap-1.5",
                                    span { class: "text-xs font-semibold text-slate-400 uppercase tracking-wider", "Current Goal" }
                                    p { class: "text-sm text-slate-300 line-clamp-3 leading-relaxed", "{info.goal}" }
                                }

                                div { class: "flex flex-col gap-1",
                                    span { class: "text-xs font-semibold text-slate-500", "Workspace Path" }
                                    span { class: "text-xs font-mono text-slate-400 truncate bg-slate-950/30 px-2 py-1 rounded border border-slate-900/80", "{info.path}" }
                                }

                                // Nested Sub-Agents
                                if let Some(subs) = sub_projects.get(proj_name) {
                                    div { class: "mt-4 border-t border-slate-800/80 pt-4 flex flex-col gap-3 pl-4 border-l-2 border-indigo-500/20 ml-2",
                                        div { class: "text-[10px] font-semibold text-slate-500 uppercase tracking-wider flex items-center gap-1.5",
                                            span { "└─" }
                                            span { "Sub-Agents" }
                                        }
                                        {
                                            subs.iter().map(|(sub_name, sub_info)| {
                                                let display_subtask_name = if let Some(sub_idx) = sub_name.rfind("_sub_") {
                                                    &sub_name[sub_idx + 5..]
                                                } else {
                                                    sub_name.as_str()
                                                };
                                                rsx! {
                                                    div { class: "bg-slate-950/40 border border-slate-850 hover:border-slate-800/60 rounded-xl p-3 flex flex-col gap-2.5 shadow-inner transition-all duration-200",
                                                        div { class: "flex items-start justify-between",
                                                            div { class: "flex flex-col gap-0.5",
                                                                h4 { class: "text-xs font-bold text-slate-300", "{display_subtask_name}" }
                                                                span { class: "text-[9px] font-mono text-slate-500", "PID: {sub_info.pid}" }
                                                            }
                                                            span {
                                                                class: "px-2 py-0.5 rounded-full text-[9px] font-bold uppercase tracking-wider "
                                                                    .to_string() + match sub_info.status.as_str() {
                                                                        "running" => "bg-sky-500/10 text-sky-400 border border-sky-500/20 animate-pulse",
                                                                        "completed" => "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                                                        "failed" => "bg-rose-500/10 text-rose-400 border border-rose-500/20",
                                                                        _ => "bg-slate-800 text-slate-400"
                                                                    },
                                                                "{sub_info.status}"
                                                            }
                                                        }
                                                        div { class: "bg-slate-950/30 border border-slate-900 rounded-lg p-2 flex flex-col gap-1",
                                                            p { class: "text-[11px] text-slate-400 leading-normal", "{sub_info.goal}" }
                                                        }
                                                        div { class: "flex items-center justify-between text-[9px] text-slate-500 border-t border-slate-900/60 pt-1.5",
                                                            span { class: "truncate max-w-[150px] font-mono", "{sub_info.path}" }
                                                            if let Some(health) = system_health.read().iter().find(|h| h.target == *sub_name) {
                                                                if health.healthy {
                                                                    span { class: "text-emerald-400 font-semibold", "✅ OK" }
                                                                } else {
                                                                    span { class: "text-rose-400 font-semibold cursor-help", title: "{health.message}", "❌ FAIL" }
                                                                }
                                                            } else {
                                                                span { class: "text-slate-400", "⏳ Pending" }
                                                            }
                                                        }
                                                    }
                                                }
                                            })
                                        }
                                    }
                                }
                            }

                            // Footer with Health check details
                            div { class: "mt-6 pt-4 border-t border-slate-850 flex items-center justify-between",
                                span { class: "text-xs text-slate-500", "Spawned: {info.spawned_at.get(..19).unwrap_or(&info.spawned_at).replace('T', \" \")}" }
                                
                                // Health status
                                if let Some(health) = system_health.read().iter().find(|h| h.target == *proj_name) {
                                    if health.healthy {
                                        span { class: "text-xs font-semibold text-emerald-400 flex items-center gap-1",
                                            "✅ OK"
                                        }
                                    } else {
                                        span {
                                            class: "text-xs font-semibold text-rose-400 flex items-center gap-1 cursor-help",
                                            title: "{health.message}",
                                            "❌ FAIL"
                                        }
                                    }
                                } else {
                                    span { class: "text-xs text-slate-400 font-medium", "⏳ Pending" }
                                }
                            }
                        }
                    }
                }
            }

            // Task Spawn Modal
            if *show_modal.read() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm transition-all duration-300",
                    div { class: "w-full max-w-lg bg-slate-900 border border-slate-800/80 rounded-2xl shadow-2xl p-6 flex flex-col gap-4 animate-in fade-in-50 zoom-in-95",
                        div { class: "flex items-center justify-between border-b border-slate-800 pb-3",
                            h3 { class: "text-lg font-bold text-slate-100", "Spawn New Autonomous Task" }
                            button {
                                class: "text-slate-400 hover:text-slate-200 transition-colors",
                                onclick: move |_| {
                                    show_modal.set(false);
                                    status_msg.set(String::new());
                                },
                                "✕"
                            }
                        }

                        div { class: "flex flex-col gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs font-semibold text-slate-400", "Project Name" }
                                input {
                                    class: "w-full bg-slate-950/85 border border-slate-800 focus:border-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 outline-none transition-all",
                                    placeholder: "e.g., custom_crawler",
                                    value: "{name}",
                                    oninput: move |e| name.set(e.value())
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs font-semibold text-slate-400", "Directory Path" }
                                input {
                                    class: "w-full bg-slate-950/85 border border-slate-800 focus:border-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 outline-none transition-all",
                                    placeholder: "e.g., /home/wimvm/works/my_project",
                                    value: "{path}",
                                    oninput: move |e| path.set(e.value())
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs font-semibold text-slate-400", "Goal & Objectives" }
                                textarea {
                                    class: "w-full bg-slate-950/85 border border-slate-800 focus:border-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 outline-none transition-all h-28 resize-none",
                                    placeholder: "Explain the details of what the agent needs to achieve...",
                                    value: "{goal}",
                                    oninput: move |e| goal.set(e.value())
                                }
                            }
                        }

                        if !status_msg.read().is_empty() {
                            p { class: "text-xs text-rose-400 font-semibold bg-rose-500/10 border border-rose-500/20 px-3 py-2 rounded-lg",
                                "{status_msg}"
                            }
                        }

                        div { class: "flex justify-end gap-3 mt-4",
                            button {
                                class: "px-4 py-2 text-sm font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all",
                                onclick: move |_| {
                                    show_modal.set(false);
                                    status_msg.set(String::new());
                                },
                                "Cancel"
                            }
                            button {
                                class: "px-4 py-2 text-sm font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-900/30 transition-all",
                                onclick: move |_| async move {
                                    if name.read().is_empty() || path.read().is_empty() || goal.read().is_empty() {
                                        status_msg.set("Please fill in all inputs.".to_string());
                                        return;
                                    }
                                    let name_val = name.read().clone();
                                    let path_val = path.read().clone();
                                    let goal_val = goal.read().clone();
                                    if let Err(e) = crate::spawn_project_task(name_val, path_val, goal_val).await {
                                        status_msg.set(format!("Failed to spawn: {}", e));
                                    } else {
                                        show_modal.set(false);
                                        name.set(String::new());
                                        path.set(String::new());
                                        goal.set(String::new());
                                        status_msg.set(String::new());
                                        if let Ok(p) = crate::get_projects().await {
                                            projects.set(p);
                                        }
                                    }
                                },
                                "Spawn Process"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn IssuesTab(issues: Signal<Vec<Issue>>) -> Element {
    let mut show_create_modal = use_signal(|| false);
    let mut new_title = use_signal(String::new);
    let mut new_body = use_signal(String::new);
    let mut error_msg = use_signal(String::new);

    let issues_list = issues.read();

    // Group issues by status
    let mut open_issues = Vec::new();
    let mut in_progress_issues = Vec::new();
    let mut resolved_issues = Vec::new();
    let mut failed_issues = Vec::new();

    for issue in issues_list.iter() {
        match issue.status.as_str() {
            "open" => open_issues.push(issue.clone()),
            "in-progress" => in_progress_issues.push(issue.clone()),
            "resolved" => resolved_issues.push(issue.clone()),
            "failed" => failed_issues.push(issue.clone()),
            _ => open_issues.push(issue.clone()),
        }
    }

    rsx! {
        div { class: "flex flex-col gap-6",
            div { class: "flex items-center justify-between",
                div {
                    h2 { class: "text-2xl font-bold text-slate-100", "Evolution Kanban Board" }
                    p { class: "text-sm text-slate-400 mt-1", "Track issues that the daemon will auto-heal and resolve." }
                }
                button {
                    class: "px-4 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-semibold transition-all duration-200 shadow-lg shadow-indigo-900/40 active:scale-95 flex items-center gap-2",
                    onclick: move |_| show_create_modal.set(true),
                    span { class: "text-lg", "+" }
                    "New Issue"
                }
            }

            // Kanban columns
            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 items-start",
                KanbanColumn {
                    title: "Open",
                    header_bg: "bg-amber-500/10 border-amber-500/20 text-amber-400",
                    badge_bg: "bg-amber-500/10 text-amber-400",
                    issues: open_issues
                }
                KanbanColumn {
                    title: "In Progress",
                    header_bg: "bg-sky-500/10 border-sky-500/20 text-sky-400",
                    badge_bg: "bg-sky-500/10 text-sky-400 animate-pulse",
                    issues: in_progress_issues
                }
                KanbanColumn {
                    title: "Resolved",
                    header_bg: "bg-emerald-500/10 border-emerald-500/20 text-emerald-400",
                    badge_bg: "bg-emerald-500/10 text-emerald-400",
                    issues: resolved_issues
                }
                KanbanColumn {
                    title: "Failed",
                    header_bg: "bg-rose-500/10 border-rose-500/20 text-rose-400",
                    badge_bg: "bg-rose-500/10 text-rose-400",
                    issues: failed_issues
                }
            }

            // Create Issue Modal
            if *show_create_modal.read() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm",
                    div { class: "w-full max-w-md bg-slate-900 border border-slate-800/80 rounded-2xl shadow-2xl p-6 flex flex-col gap-4 animate-in fade-in-50 zoom-in-95",
                        div { class: "flex items-center justify-between border-b border-slate-800 pb-3",
                            h3 { class: "text-lg font-bold text-slate-100", "Submit Evolution Issue" }
                            button {
                                class: "text-slate-400 hover:text-slate-200 transition-colors",
                                onclick: move |_| {
                                    show_create_modal.set(false);
                                    error_msg.set(String::new());
                                },
                                "✕"
                            }
                        }

                        div { class: "flex flex-col gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs font-semibold text-slate-400", "Issue Title" }
                                input {
                                    class: "w-full bg-slate-950/85 border border-slate-800 focus:border-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 outline-none transition-all",
                                    placeholder: "e.g., Fix compilation warnings",
                                    value: "{new_title}",
                                    oninput: move |e| new_title.set(e.value())
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs font-semibold text-slate-400", "Issue Description" }
                                textarea {
                                    class: "w-full bg-slate-950/85 border border-slate-800 focus:border-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 outline-none transition-all h-28 resize-none",
                                    placeholder: "Write out what compile flags, bugs, or missing features to resolve...",
                                    value: "{new_body}",
                                    oninput: move |e| new_body.set(e.value())
                                }
                            }
                        }

                        if !error_msg.read().is_empty() {
                            p { class: "text-xs text-rose-400 font-semibold bg-rose-500/10 border border-rose-500/20 px-3 py-2 rounded-lg",
                                "{error_msg}"
                            }
                        }

                        div { class: "flex justify-end gap-3 mt-4",
                            button {
                                class: "px-4 py-2 text-sm font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all",
                                onclick: move |_| {
                                    show_create_modal.set(false);
                                    error_msg.set(String::new());
                                },
                                "Cancel"
                            }
                            button {
                                class: "px-4 py-2 text-sm font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-900/30 transition-all",
                                onclick: move |_| async move {
                                    if new_title.read().is_empty() {
                                        error_msg.set("Title is required.".to_string());
                                        return;
                                    }
                                    let title_val = new_title.read().clone();
                                    let body_val = new_body.read().clone();
                                    if let Err(e) = crate::create_issue(title_val, body_val).await {
                                        error_msg.set(format!("Failed to save: {}", e));
                                    } else {
                                        show_create_modal.set(false);
                                        new_title.set(String::new());
                                        new_body.set(String::new());
                                        error_msg.set(String::new());
                                        if let Ok(i) = crate::get_issues().await {
                                            issues.set(i);
                                        }
                                    }
                                },
                                "Create Issue"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn KanbanColumn(
    title: &'static str,
    header_bg: &'static str,
    badge_bg: &'static str,
    issues: Vec<Issue>
) -> Element {
    rsx! {
        div { class: "bg-slate-900/30 border border-slate-850 rounded-2xl p-4 flex flex-col gap-4 min-h-[400px]",
            div { class: format!("px-4 py-2.5 rounded-xl border flex items-center justify-between font-bold {}", header_bg),
                span { "{title}" }
                span { class: format!("text-xs px-2 py-0.5 rounded-full {}", badge_bg),
                    "{issues.len()}"
                }
            }

            div { class: "flex flex-col gap-3 overflow-y-auto max-h-[600px] scrollbar-thin scrollbar-thumb-slate-800",
                if issues.is_empty() {
                    div { class: "text-center py-8 text-xs text-slate-500 font-medium border border-dashed border-slate-850 rounded-xl",
                        "No issues"
                    }
                } else {
                    for issue in issues.iter() {
                        div { class: "bg-slate-900/60 border border-slate-850 hover:border-slate-800/80 rounded-xl p-4 flex flex-col gap-3 shadow-sm hover:shadow-md transition-all duration-200",
                            div { class: "flex flex-col gap-1.5",
                                h4 { class: "font-semibold text-slate-200 text-sm leading-snug", "{issue.title}" }
                                p { class: "text-xs text-slate-400 line-clamp-2 leading-relaxed", "{issue.body}" }
                            }
                            div { class: "flex items-center justify-between border-t border-slate-850 pt-2.5 text-[10px] text-slate-500",
                                span { "ID: #{issue.id}" }
                                span { "{issue.created_at.get(..10).unwrap_or(\"\")}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn VaultTab(
    vault_notes: Signal<Vec<(String, String)>>,
    projects: Signal<HashMap<String, ProjectInfo>>
) -> Element {
    let mut selected_note_index = use_signal(|| None::<usize>);
    let mut edit_note_name = use_signal(String::new);
    let mut edit_note_content = use_signal(String::new);
    let mut create_mode = use_signal(|| false);

    let mut inject_project = use_signal(String::new);
    let mut inject_status = use_signal(String::new);

    let projects_map = projects.read();

    rsx! {
        div { class: "grid grid-cols-1 lg:grid-cols-3 gap-8 items-start h-[calc(100vh-180px)]",
            // Notes list sidebar
            div { class: "bg-slate-900/40 border border-slate-850 rounded-2xl p-4 flex flex-col gap-4 h-full",
                div { class: "flex items-center justify-between",
                    h3 { class: "font-bold text-slate-200", "Knowledge Vault" }
                    button {
                        class: "px-3 py-1 rounded-lg bg-indigo-600/20 hover:bg-indigo-600/30 text-indigo-400 text-xs font-semibold border border-indigo-500/20 transition-all",
                        onclick: move |_| {
                            create_mode.set(true);
                            selected_note_index.set(None);
                            edit_note_name.set(String::new());
                            edit_note_content.set(String::new());
                        },
                        "+ Add Note"
                    }
                }

                div { class: "flex-1 overflow-y-auto flex flex-col gap-2 scrollbar-thin scrollbar-thumb-slate-800",
                    for i in 0..vault_notes.read().len() {
                        if let Some((name, _)) = vault_notes.read().get(i).cloned() {
                            button {
                                class: "w-full text-left px-4 py-3 rounded-xl text-sm font-medium transition-all duration-200 border "
                                    .to_string() + if Some(i) == *selected_note_index.read() && !*create_mode.read() {
                                        "bg-indigo-600/10 text-indigo-300 border-indigo-500/20"
                                    } else {
                                        "hover:bg-slate-800/40 text-slate-400 hover:text-slate-300 border-transparent"
                                    },
                                onclick: move |_| {
                                    create_mode.set(false);
                                    selected_note_index.set(Some(i));
                                    edit_note_name.set(name.clone());
                                    if let Some(note) = vault_notes.read().get(i) {
                                        edit_note_content.set(note.1.clone());
                                    }
                                    inject_status.set(String::new());
                                },
                                "📝 {name}"
                            }
                        }
                    }
                }
            }

            // Note editor panel
            div { class: "lg:col-span-2 bg-slate-900/30 border border-slate-850 rounded-2xl p-6 h-full flex flex-col justify-between",
                if *create_mode.read() || selected_note_index.read().is_some() {
                    div { class: "flex-1 flex flex-col gap-4 overflow-hidden",
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-semibold text-slate-500 uppercase tracking-wider", "Note Filename" }
                            input {
                                class: "w-full bg-slate-950/80 border border-slate-800 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-slate-100 outline-none transition-all",
                                placeholder: "e.g., custom_rules.md",
                                readonly: !*create_mode.read(),
                                value: "{edit_note_name}",
                                oninput: move |e| edit_note_name.set(e.value())
                            }
                        }

                        div { class: "flex-1 flex flex-col gap-1.5 overflow-hidden",
                            label { class: "text-xs font-semibold text-slate-500 uppercase tracking-wider", "Note Content" }
                            textarea {
                                class: "flex-1 w-full bg-slate-950/80 border border-slate-800 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-slate-100 outline-none transition-all resize-none font-mono text-sm leading-relaxed scrollbar-thin scrollbar-thumb-slate-800",
                                placeholder: "Write note in Markdown formatting...",
                                value: "{edit_note_content}",
                                oninput: move |e| edit_note_content.set(e.value())
                            }
                        }

                        // Inject knowledge section
                        if selected_note_index.read().is_some() && !*create_mode.read() {
                            div { class: "bg-slate-950/40 border border-slate-850 rounded-xl p-4 flex flex-col gap-3",
                                h4 { class: "text-xs font-bold text-slate-400 uppercase tracking-wider", "Inject Context Memory" }
                                div { class: "flex items-center gap-4",
                                    select {
                                        class: "bg-slate-900 border border-slate-850 rounded-lg px-3 py-1.5 text-xs text-slate-200 outline-none focus:border-indigo-500 transition-all",
                                        value: "{inject_project}",
                                        onchange: move |e| inject_project.set(e.value()),
                                        option { value: "", "Select project target..." }
                                        for proj_key in projects_map.keys() {
                                            option { value: "{proj_key}", "{proj_key}" }
                                        }
                                    }
                                    button {
                                        class: "px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold shadow-lg shadow-indigo-900/30 transition-all active:scale-95",
                                        onclick: move |_| async move {
                                            if inject_project.read().is_empty() {
                                                inject_status.set("Please select a project first.".to_string());
                                                return;
                                            }
                                            let project_val = inject_project.read().clone();
                                            let note_name_val = edit_note_name.read().clone();
                                            match crate::inject_knowledge(project_val, note_name_val).await {
                                                Ok(_) => inject_status.set("Successfully injected knowledge into project context!".to_string()),
                                                Err(e) => inject_status.set(format!("Error: {}", e))
                                            }
                                        },
                                        "Inject to Project context.md"
                                    }
                                }
                                if !inject_status.read().is_empty() {
                                    p { class: "text-[11px] font-semibold text-indigo-400 mt-1", "{inject_status}" }
                                }
                            }
                        }
                    }

                    // Save / Reset controls
                    div { class: "flex justify-end gap-3 mt-6 pt-4 border-t border-slate-850",
                        button {
                            class: "px-4 py-2 text-xs font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all",
                            onclick: move |_| {
                                selected_note_index.set(None);
                                create_mode.set(false);
                                edit_note_name.set(String::new());
                                edit_note_content.set(String::new());
                                inject_status.set(String::new());
                            },
                            "Cancel"
                        }
                        button {
                            class: "px-4 py-2 text-xs font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-900/30 transition-all",
                            onclick: move |_| async move {
                                if edit_note_name.read().is_empty() { return; }
                                let mut name_with_ext = edit_note_name.read().clone();
                                if !name_with_ext.ends_with(".md") {
                                    name_with_ext.push_str(".md");
                                }
                                let name_val = name_with_ext.clone();
                                let content_val = edit_note_content.read().clone();
                                if let Ok(_) = crate::save_vault_note(name_val, content_val).await {
                                    selected_note_index.set(None);
                                    create_mode.set(false);
                                    edit_note_name.set(String::new());
                                    edit_note_content.set(String::new());
                                    inject_status.set(String::new());
                                    if let Ok(vn) = crate::get_vault_notes().await {
                                        vault_notes.set(vn);
                                    }
                                }
                            },
                            "Save Note"
                        }
                    }
                } else {
                    div { class: "flex-1 flex flex-col items-center justify-center text-center gap-3",
                        span { class: "text-4xl", "📚" }
                        h3 { class: "font-semibold text-slate-300 text-lg", "Select a Note" }
                        p { class: "text-slate-500 text-sm max-w-xs", "Choose a note from the left sidebar to view/edit, or build a new one." }
                    }
                }
            }
        }
    }
}

#[component]
fn LogsTab(logs: Signal<String>) -> Element {
    rsx! {
        div { class: "flex flex-col gap-4 h-[calc(100vh-180px)]",
            div {
                h2 { class: "text-2xl font-bold text-slate-100", "Live Notification Logs" }
                p { class: "text-sm text-slate-400 mt-1", "View real-time event updates and background agent activities." }
            }

            // Shell Terminal Viewer
            div { class: "flex-1 bg-slate-950 border border-slate-850 rounded-2xl p-6 font-mono text-sm leading-relaxed text-slate-300 overflow-y-auto flex flex-col gap-1.5 scrollbar-thin scrollbar-thumb-slate-800 scrollbar-track-transparent select-text",
                if logs.read().is_empty() {
                    div { class: "text-slate-500 italic py-8 text-center",
                        "Terminal is idle. Waiting for logs..."
                    }
                } else {
                    for line in logs.read().lines() {
                        div { class: "whitespace-pre-wrap py-0.5",
                            if line.contains("ERROR") {
                                span { class: "text-rose-400 font-semibold", "{line}" }
                            } else if line.contains("WARN") {
                                span { class: "text-amber-400 font-semibold", "{line}" }
                            } else {
                                span { class: "text-slate-300", "{line}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
