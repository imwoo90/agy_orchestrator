use dioxus::prelude::*;
use std::collections::HashMap;
use crate::frontend::app::{ProjectInfo, HealthCheckResult};

#[component]
pub fn ProjectsTab(
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
        div { class: "flex flex-col gap-6 h-full w-full overflow-hidden",
            div { class: "flex items-center justify-between shrink-0",
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
                div { class: "bg-slate-900/30 border border-slate-850 rounded-2xl p-12 text-center flex flex-col items-center justify-center gap-3 flex-1",
                    span { class: "text-4xl", "📭" }
                    h3 { class: "font-semibold text-slate-300 text-lg", "No projects registered" }
                    p { class: "text-slate-500 text-sm max-w-sm", "Spawn your first project target to start executing agent tasks." }
                }
            } else {
                div { class: "flex-1 overflow-y-auto pr-2 scrollbar-thin scrollbar-thumb-slate-800",
                    div { class: "grid grid-cols-1 xl:grid-cols-2 gap-6 pb-6",
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
