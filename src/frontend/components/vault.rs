use dioxus::prelude::*;
use std::collections::HashMap;
use crate::frontend::app::ProjectInfo;

#[component]
pub fn VaultTab(
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
        div { class: "grid grid-cols-1 lg:grid-cols-3 gap-8 items-stretch h-full w-full overflow-hidden",
            // Notes list sidebar
            div { class: "bg-slate-900/40 border border-slate-850 rounded-2xl p-4 flex flex-col gap-4 h-full min-h-0",
                div { class: "flex items-center justify-between shrink-0",
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

                div { class: "flex-1 overflow-y-auto flex flex-col gap-2 scrollbar-thin scrollbar-thumb-slate-800 pr-1",
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
            div { class: "lg:col-span-2 bg-slate-900/30 border border-slate-850 rounded-2xl p-6 h-full flex flex-col justify-between min-h-0 overflow-hidden",
                if *create_mode.read() || selected_note_index.read().is_some() {
                    div { class: "flex-1 flex flex-col gap-4 overflow-hidden min-h-0",
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

                        div { class: "flex-1 flex flex-col gap-1.5 overflow-hidden min-h-0",
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
                            div { class: "bg-slate-950/40 border border-slate-850 rounded-xl p-4 flex flex-col gap-3 shrink-0",
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
                    div { class: "flex justify-end gap-3 mt-6 pt-4 border-t border-slate-850 shrink-0",
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
                                if crate::save_vault_note(name_val, content_val).await.is_ok() {
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
