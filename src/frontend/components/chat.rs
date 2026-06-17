use dioxus::prelude::*;
use dioxus::document::eval;
use std::collections::HashMap;
use crate::frontend::app::{Issue, ChatMessage, ChatSession};


#[derive(Debug, Clone, PartialEq)]
enum InlineSpan {
    Text(String),
    Bold(String),
    Code(String),
    FileLink { label: String, path: String },
}

#[derive(Debug, Clone, PartialEq)]
enum MarkdownBlock {
    Header(usize, String),
    CodeBlock(String, String),
    List(Vec<String>),
    Paragraph(String),
}

#[allow(clippy::while_let_on_iterator)]
fn parse_inline_line(text: &str) -> Vec<InlineSpan> {
    let mut spans = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current_text = String::new();

    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            chars.next(); // consume the second '*'
            if !current_text.is_empty() {
                spans.push(InlineSpan::Text(current_text));
                current_text = String::new();
            }
            let mut bold_content = String::new();
            let mut found_end = false;
            while let Some(bc) = chars.next() {
                if bc == '*' && chars.peek() == Some(&'*') {
                    chars.next(); // consume the second '*'
                    found_end = true;
                    break;
                }
                bold_content.push(bc);
            }
            if found_end {
                spans.push(InlineSpan::Bold(bold_content));
            } else {
                current_text.push_str("**");
                current_text.push_str(&bold_content);
            }
        } else if c == '`' {
            if !current_text.is_empty() {
                spans.push(InlineSpan::Text(current_text));
                current_text = String::new();
            }
            let mut code_content = String::new();
            let mut found_end = false;
            while let Some(cc) = chars.next() {
                if cc == '`' {
                    found_end = true;
                    break;
                }
                code_content.push(cc);
            }
            if found_end {
                spans.push(InlineSpan::Code(code_content));
            } else {
                current_text.push('`');
                current_text.push_str(&code_content);
            }
        } else if c == '[' {
            if !current_text.is_empty() {
                spans.push(InlineSpan::Text(current_text));
                current_text = String::new();
            }
            let mut label = String::new();
            let mut found_label_end = false;
            while let Some(lc) = chars.next() {
                if lc == ']' {
                    found_label_end = true;
                    break;
                }
                label.push(lc);
            }
            if found_label_end && chars.peek() == Some(&'(') {
                chars.next(); // consume '('
                let mut path = String::new();
                let mut found_path_end = false;
                while let Some(pc) = chars.next() {
                    if pc == ')' {
                        found_path_end = true;
                        break;
                    }
                    path.push(pc);
                }
                if found_path_end {
                    spans.push(InlineSpan::FileLink { label, path });
                } else {
                    current_text.push('[');
                    current_text.push_str(&label);
                    current_text.push_str("](");
                    current_text.push_str(&path);
                }
            } else {
                current_text.push('[');
                current_text.push_str(&label);
            }
        } else {
            current_text.push(c);
        }
    }
    if !current_text.is_empty() {
        spans.push(InlineSpan::Text(current_text));
    }
    spans
}

fn parse_markdown_blocks(text: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut lines = text.lines().peekable();
    
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(stripped) = trimmed.strip_prefix("```") {
            let lang = stripped.trim().to_string();
            let mut code_content = String::new();
            for next_line in lines.by_ref() {
                if next_line.trim().starts_with("```") {
                    break;
                }
                code_content.push_str(next_line);
                code_content.push('\n');
            }
            if code_content.ends_with('\n') {
                code_content.pop();
            }
            blocks.push(MarkdownBlock::CodeBlock(lang, code_content));
        } else if trimmed.starts_with('#') {
            let mut level = 0;
            let mut chars = trimmed.chars();
            while chars.next() == Some('#') {
                level += 1;
            }
            let content = trimmed[level..].trim().to_string();
            blocks.push(MarkdownBlock::Header(level, content));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let mut items = vec![trimmed[2..].trim().to_string()];
            while let Some(next_line) = lines.peek() {
                let next_trimmed = next_line.trim();
                if next_trimmed.starts_with("- ") || next_trimmed.starts_with("* ") {
                    items.push(next_trimmed[2..].trim().to_string());
                    lines.next();
                } else {
                    break;
                }
            }
            blocks.push(MarkdownBlock::List(items));
        } else {
            blocks.push(MarkdownBlock::Paragraph(trimmed.to_string()));
        }
    }
    blocks
}

fn render_spans(spans: Vec<InlineSpan>) -> Element {
    rsx! {
        span {
            for span in spans {
                match span {
                    InlineSpan::Text(txt) => rsx! { span { "{txt}" } },
                    InlineSpan::Bold(txt) => rsx! { strong { class: "font-semibold text-slate-100", "{txt}" } },
                    InlineSpan::Code(txt) => rsx! {
                        code { class: "bg-slate-900/90 text-indigo-300 px-1.5 py-0.5 rounded font-mono text-[13px] border border-slate-800/80 mx-0.5", "{txt}" }
                    },
                    InlineSpan::FileLink { label, path } => {
                        let is_file_scheme = path.starts_with("file://");
                        if is_file_scheme {
                            rsx! {
                                a {
                                    class: "inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-slate-900/60 hover:bg-slate-900 border border-slate-750 hover:border-slate-600 text-indigo-400 hover:text-indigo-300 font-mono text-[12px] transition-all duration-150 cursor-pointer my-0.5",
                                    href: "{path}",
                                    target: "_blank",
                                    span { class: "text-[10px]", "📄" }
                                    span { "{label}" }
                                }
                            }
                        } else {
                            rsx! {
                                a {
                                    class: "text-indigo-400 hover:text-indigo-300 underline transition-colors cursor-pointer",
                                    href: "{path}",
                                    target: "_blank",
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_markdown(text: &str) -> Element {
    let blocks = parse_markdown_blocks(text);
    rsx! {
        div { class: "flex flex-col gap-3.5",
            for block in blocks {
                match block {
                    MarkdownBlock::Header(level, content) => {
                        let header_class = match level {
                            1 => "text-lg font-bold text-slate-100 border-b border-slate-800 pb-1 mt-2",
                            2 => "text-md font-bold text-slate-200 mt-1.5",
                            _ => "text-sm font-semibold text-slate-300 mt-1",
                        };
                        rsx! {
                            div { class: "{header_class}",
                                {render_spans(parse_inline_line(&content))}
                            }
                        }
                    }
                    MarkdownBlock::CodeBlock(lang, content) => {
                        let mut copy_txt = use_signal(|| "Copy".to_string());
                        let content_clone = content.clone();
                        rsx! {
                            div { class: "relative group my-2 bg-slate-950/80 border border-slate-850 rounded-xl overflow-hidden shadow-inner w-full",
                                div { class: "flex items-center justify-between px-4 py-1.5 bg-slate-900/60 border-b border-slate-850/80 text-[11px] font-mono text-slate-400",
                                    span { "{lang}" }
                                    button {
                                        class: "px-2 py-0.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white transition-all cursor-pointer",
                                        onclick: move |_| {
                                            let code = content_clone.clone();
                                            spawn(async move {
                                                let _ = eval(&format!("navigator.clipboard.writeText({});", serde_json::to_string(&code).unwrap()));
                                                copy_txt.set("Copied!".to_string());
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    gloo_timers::future::TimeoutFuture::new(1500).await;
                                                }
                                                #[cfg(not(target_arch = "wasm32"))]
                                                {
                                                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                                                }
                                                copy_txt.set("Copy".to_string());
                                            });
                                        },
                                        "{copy_txt}"
                                    }
                                }
                                pre { class: "p-4 overflow-x-auto text-[13px] font-mono leading-relaxed text-slate-300",
                                    code { "{content}" }
                                }
                            }
                        }
                    }
                    MarkdownBlock::List(items) => {
                        rsx! {
                            ul { class: "list-disc pl-5 flex flex-col gap-1.5 text-slate-300 text-sm",
                                for item in items {
                                    li {
                                        {render_spans(parse_inline_line(&item))}
                                    }
                                }
                            }
                        }
                    }
                    MarkdownBlock::Paragraph(content) => {
                        rsx! {
                            p { class: "text-slate-300 text-sm leading-relaxed",
                                {render_spans(parse_inline_line(&content))}
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ChatTab(
    messages: Signal<HashMap<String, Vec<ChatMessage>>>,
    issues: Signal<Vec<Issue>>,
    active_session_id: Signal<Option<String>>,
    chat_sessions: Signal<Vec<ChatSession>>,
) -> Element {
    let mut input_text = use_signal(String::new);
    let is_loading = use_signal(HashMap::<String, bool>::new);

    use_effect(move || {
        // Track active_session_id, messages, and is_loading to trigger when they change
        let _ = active_session_id.read();
        let _ = messages.read();
        let _ = is_loading.read();
        
        // Scroll the message stream area to the bottom & focus input
        let _ = eval("
            setTimeout(() => {
                let el = document.getElementById('chat-messages-container');
                if (el) {
                    el.scrollTop = el.scrollHeight;
                }
                let input = document.getElementById('chat-input-field');
                if (input) {
                    input.focus();
                }
            }, 50);
        ");
    });

    let active_id_opt = active_session_id.read().clone();
    let active_loading = if let Some(ref id) = active_id_opt {
        is_loading.read().get(id).copied().unwrap_or(false)
    } else {
        false
    };
    let current_session_title = if let Some(ref id) = active_id_opt {
        chat_sessions.read().iter().find(|s| s.id == *id).map(|s| s.title.clone()).unwrap_or_else(|| "AI Personal Secretary".to_string())
    } else {
        "AI Personal Secretary".to_string()
    };

    let display_messages = if let Some(ref id) = active_id_opt {
        if let Some(msgs_list) = messages.read().get(id) {
            msgs_list.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let display_messages = if display_messages.is_empty() {
        vec![
            ChatMessage {
                is_user: false,
                text: "Hello! I am your AI Personal Secretary. 🤖\n\nI can help you manage your projects, check tasks, and automate workflows in a Just-in-Time manner.\n\nAsk me anything, e.g. *'What are the ongoing tasks?'* or *'Show my projects'*!".to_string(),
                timestamp: chrono::Local::now().format("%H:%M").to_string(),
            }
        ]
    } else {
        display_messages
    };

    let send_custom_message = move |text: String| {
        let active_id = match active_session_id.read().clone() {
            Some(id) => id,
            None => return,
        };

        let currently_loading = is_loading.read().get(&active_id).copied().unwrap_or(false);
        if text.is_empty() || currently_loading {
            return;
        }

        let mut msg_list = messages;
        let mut loading = is_loading;
        let mut issues_sig = issues;
        let mut chat_sessions_sig = chat_sessions;
        let active_id_spawn = active_id.clone();
        let mut active_session_id_ref = active_session_id;

        spawn(async move {
            msg_list.write().entry(active_id_spawn.clone()).or_default().push(ChatMessage {
                is_user: true,
                text: text.clone(),
                timestamp: chrono::Local::now().format("%H:%M").to_string(),
            });
            loading.write().insert(active_id_spawn.clone(), true);

            let mut final_id = active_id_spawn.clone();
            match crate::send_chat_message(active_id_spawn.clone(), text).await {
                Ok(response) => {
                    final_id = response.actual_session_id.clone();
                    
                    // Session promotion migration inside the HashMap cache
                    if final_id != active_id_spawn {
                        let mut map = msg_list;
                        let draft_history = map.write().remove(&active_id_spawn);
                        if let Some(draft_history) = draft_history {
                            map.write().insert(final_id.clone(), draft_history);
                        }
                    }

                    if Some(active_id_spawn.clone()) == *active_session_id_ref.read() {
                        active_session_id_ref.set(Some(response.actual_session_id.clone()));
                    }

                    msg_list.write().entry(final_id.clone()).or_default().push(ChatMessage {
                        is_user: false,
                        text: response.reply,
                        timestamp: chrono::Local::now().format("%H:%M").to_string(),
                    });
                    
                    if let Ok(sessions) = crate::get_chat_sessions().await {
                        chat_sessions_sig.set(sessions);
                    }
                    if let Ok(latest_issues) = crate::get_issues().await {
                        issues_sig.set(latest_issues);
                    }
                }
                Err(e) => {
                    msg_list.write().entry(active_id_spawn.clone()).or_default().push(ChatMessage {
                        is_user: false,
                        text: format!("⚠️ Error: {}", e),
                        timestamp: chrono::Local::now().format("%H:%M").to_string(),
                    });
                }
            }
            loading.write().insert(active_id_spawn.clone(), false);
            loading.write().insert(final_id, false);
        });
    };

    let mut send_message = move || {
        let text = input_text.read().trim().to_string();
        if !text.is_empty() {
            send_custom_message(text);
            input_text.set(String::new());
        }
    };

    rsx! {
        div { class: "flex h-full w-full bg-gradient-to-br from-slate-900/60 to-slate-950/80 border border-slate-800/80 rounded-2xl overflow-hidden shadow-2xl backdrop-blur-md",
            
            // Sidebar
            div { class: "w-64 border-r border-slate-800/70 flex flex-col bg-slate-950/45 shrink-0",
                
                // New Chat Button
                div { class: "p-4 border-b border-slate-800/70",
                    button {
                        class: "w-full py-2.5 px-4 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-semibold text-xs transition-all duration-200 flex items-center justify-center gap-2 shadow-lg shadow-indigo-900/30 hover:shadow-indigo-550/30 hover:scale-[1.02] cursor-pointer",
                        onclick: move |_| {
                            let mut active_sig = active_session_id;
                            let mut sessions_sig = chat_sessions;
                            let mut msgs = messages;
                            spawn(async move {
                                if let Ok(new_id) = crate::create_chat_session().await {
                                    active_sig.set(Some(new_id.clone()));
                                    if let Ok(s) = crate::get_chat_sessions().await {
                                        sessions_sig.set(s);
                                    }
                                    msgs.write().insert(new_id, Vec::new());
                                }
                            });
                        },
                        span { class: "text-sm", "💬" }
                        span { "New Chat" }
                    }
                }

                // Rooms List
                div { class: "flex-1 overflow-y-auto p-3 flex flex-col gap-2 scrollbar-thin scrollbar-thumb-slate-800 scrollbar-track-transparent",
                    if chat_sessions.read().is_empty() {
                        div { class: "text-center text-[11px] text-slate-500 py-8 font-medium",
                            "No active chats"
                        }
                    } else {
                        for session in chat_sessions.read().clone() {
                            {
                                let is_active = Some(session.id.clone()) == *active_session_id.read();
                                let session_id_clone = session.id.clone();
                                let session_id_delete = session.id.clone();
                                
                                let time_display = if session.updated_at.len() >= 16 {
                                    format!("{} {}", &session.updated_at[5..10], &session.updated_at[11..16])
                                } else {
                                    session.updated_at.clone()
                                };

                                rsx! {
                                    div {
                                        class: format!("group relative flex items-center justify-between px-3.5 py-3 rounded-xl border transition-all duration-200 cursor-pointer {}",
                                            if is_active {
                                                "bg-indigo-600/10 border-indigo-500/30 text-slate-100 shadow-sm"
                                            } else {
                                                "bg-slate-900/10 hover:bg-slate-900/40 border-transparent hover:border-slate-800/50 text-slate-400 hover:text-slate-200"
                                            }
                                        ),
                                        onclick: move |_| {
                                            let s_id = session_id_clone.clone();
                                            let mut active_sig = active_session_id;
                                            let mut msgs = messages;
                                            spawn(async move {
                                                let _ = crate::set_active_session_id(s_id.clone()).await;
                                                active_sig.set(Some(s_id.clone()));
                                                
                                                let has_cache = msgs.read().contains_key(&s_id);
                                                if !has_cache {
                                                    if let Ok(history) = crate::get_chat_history(s_id.clone()).await {
                                                        msgs.write().insert(s_id, history);
                                                    }
                                                }
                                            });
                                        },
                                        
                                        div { class: "flex-1 min-w-0 pr-2",
                                            h3 { class: "text-xs font-semibold truncate", "{session.title}" }
                                            p { class: "text-[9px] text-slate-500 font-mono mt-0.5", "{time_display}" }
                                        }

                                        button {
                                            class: "p-1.5 rounded-lg bg-red-950/20 hover:bg-red-900/40 border border-red-900/20 hover:border-red-700/40 text-red-400 transition-all cursor-pointer opacity-0 group-hover:opacity-100 focus:opacity-100 active:scale-90",
                                            title: "Delete chat room",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                let s_id = session_id_delete.clone();
                                                let mut active_sig = active_session_id;
                                                let mut sessions_sig = chat_sessions;
                                                let mut msgs = messages;
                                                spawn(async move {
                                                    let mut eval_confirm = eval("dioxus.send(confirm('이 대화방을 정말로 삭제하시겠습니까?'));");
                                                    let confirmed = eval_confirm.recv::<bool>().await.unwrap_or(false);
                                                    if !confirmed {
                                                        return;
                                                    }
                                                    if crate::delete_chat_session(s_id.clone()).await.is_ok() {
                                                        msgs.write().remove(&s_id);
                                                        if let Ok(s) = crate::get_chat_sessions().await {
                                                            sessions_sig.set(s);
                                                        }
                                                        match crate::get_active_session_id().await {
                                                            Ok(Some(active_id)) => {
                                                                active_sig.set(Some(active_id.clone()));
                                                                let has_cache = msgs.read().contains_key(&active_id);
                                                                if !has_cache {
                                                                    if let Ok(history) = crate::get_chat_history(active_id.clone()).await {
                                                                        msgs.write().insert(active_id, history);
                                                                    }
                                                                }
                                                            }
                                                            Ok(None) | Err(_) => {
                                                                active_sig.set(None);
                                                            }
                                                        }
                                                    }
                                                });
                                            },
                                            svg { class: "w-3.5 h-3.5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Right pane
            div { class: "flex-1 flex flex-col overflow-hidden relative bg-slate-950/20",
                if let Some(active_id) = active_session_id.read().clone() {
                    div { class: "flex-1 flex flex-col overflow-hidden",
                        // Header
                        div { class: "bg-slate-900/90 px-6 py-4 border-b border-slate-800/70 flex items-center justify-between shadow-sm shrink-0",
                            div { class: "flex items-center gap-3.5 min-w-0 flex-1",
                                div { class: "p-2 rounded-xl bg-indigo-500/10 border border-indigo-500/20 text-indigo-400 text-lg shadow-inner shrink-0",
                                    "✨"
                                }
                                div { class: "min-w-0",
                                    h2 { class: "text-sm font-bold text-slate-100 flex items-center gap-2 truncate",
                                        "{current_session_title}"
                                        span { class: "text-[10px] px-1.5 py-0.5 rounded bg-indigo-500/15 text-indigo-300 font-semibold border border-indigo-500/20 shrink-0", "Hermes Mode" }
                                    }
                                    p { class: "text-[10px] text-slate-450 font-medium mt-0.5 truncate", "Active Room ID: {active_id}" }
                                }
                            }
                            
                            div { class: "flex items-center gap-4 shrink-0",
                                button {
                                    class: "px-3 py-1.5 rounded-lg text-[11px] font-semibold bg-red-950/30 hover:bg-red-950/60 border border-red-900/40 hover:border-red-700/60 text-red-300 transition-all flex items-center gap-1.5 cursor-pointer shadow-md active:scale-95",
                                    onclick: move |_| {
                                        let mut msg_list = messages;
                                        let mut loading = is_loading;
                                        let active_id_reset = active_id.clone();
                                        spawn(async move {
                                            loading.write().insert(active_id_reset.clone(), true);
                                            match crate::send_chat_message(active_id_reset.clone(), "reset session".to_string()).await {
                                                Ok(_) => {
                                                    msg_list.write().insert(active_id_reset.clone(), vec![
                                                        ChatMessage {
                                                            is_user: false,
                                                            text: "Chat session has been reset. The next message will start a new conversation.".to_string(),
                                                            timestamp: chrono::Local::now().format("%H:%M").to_string(),
                                                        }
                                                    ]);
                                                }
                                                Err(e) => {
                                                    msg_list.write().entry(active_id_reset.clone()).or_default().push(ChatMessage {
                                                        is_user: false,
                                                        text: format!("⚠️ Error resetting chat: {}", e),
                                                        timestamp: chrono::Local::now().format("%H:%M").to_string(),
                                                    });
                                                }
                                            }
                                            loading.write().insert(active_id_reset.clone(), false);
                                        });
                                    },
                                    span { "🗑️" }
                                    span { "Reset Session" }
                                }
                                div { class: "h-4 w-[1px] bg-slate-800" }
                                div { class: "flex items-center gap-2",
                                    span { class: "h-2 w-2 rounded-full bg-emerald-500 animate-pulse" }
                                    span { class: "text-[10px] text-slate-400 font-mono", "Online" }
                                }
                            }
                        }

                        // Message Stream Area
                        div {
                            id: "chat-messages-container",
                            class: "flex-1 overflow-y-auto p-6 flex flex-col gap-6 scrollbar-thin scrollbar-thumb-slate-800 scrollbar-track-transparent",
                            for msg in display_messages.iter() {
                                div {
                                    class: format!("flex gap-3.5 max-w-[85%] {}", if msg.is_user { "self-end flex-row-reverse" } else { "self-start" }),
                                    
                                    if msg.is_user {
                                        div { class: "w-8 h-8 rounded-full bg-gradient-to-tr from-indigo-600 to-violet-600 flex items-center justify-center text-white text-xs font-bold border border-indigo-400/20 shadow-md flex-shrink-0",
                                            svg { class: "w-4 h-4", fill: "currentColor", view_box: "0 0 20 20",
                                                path { fill_rule: "evenodd", d: "M10 9a3 3 0 100-6 3 3 0 000 6zm-7 9a7 7 0 1114 0H3z", clip_rule: "evenodd" }
                                            }
                                        }
                                    } else {
                                        div { class: "w-8 h-8 rounded-full bg-gradient-to-tr from-slate-850 to-indigo-950 flex items-center justify-center text-indigo-400 text-xs border border-indigo-500/20 shadow-md flex-shrink-0",
                                            svg { class: "w-4.5 h-4.5 text-indigo-400", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" }
                                            }
                                        }
                                    }

                                    div { class: "flex flex-col",
                                        div {
                                            class: format!("px-4.5 py-3.5 rounded-2xl shadow-lg border text-slate-100 transition-all {}",
                                                if msg.is_user {
                                                    "bg-gradient-to-br from-indigo-600 to-indigo-700 border-indigo-500/40 rounded-tr-none text-white"
                                                } else {
                                                    "bg-slate-900/70 border-slate-800/80 rounded-tl-none"
                                                }
                                            ),
                                            {
                                                if msg.is_user {
                                                    let display_text = if msg.text == "agy-orchestrator info" || msg.text == "env -u PORT -u ADDR -u IP /home/wimvm/.local/bin/agy-orchestrator info" || msg.text == "/home/wimvm/.local/bin/agy-orchestrator info" {
                                                        "📋 System Info".to_string()
                                                    } else if msg.text == "agy-orchestrator list" || msg.text == "env -u PORT -u ADDR -u IP agy-orchestrator list" {
                                                        "🔍 List Projects".to_string()
                                                    } else if msg.text == "agy-orchestrator issue --list" {
                                                        "🐛 Active Issues".to_string()
                                                    } else {
                                                        msg.text.clone()
                                                    };
                                                    rsx! { p { class: "text-sm leading-relaxed", "{display_text}" } }
                                                } else {
                                                    render_markdown(&msg.text)
                                                }
                                            }
                                        }
                                        span { class: format!("text-[9px] text-slate-500 mt-1.5 font-mono px-1 {}", if msg.is_user { "text-right" } else { "text-left" }), "{msg.timestamp}" }
                                    }
                                }
                            }

                            if active_loading {
                                div { class: "self-start flex gap-3.5 max-w-[85%]",
                                    div { class: "w-8 h-8 rounded-full bg-gradient-to-tr from-slate-850 to-indigo-950 flex items-center justify-center text-indigo-400 border border-indigo-500/20 shadow flex-shrink-0",
                                        svg { class: "w-4.5 h-4.5 text-indigo-400 animate-pulse", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" }
                                        }
                                    }
                                    div { class: "flex items-center gap-1.5 bg-slate-900/50 border border-slate-800/80 px-4 py-3.5 rounded-2xl rounded-tl-none shadow-md",
                                        div { class: "h-2 w-2 bg-indigo-450 rounded-full animate-bounce [animation-duration:1s]" }
                                        div { class: "h-2 w-2 bg-indigo-450 rounded-full animate-bounce [animation-duration:1s] [animation-delay:0.2s]" }
                                        div { class: "h-2 w-2 bg-indigo-450 rounded-full animate-bounce [animation-duration:1s] [animation-delay:0.4s]" }
                                    }
                                }
                            }
                        }



                        // Input Bar
                        div { class: "bg-slate-900/60 border-t border-slate-850/80 p-4.5 flex gap-3 items-center backdrop-blur-md shrink-0",
                            input {
                                id: "chat-input-field",
                                class: "flex-1 bg-slate-950 border border-slate-850 rounded-xl px-4.5 py-3 text-sm text-slate-200 placeholder:text-slate-550 focus:outline-none focus:border-indigo-500/60 focus:ring-1 focus:ring-indigo-500/20 transition-all shadow-inner",
                                placeholder: "Ask your JIT secretary a question or command...",
                                value: "{input_text}",
                                oninput: move |evt| input_text.set(evt.value()),
                                onkeydown: move |evt| {
                                    if evt.key() == Key::Enter {
                                        send_message();
                                    }
                                }
                            }
                            button {
                                class: format!("px-5.5 py-3.5 rounded-xl font-bold text-sm transition-all duration-250 active:scale-95 flex items-center gap-2 cursor-pointer shadow-lg {}",
                                    if active_loading {
                                        "bg-slate-800 text-slate-500 border border-slate-750"
                                    } else {
                                        "bg-indigo-600 hover:bg-indigo-500 text-white shadow-indigo-900/30 hover:shadow-indigo-550/30 hover:shadow-xl"
                                    }
                                ),
                                onclick: move |_| send_message(),
                                disabled: active_loading,
                                span { "Send" }
                                span { class: "text-[12px]", "➔" }
                            }
                        }
                    }
                } else {
                    // Empty State View when no chat room is selected
                    div { class: "flex-1 flex flex-col items-center justify-center p-8 text-center gap-6",
                        div { class: "p-4.5 rounded-2xl bg-indigo-500/10 border border-indigo-500/20 text-indigo-400 text-3xl shadow-inner animate-pulse",
                            "✨"
                        }
                        div {
                            h2 { class: "text-md font-bold text-slate-100", "AI Orchestrator Secretary" }
                            p { class: "text-xs text-slate-450 max-w-sm mt-2 leading-relaxed",
                                "Manage your workflows, projects, and autonomous coding assistants in a Just-in-Time manner. Select a chat session room or start a fresh one to begin."
                            }
                        }
                        button {
                            class: "py-2.5 px-5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-bold text-xs transition-all duration-250 flex items-center gap-2 shadow-lg shadow-indigo-900/30 hover:shadow-indigo-550/30 active:scale-95 cursor-pointer",
                            onclick: move |_| {
                                let mut active_sig = active_session_id;
                                let mut sessions_sig = chat_sessions;
                                let mut msgs = messages;
                                spawn(async move {
                                    if let Ok(new_id) = crate::create_chat_session().await {
                                        active_sig.set(Some(new_id.clone()));
                                        if let Ok(s) = crate::get_chat_sessions().await {
                                            sessions_sig.set(s);
                                        }
                                        msgs.write().insert(new_id, Vec::new());
                                    }
                                });
                            },
                            span { class: "text-sm", "💬" }
                            span { "Create First Chat" }
                        }
                    }
                }
            }
        }
    }
}
