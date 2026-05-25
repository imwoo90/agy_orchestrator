use dioxus::prelude::*;
use crate::frontend::app::Issue;

#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub is_user: bool,
    pub text: String,
    pub timestamp: String,
}

#[component]
pub fn ChatTab(issues: Signal<Vec<Issue>>) -> Element {
    let messages = use_signal(|| vec![
        ChatMessage {
            is_user: false,
            text: "Hello! I am your AI Orchestrator Assistant. 🤖\n\nI can help you manage your coding evolution and automate task creation.\n\nType conversational requests, or use a command prefix to create tasks:\n- **create task: [Title]**\n- **add task: [Title]**\n\nHow can I help you today?".to_string(),
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
        }
    ]);
    let mut input_text = use_signal(String::new);
    let is_loading = use_signal(|| false);

    let send_message = move || {
        let text = input_text.read().trim().to_string();
        if text.is_empty() || *is_loading.read() {
            return;
        }

        let mut msg_list = messages;
        let mut loading = is_loading;
        let mut input = input_text;
        let mut issues_sig = issues;

        spawn(async move {
            msg_list.write().push(ChatMessage {
                is_user: true,
                text: text.clone(),
                timestamp: chrono::Local::now().format("%H:%M").to_string(),
            });
            input.set(String::new());
            loading.set(true);

            match crate::send_chat_message(text).await {
                Ok(reply) => {
                    msg_list.write().push(ChatMessage {
                        is_user: false,
                        text: reply,
                        timestamp: chrono::Local::now().format("%H:%M").to_string(),
                    });
                    
                    if let Ok(latest_issues) = crate::get_issues().await {
                        issues_sig.set(latest_issues);
                    }
                }
                Err(e) => {
                    msg_list.write().push(ChatMessage {
                        is_user: false,
                        text: format!("⚠️ Error: {}", e),
                        timestamp: chrono::Local::now().format("%H:%M").to_string(),
                    });
                }
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "flex flex-col h-[calc(100vh-12rem)] max-w-4xl mx-auto bg-slate-900/40 border border-slate-850 rounded-2xl overflow-hidden shadow-2xl backdrop-blur-sm",
            div { class: "bg-slate-900/80 px-6 py-4 border-b border-slate-800 flex items-center justify-between",
                div { class: "flex items-center gap-3",
                    span { class: "text-lg", "💬" }
                    div {
                        h2 { class: "text-sm font-bold text-slate-100", "AI Assistant" }
                        p { class: "text-[10px] text-indigo-400 font-medium", "Chat to automate workflows & task creation" }
                    }
                }
                div { class: "flex items-center gap-2",
                    span { class: "h-2 w-2 rounded-full bg-emerald-500 animate-pulse" }
                    span { class: "text-[10px] text-slate-400 font-mono", "Online" }
                }
            }

            div { class: "flex-1 overflow-y-auto p-6 flex flex-col gap-4 bg-slate-950/20",
                for msg in messages.read().iter() {
                    div {
                        class: format!("flex flex-col max-w-[80%] {}", if msg.is_user { "self-end items-end" } else { "self-start items-start" }),
                        div {
                            class: format!("px-4 py-3 rounded-2xl text-sm leading-relaxed shadow-md {}",
                                if msg.is_user {
                                    "bg-indigo-600 text-white rounded-br-none"
                                } else {
                                    "bg-slate-800/80 text-slate-100 border border-slate-750 rounded-bl-none"
                                }
                            ),
                            style: "white-space: pre-wrap;",
                            {
                                let parts = msg.text.split("**");
                                parts.enumerate().map(|(idx, part)| {
                                    if idx % 2 == 1 {
                                        rsx! { strong { class: "font-bold text-indigo-200", "{part}" } }
                                    } else {
                                        rsx! { span { "{part}" } }
                                    }
                                })
                            }
                        }
                        span { class: "text-[10px] text-slate-500 mt-1 px-1 font-mono", "{msg.timestamp}" }
                    }
                }

                if *is_loading.read() {
                    div { class: "self-start flex items-center gap-2 bg-slate-800/50 border border-slate-750 px-4 py-3 rounded-2xl rounded-bl-none max-w-[100px]",
                        div { class: "h-2 w-2 bg-indigo-400 rounded-full animate-bounce" }
                        div { class: "h-2 w-2 bg-indigo-400 rounded-full animate-bounce [animation-delay:0.2s]" }
                        div { class: "h-2 w-2 bg-indigo-400 rounded-full animate-bounce [animation-delay:0.4s]" }
                    }
                }
            }

            div { class: "bg-slate-900/60 border-t border-slate-800 p-4 flex gap-3 items-center",
                input {
                    class: "flex-1 bg-slate-950 border border-slate-800 rounded-xl px-4 py-3 text-sm text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-indigo-500/85 transition-all shadow-inner",
                    placeholder: "Type a message or use 'create task: [Title]'...",
                    value: "{input_text}",
                    oninput: move |evt| input_text.set(evt.value()),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Enter {
                            send_message();
                        }
                    }
                }
                button {
                    class: format!("px-5 py-3 rounded-xl font-semibold text-sm transition-all duration-200 active:scale-95 flex items-center gap-2 cursor-pointer shadow-lg {}",
                        if *is_loading.read() {
                            "bg-slate-800 text-slate-500 border border-slate-750"
                        } else {
                            "bg-indigo-600 hover:bg-indigo-500 text-white shadow-indigo-900/20"
                        }
                    ),
                    onclick: move |_| send_message(),
                    disabled: *is_loading.read(),
                    span { "Send" }
                    span { "➔" }
                }
            }
        }
    }
}
