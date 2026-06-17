use dioxus::prelude::*;
use crate::frontend::app::Issue;

#[component]
pub fn IssuesTab(issues: Signal<Vec<Issue>>) -> Element {
    let mut show_create_modal = use_signal(|| false);
    let mut selected_issue = use_signal(|| None::<Issue>);
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
                div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/85 backdrop-blur-md animate-in fade-in duration-200",
                    div { 
                        class: format!(
                            "w-full max-w-3xl bg-gradient-to-b from-slate-900 to-slate-950 border-t-4 border-x border-b border-slate-800/80 rounded-2xl shadow-2xl p-6 md:p-7 flex flex-col gap-6 relative overflow-hidden animate-in zoom-in-95 duration-200 {}",
                            match issue.status.as_str() {
                                "open" => "border-t-amber-500",
                                "in-progress" => "border-t-sky-500/80 animate-pulse",
                                "resolved" => "border-t-emerald-500",
                                "failed" => "border-t-rose-500",
                                _ => "border-t-slate-750"
                            }
                        ),
                        
                        // Neon glow decoration inside the modal background
                        div { class: "absolute -top-24 -left-24 w-56 h-56 bg-indigo-500/10 rounded-full blur-3xl pointer-events-none" }
                        div { class: "absolute -bottom-24 -right-24 w-56 h-56 bg-emerald-500/5 rounded-full blur-3xl pointer-events-none" }

                        // Header Block
                        div { 
                            class: format!(
                                "flex items-start justify-between border-b border-slate-800/60 pb-5 px-1 relative z-10 rounded-t-xl {}",
                                match issue.status.as_str() {
                                    "open" => "bg-amber-500/5",
                                    "in-progress" => "bg-sky-500/5",
                                    "resolved" => "bg-emerald-500/5",
                                    "failed" => "bg-rose-500/5",
                                    _ => "bg-slate-800/20"
                                }
                            ),
                            div { class: "flex flex-col gap-2.5 flex-1 min-w-0",
                                div { class: "flex items-center gap-2.5 flex-wrap",
                                    span { class: "text-[10px] font-mono font-bold bg-slate-850 text-slate-400 border border-slate-800 px-2 py-0.5 rounded-md", "ID: #{issue.id}" }
                                    // Status Badge
                                    match issue.status.as_str() {
                                        "open" => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded-md bg-amber-500/10 text-amber-450 border border-amber-500/25", "OPEN" } },
                                        "in-progress" => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded-md bg-sky-500/10 text-sky-400 border border-sky-500/25 animate-pulse", "IN PROGRESS" } },
                                        "resolved" => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/25", "RESOLVED" } },
                                        "failed" => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded-md bg-rose-500/10 text-rose-455 border border-rose-500/25", "FAILED" } },
                                        _ => rsx! { span { class: "text-[9px] font-extrabold tracking-wider px-2 py-0.5 rounded-md bg-slate-800 text-slate-400", "UNKNOWN" } }
                                    }
                                }
                                h3 { class: "text-xl font-bold text-slate-100 mt-1 leading-snug break-words tracking-tight", "{issue.title}" }
                            }
                            button {
                                class: "text-slate-400 hover:text-rose-400 hover:rotate-90 transition-all duration-300 text-xl p-1.5 shrink-0 cursor-pointer ml-3 bg-slate-850 hover:bg-rose-950/20 border border-slate-800 hover:border-rose-900/30 rounded-lg",
                                onclick: move |_| {
                                    selected_issue.set(None);
                                },
                                "✕"
                            }
                        }

                        // Failed Warning Guideline Box
                        if issue.status == "failed" {
                            div { class: "bg-rose-500/5 border border-rose-500/20 rounded-xl p-4 flex gap-3 relative z-10 items-start shadow-sm",
                                span { class: "text-lg shrink-0", "🚨" }
                                div { class: "flex flex-col gap-1.5",
                                    h4 { class: "text-xs font-bold text-rose-400 tracking-wide uppercase", "Evolution Integrity Harness Rejected This Code" }
                                    p { class: "text-[12px] text-slate-400 leading-relaxed",
                                        "The automated Self-Evolution agent encountered build failures (Clippy warnings/errors, failing unit tests, or Static Integrity Gate structures). To maintain project sanity, all uncommitted changes were safely rolled back. You can edit the code locally to fix the warnings/bugs, then click "
                                        strong { class: "text-indigo-400 font-semibold", "Run Harness" }
                                        " to re-verify and push changes automatically."
                                    }
                                }
                            }
                        }

                        // Grid Content Details
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-5 relative z-10",
                            // Left Details Box: Description
                            div { class: "flex flex-col gap-2.5",
                                label { class: "text-[10px] font-bold text-slate-500 uppercase tracking-wider", "Issue Description" }
                                div { class: "bg-slate-950/70 border border-slate-850 rounded-xl p-4.5 min-h-[180px] max-h-[280px] overflow-y-auto whitespace-pre-wrap text-[13px] text-slate-300 leading-relaxed font-sans scrollbar-thin scrollbar-thumb-slate-800",
                                    "{issue.body}"
                                }
                            }
                            
                            // Right Details Box: Metadata & Status Details
                            div { class: "flex flex-col gap-4 justify-between",
                                div { class: "flex flex-col gap-3",
                                    label { class: "text-[10px] font-bold text-slate-500 uppercase tracking-wider", "System Metadata" }
                                    
                                    div { class: "flex flex-col gap-2 text-xs",
                                        div { class: "flex justify-between items-center bg-slate-950/40 border border-slate-850/50 px-4 py-2.5 rounded-xl",
                                            span { class: "text-slate-400 font-medium", "📅 Created At" }
                                            span { class: "font-mono text-slate-200 font-semibold", "{issue.created_at}" }
                                        }
                                        if let Some(ref res_at) = issue.resolved_at {
                                            div { class: "flex justify-between items-center bg-emerald-500/5 border border-emerald-500/10 px-4 py-2.5 rounded-xl",
                                                span { class: "text-emerald-450 font-medium", "✅ Resolved At" }
                                                span { class: "font-mono text-emerald-400 font-semibold", "{res_at}" }
                                            }
                                        } else {
                                            div { class: "flex justify-between items-center bg-slate-950/20 border border-slate-850/30 px-4 py-2.5 rounded-xl text-slate-500",
                                                span { "⏳ Resolution Status" }
                                                span { class: "italic", "Pending verification" }
                                            }
                                        }
                                    }
                                }

                                // Interactive Quick Guide card inside metadata column
                                div { class: "bg-slate-950/35 border border-slate-850/60 rounded-xl p-4 flex flex-col gap-1.5 text-xs text-slate-450 leading-relaxed",
                                    h5 { class: "font-bold text-slate-350 text-[11px]", "💡 Harness System Mechanism" }
                                    p { "When a task status transitions to active, the agent evolver will attempt static integrity gates (warnings-free Rust lints, successful cargo compile/tests) to commit code directly without manual git steps." }
                                }
                            }
                        }

                        // Actions Footer
                        div { class: "flex flex-wrap items-center justify-between border-t border-slate-800/60 pt-5 gap-3 relative z-10",
                            button {
                                class: "px-5 py-2.5 text-xs font-semibold rounded-xl bg-slate-850 hover:bg-slate-800 text-slate-300 border border-slate-800 hover:border-slate-750 transition-all cursor-pointer active:scale-95 shadow-sm",
                                onclick: move |_| {
                                    selected_issue.set(None);
                                },
                                "Close View"
                            }

                            // Run buttons inside modal if status permits actions
                            if issue.status == "open" || issue.status == "failed" {
                                div { class: "flex items-center gap-3.5",
                                    button {
                                        class: "px-4 py-2.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white border border-indigo-550/40 text-xs font-bold active:scale-95 transition-all cursor-pointer shadow-lg shadow-indigo-900/30",
                                        onclick: {
                                            let id = issue.id;
                                            move |_| {
                                                selected_issue.set(None); // Close view during build to let them see Kanban
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
                                        class: "px-4 py-2.5 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white border border-emerald-550/40 text-xs font-bold active:scale-95 transition-all cursor-pointer shadow-lg shadow-emerald-900/30",
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
