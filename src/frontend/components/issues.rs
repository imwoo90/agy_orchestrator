use dioxus::prelude::*;
use crate::frontend::app::Issue;

#[component]
pub fn IssuesTab(issues: Signal<Vec<Issue>>) -> Element {
    let mut show_create_modal = use_signal(|| false);
    let mut selected_issue = use_signal(|| None::<Issue>);
    let mut new_title = use_signal(String::new);
    let mut new_body = use_signal(String::new);
    let mut error_msg = use_signal(String::new);

    // Sync issues status from backend in real-time every 3 seconds to catch background harness completion
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            if let Ok(latest_issues) = crate::get_issues().await {
                issues.set(latest_issues);
            }
        }
    });

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
        div { class: "flex flex-col gap-6 h-full w-full overflow-hidden",
            div { class: "flex items-center justify-between shrink-0",
                div {
                    h2 { class: "text-2xl font-bold text-slate-100", "Evolution Kanban Board" }
                    p { class: "text-sm text-slate-400 mt-1", "Track issues that the daemon will auto-heal and resolve." }
                }
                button {
                    class: "px-4 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-semibold transition-all duration-200 shadow-lg shadow-indigo-900/40 active:scale-95 flex items-center gap-2 cursor-pointer",
                    onclick: move |_| show_create_modal.set(true),
                    span { class: "text-lg", "+" }
                    "New Issue"
                }
            }

            // Kanban columns
            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 items-stretch flex-1 min-h-0 overflow-hidden",
                KanbanColumn {
                    title: "Open",
                    header_bg: "bg-amber-500/10 border-amber-500/20 text-amber-400",
                    badge_bg: "bg-amber-500/10 text-amber-400",
                    issues: open_issues,
                    issues_sig: issues,
                    selected_issue: selected_issue
                }
                KanbanColumn {
                    title: "In Progress",
                    header_bg: "bg-sky-500/10 border-sky-500/20 text-sky-400",
                    badge_bg: "bg-sky-500/10 text-sky-400 animate-pulse",
                    issues: in_progress_issues,
                    issues_sig: issues,
                    selected_issue: selected_issue
                }
                KanbanColumn {
                    title: "Resolved",
                    header_bg: "bg-emerald-500/10 border-emerald-500/20 text-emerald-400",
                    badge_bg: "bg-emerald-500/10 text-emerald-400",
                    issues: resolved_issues,
                    issues_sig: issues,
                    selected_issue: selected_issue
                }
                KanbanColumn {
                    title: "Failed",
                    header_bg: "bg-rose-500/10 border-rose-500/20 text-rose-400",
                    badge_bg: "bg-rose-500/10 text-rose-400",
                    issues: failed_issues,
                    issues_sig: issues,
                    selected_issue: selected_issue
                }
            }

            // Create Issue Modal
            if *show_create_modal.read() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm",
                    div { class: "w-full max-w-md bg-slate-900 border border-slate-800/80 rounded-2xl shadow-2xl p-6 flex flex-col gap-4 animate-in fade-in-50 zoom-in-95",
                        div { class: "flex items-center justify-between border-b border-slate-800 pb-3",
                            h3 { class: "text-lg font-bold text-slate-100", "Submit Evolution Issue" }
                            button {
                                class: "text-slate-400 hover:text-slate-200 transition-colors cursor-pointer",
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
                                class: "px-4 py-2 text-sm font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all cursor-pointer",
                                onclick: move |_| {
                                    show_create_modal.set(false);
                                    error_msg.set(String::new());
                                },
                                "Cancel"
                            }
                            button {
                                class: "px-4 py-2 text-sm font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-900/30 transition-all cursor-pointer",
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
            }            // Issue Detail Modal
            if let Some(issue) = selected_issue.read().clone() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm",
                    div { 
                        class: "w-full max-w-lg bg-slate-900 border border-slate-800/80 rounded-2xl shadow-2xl p-6 flex flex-col gap-4 animate-in fade-in-50 zoom-in-95",
                        
                        // Header Block (Direct Match to Create Issue Modal)
                        div { class: "flex items-start justify-between border-b border-slate-800 pb-3",
                            div { class: "flex flex-col gap-1.5 flex-1 min-w-0",
                                div { class: "flex items-center gap-2 flex-wrap",
                                    span { class: "text-[10px] font-mono font-bold bg-slate-950 text-slate-400 border border-slate-800/60 px-1.5 py-0.5 rounded-md", "ID: #{issue.id}" }
                                    // Status Badge
                                    match issue.status.as_str() {
                                        "open" => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20", "OPEN" } },
                                        "in-progress" => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded bg-sky-500/10 text-sky-400 border border-sky-500/20 animate-pulse", "IN PROGRESS" } },
                                        "resolved" => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20", "RESOLVED" } },
                                        "failed" => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded bg-rose-500/10 text-rose-400 border border-rose-500/20", "FAILED" } },
                                        _ => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded bg-slate-800 text-slate-400", "UNKNOWN" } }
                                    }
                                }
                                h3 { class: "text-base font-bold text-slate-100 mt-1 leading-snug break-words tracking-tight", "{issue.title}" }
                            }
                            button {
                                class: "text-slate-400 hover:text-slate-200 transition-colors cursor-pointer ml-3 text-lg p-0.5",
                                onclick: move |_| {
                                    selected_issue.set(None);
                                },
                                "✕"
                            }
                        }

                        // Failed Warning Guideline Box (If applicable)
                        if issue.status == "failed" {
                            div { class: "bg-rose-500/5 border border-rose-500/10 rounded-xl p-3 flex gap-2.5 items-start",
                                span { class: "text-sm shrink-0", "🚨" }
                                div { class: "flex flex-col gap-1",
                                    h4 { class: "text-[11px] font-bold text-rose-400 tracking-wide uppercase", "Evolution Integrity Harness Failed" }
                                    p { class: "text-[11px] text-slate-400 leading-relaxed",
                                        "Build failures occurred. Local changes were rolled back to keep your workspace stable. Fix build errors/logs locally, then click "
                                        strong { class: "text-indigo-400 font-semibold", "Run Harness" }
                                        " to re-verify."
                                    }
                                }
                            }
                        }

                        // Body Content Section
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-[10px] font-semibold text-slate-400", "Description" }
                            div { class: "w-full bg-slate-950/85 border border-slate-800 rounded-xl px-3.5 py-3 text-xs text-slate-300 leading-relaxed font-sans max-h-56 overflow-y-auto scrollbar-thin scrollbar-thumb-slate-800",
                                "{issue.body}"
                            }
                        }

                        // Dates Section
                        div { class: "flex flex-col gap-2 text-[10px] border-t border-slate-800 pt-3 text-slate-400",
                            div { class: "flex justify-between items-center px-1",
                                span { "Created At" }
                                span { class: "font-mono font-medium text-slate-350", "{issue.created_at}" }
                            }
                            if let Some(ref res_at) = issue.resolved_at {
                                div { class: "flex justify-between items-center px-1",
                                    span { "Resolved At" }
                                    span { class: "font-mono font-medium text-emerald-400", "{res_at}" }
                                }
                            }
                        }

                        // Actions Footer (Aligned right, matching Create Issue Modal button style)
                        div { class: "flex justify-end gap-3 mt-2",
                            button {
                                class: "px-4 py-2 text-xs font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all cursor-pointer",
                                onclick: move |_| {
                                    selected_issue.set(None);
                                },
                                "Close"
                            }

                            if issue.status == "open" || issue.status == "failed" {
                                button {
                                    class: "px-4 py-2 text-xs font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-900/30 transition-all cursor-pointer",
                                    onclick: {
                                        let id = issue.id;
                                        move |_| {
                                            selected_issue.set(None);
                                            spawn(async move {
                                                if crate::run_evolution_harness_fn(id).await.is_ok() {
                                                    if let Ok(i) = crate::get_issues().await {
                                                        issues.set(i);
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    "Run Harness ⚙️"
                                }
                                button {
                                    class: "px-4 py-2 text-xs font-semibold rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white shadow-lg shadow-emerald-900/30 transition-all cursor-pointer",
                                    onclick: {
                                        let id = issue.id;
                                        move |_| {
                                            selected_issue.set(None);
                                            spawn(async move {
                                                if crate::resolve_issue_fn(id).await.is_ok() {
                                                    if let Ok(i) = crate::get_issues().await {
                                                        issues.set(i);
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    "Resolve ✅"
                                }
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
    issues: Vec<Issue>,
    issues_sig: Signal<Vec<Issue>>,
    selected_issue: Signal<Option<Issue>>
) -> Element {
    rsx! {
        div { class: "bg-slate-900/30 border border-slate-850 rounded-2xl p-4 flex flex-col gap-4 h-full min-h-0",
            div { class: format!("px-4 py-2.5 rounded-xl border flex items-center justify-between font-bold shrink-0 {}", header_bg),
                span { "{title}" }
                span { class: format!("text-xs px-2 py-0.5 rounded-full {}", badge_bg),
                    "{issues.len()}"
                }
            }

            div { class: "flex-1 flex flex-col gap-3 overflow-y-auto scrollbar-thin scrollbar-thumb-slate-800 pr-1",
                if issues.is_empty() {
                    div { class: "text-center py-8 text-xs text-slate-500 font-medium border border-dashed border-slate-850 rounded-xl",
                        "No issues"
                    }
                } else {
                    for issue in issues.iter() {
                        div { 
                            class: "bg-slate-900/60 border border-slate-850 hover:border-slate-750 rounded-xl p-4 flex flex-col gap-3 shadow-sm hover:shadow-md transition-all duration-200 cursor-pointer active:scale-[0.99]",
                            onclick: {
                                let issue_clone = issue.clone();
                                move |_| {
                                    selected_issue.set(Some(issue_clone.clone()));
                                }
                            },
                            div { class: "flex flex-col gap-1.5",
                                h4 { class: "font-semibold text-slate-200 text-sm leading-snug", "{issue.title}" }
                                p { class: "text-xs text-slate-400 line-clamp-2 leading-relaxed", "{issue.body}" }
                            }
                            if issue.status == "open" || issue.status == "failed" {
                                div { class: "flex items-center gap-2 mt-1 pb-1 border-b border-slate-850/40",
                                    button {
                                        class: "flex-1 py-1 rounded bg-indigo-600/20 hover:bg-indigo-600/40 text-indigo-300 border border-indigo-500/20 text-[10px] font-bold active:scale-95 transition-all cursor-pointer text-center",
                                        onclick: {
                                            let id = issue.id;
                                            move |evt| {
                                                evt.stop_propagation(); // Prevent modal overlay when clicking buttons
                                                spawn(async move {
                                                    if crate::run_evolution_harness_fn(id).await.is_ok() {
                                                        if let Ok(i) = crate::get_issues().await {
                                                            issues_sig.set(i);
                                                        }
                                                    }
                                                });
                                            }
                                        },
                                        "Run Harness ⚙️"
                                    }
                                    button {
                                        class: "flex-1 py-1 rounded bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-400 border border-emerald-500/20 text-[10px] font-bold active:scale-95 transition-all cursor-pointer text-center",
                                        onclick: {
                                            let id = issue.id;
                                            move |evt| {
                                                evt.stop_propagation(); // Prevent modal overlay when clicking buttons
                                                spawn(async move {
                                                    if crate::resolve_issue_fn(id).await.is_ok() {
                                                        if let Ok(i) = crate::get_issues().await {
                                                            issues_sig.set(i);
                                                        }
                                                    }
                                                });
                                            }
                                        },
                                        "Resolve ✅"
                                    }
                                }
                            }
                            div { class: "flex items-center justify-between text-[10px] text-slate-500",
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
