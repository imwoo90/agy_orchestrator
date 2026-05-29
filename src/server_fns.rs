use dioxus::prelude::*;
use std::collections::HashMap;
use crate::models::{ProjectInfo, Issue, ChatMessage, ChatSession, HealthCheckResult, FeedbackResponse, ChatResponse};

#[cfg(not(target_arch = "wasm32"))]
use crate::backend;

#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports)]
use crate::backend::chat_session::{
    self,
    promote_session_if_draft,
    append_mock_transcript_line,
    check_and_rename_session,
    get_transcript_content_by_id,
    register_draft_mapping,
    remove_draft_mapping,
    resolve_draft_id,
    load_chat_sessions,
    save_chat_sessions,
    get_brain_sessions,
    find_newest_brain_session,
    find_oldest_brain_session,
    find_parent_brain_session,
};

#[allow(dead_code)]
static DRAFT_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
struct HistoryCacheEntry {
    modified: std::time::SystemTime,
    file_size: u64,
    history: Vec<ChatMessage>,
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
static HISTORY_CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<String, HistoryCacheEntry>>> = std::sync::OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn log_notification(msg: &str) {
    use std::io::Write;
    let base_dir = backend::vault::get_base_dir();
    let notifications_path = base_dir.join("notifications.log");
    if let Ok(mut log_file) = std::fs::OpenOptions::new().create(true).append(true).open(&notifications_path) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(log_file, "[{}] {}", timestamp, msg);
    }
}

// Server Functions
#[server]
pub async fn get_projects() -> Result<HashMap<String, ProjectInfo>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(backend::state::load_state())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn get_issues() -> Result<Vec<Issue>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(backend::issue::load_issues())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn get_logs() -> Result<String, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let logs_path = backend::vault::get_base_dir().join("notifications.log");
        let data = std::fs::read_to_string(logs_path).unwrap_or_default();
        Ok(data)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn get_vault_notes() -> Result<Vec<(String, String)>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut notes = Vec::new();
        let vault_dir = backend::vault::get_base_dir().join("memory/vault");
        if let Ok(entries) = std::fs::read_dir(vault_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md") {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        notes.push((name, content));
                    }
                }
            }
        }
        Ok(notes)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn create_issue(title: String, body: String) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut issues = backend::issue::load_issues();
        let next_id = issues.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        issues.push(Issue {
            id: next_id,
            title,
            body,
            status: "open".to_string(),
            created_at: chrono::Local::now().to_rfc3339(),
            resolved_at: None,
        });
        backend::issue::save_issues(&issues).map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn submit_feedback_fn(raw_text: String) -> Result<FeedbackResponse, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        backend::issue::create_refined_feedback_issue(raw_text)
            .map_err(ServerFnError::new)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn save_vault_note(name: String, content: String) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let sanitized_topic = name
            .trim()
            .to_lowercase()
            .replace(".md", "")
            .replace(' ', "_")
            .replace(|c: char| !c.is_alphanumeric() && c != '_', "");

        if sanitized_topic.is_empty() {
            return Err(ServerFnError::new("Invalid note name"));
        }

        let vault_dir = backend::vault::get_base_dir().join("memory/vault");
        let file_path = vault_dir.join(format!("{}.md", sanitized_topic));
        std::fs::write(file_path, content).map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn get_system_health() -> Result<Vec<HealthCheckResult>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let health_path = backend::vault::get_base_dir().join("health.json");
        if health_path.exists() {
            if let Ok(data) = std::fs::read_to_string(health_path) {
                if let Ok(results) = serde_json::from_str(&data) {
                    return Ok(results);
                }
            }
        }
        Ok(Vec::new())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn inject_knowledge(project: String, note_name: String) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let state = backend::state::load_state();
        let info = match state.get(&project) {
            Some(i) => i,
            None => return Err(ServerFnError::new("Project not found")),
        };

        let vault_dir = backend::vault::get_base_dir().join("memory/vault");
        let note_path = vault_dir.join(&note_name);
        if !note_path.exists() {
            return Err(ServerFnError::new("Note not found"));
        }

        let note_content = std::fs::read_to_string(&note_path).map_err(|e| ServerFnError::new(e.to_string()))?;
        let context_path = std::path::Path::new(&info.path).join("context.md");
        
        let mut context_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&context_path)
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        use std::io::Write;
        writeln!(
            context_file,
            "\n\n# 🧠 Injected Knowledge from Note '{}' at {}\n\n{}",
            note_name, timestamp, note_content
        ).map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn get_daemon_status() -> Result<bool, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(backend::daemon::is_daemon_running())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn toggle_daemon() -> Result<bool, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let is_running = backend::daemon::is_daemon_running();
        if is_running {
            if let Some(pid) = backend::daemon::get_daemon_pid() {
                let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
                let pid_path = backend::vault::get_base_dir().join("daemon.pid");
                let _ = std::fs::remove_file(pid_path);
            }
            Ok(false)
        } else {
            let current_exe = std::env::current_exe().map_err(|e| ServerFnError::new(e.to_string()))?;
            let mut cmd = std::process::Command::new(&current_exe);
            cmd.arg("daemon").arg("--start");
            cmd.env_remove("PORT");
            cmd.env_remove("ADDR");
            cmd.env_remove("IP");
            cmd.env_remove("DIOXUS_ACTIVE");
            cmd.status().map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(true)
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn spawn_project_task(name: String, path: String, goal: String) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let cli_struct = backend::cli::Cli {
            command: backend::cli::Commands::Spawn { name, path, goal }
        };
        backend::cli::run_cli(cli_struct).map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn get_upgrade_status() -> Result<Option<(String, String)>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        backend::upgrade::check_latest_release().map_err(|e| ServerFnError::new(e.to_string()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn get_chat_history(session_id: String) -> Result<Vec<ChatMessage>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let resolved_id = resolve_draft_id(&session_id);
        if resolved_id.is_empty() || resolved_id.starts_with("draft-") {
            return Ok(Vec::new());
        }

        let transcript_path = backend::vault::get_brain_dir()
            .join(&resolved_id)
            .join(".system_generated/logs/transcript_full.jsonl");

        if !transcript_path.exists() {
            return Ok(Vec::new());
        }

        let metadata = std::fs::metadata(&transcript_path)
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let file_size = metadata.len();

        let cache = HISTORY_CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
        if let Ok(lock) = cache.lock() {
            if let Some(entry) = lock.get(&resolved_id) {
                if entry.modified == modified && entry.file_size == file_size {
                    return Ok(entry.history.clone());
                }
            }
        }

        let file_content = std::fs::read_to_string(&transcript_path)
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut history = Vec::new();
        for line in file_content.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                let source = json["source"].as_str().unwrap_or("");
                let msg_type = json["type"].as_str().unwrap_or("");
                let content = json["content"].as_str().unwrap_or("");

                let timestamp = if let Some(ts_str) = json["created_at"].as_str() {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                        dt.with_timezone(&chrono::Local).format("%H:%M").to_string()
                    } else {
                        chrono::Local::now().format("%H:%M").to_string()
                    }
                } else {
                    chrono::Local::now().format("%H:%M").to_string()
                };

                if msg_type == "USER_INPUT" {
                    let mut text = content.to_string();
                    if let Some(user_msg_idx) = text.find("User Message:") {
                        let after_prefix = &text[user_msg_idx + "User Message:".len()..];
                        if let Some(end_req_idx) = after_prefix.find("</USER_REQUEST>") {
                            text = after_prefix[..end_req_idx].trim().to_string();
                        } else {
                            text = after_prefix.trim().to_string();
                        }
                    } else {
                        if text.starts_with("<USER_REQUEST>") {
                            text = text.replace("<USER_REQUEST>", "").replace("</USER_REQUEST>", "").trim().to_string();
                        }
                    }

                    if let Some(banner_idx) = text.find("==================================================") {
                        text = text[..banner_idx].trim().to_string();
                    }

                    history.push(ChatMessage {
                        is_user: true,
                        text,
                        timestamp,
                    });
                } else if source == "MODEL" && msg_type == "PLANNER_RESPONSE" {
                    let mut text = content.to_string();
                    if let Some(start_idx) = text.find("[CREATE_TASK:") {
                        if let Some(end_idx) = text[start_idx..].find(']') {
                            let full_tag = &text[start_idx..start_idx + end_idx + 1];
                            text = text.replace(full_tag, "").trim().to_string();
                        }
                    }
                    
                    let clean_val = |s: &str| -> String {
                        let mut val = s.trim();
                        while val.starts_with('\\') || val.starts_with('"') {
                            val = &val[1..];
                        }
                        while val.ends_with('\\') || val.ends_with('"') {
                            val = &val[..val.len() - 1];
                        }
                        val.trim().to_string()
                    };

                    let mut tool_desc = String::new();
                    if let Some(tool_calls) = json["tool_calls"].as_array() {
                        if !tool_calls.is_empty() {
                            tool_desc.push_str("```\n");
                            tool_desc.push_str("┌────────────────────────────────────────────────────────┐\n");
                            tool_desc.push_str("│  🔧 TOOL EXECUTION                                      │\n");
                            tool_desc.push_str("└────────────────────────────────────────────────────────┘\n");
                            for tool in tool_calls {
                                let name = tool["name"].as_str().unwrap_or("");
                                let args = &tool["args"];
                                
                                let action = args["toolAction"].as_str()
                                    .or_else(|| args["toolSummary"].as_str())
                                    .unwrap_or("")
                                    .trim_matches('"');
                                
                                let action_clean = clean_val(action);
                                if !action_clean.is_empty() {
                                    tool_desc.push_str(&format!(" ▶ {}\n", action_clean));
                                } else {
                                    tool_desc.push_str(&format!(" ▶ Executing: {}\n", name));
                                }
                                
                                if name == "run_command" {
                                    if let Some(cmd) = args["CommandLine"].as_str() {
                                        tool_desc.push_str(&format!("   💻 $ {}\n", clean_val(cmd)));
                                    }
                                } else if name == "view_file" || name == "write_to_file" || name == "replace_file_content" || name == "multi_replace_file_content" {
                                    if let Some(path) = args["AbsolutePath"].as_str().or_else(|| args["TargetFile"].as_str()) {
                                        let path_clean = clean_val(path);
                                        let file_name = std::path::Path::new(&path_clean)
                                            .file_name()
                                            .and_then(|f| f.to_str())
                                            .unwrap_or(&path_clean);
                                        tool_desc.push_str(&format!("   📄 file: {}\n", file_name));
                                    }
                                } else if name == "grep_search" {
                                    if let Some(query) = args["Query"].as_str() {
                                        tool_desc.push_str(&format!("   🔍 query: \"{}\"\n", clean_val(query)));
                                    }
                                } else if name == "list_dir" {
                                    if let Some(path) = args["DirectoryPath"].as_str() {
                                        tool_desc.push_str(&format!("   📂 dir: {}\n", clean_val(path)));
                                    }
                                } else if name == "invoke_subagent" {
                                    if let Some(subagents) = args["Subagents"].as_array() {
                                        for agent in subagents {
                                            if let Some(role) = agent["Role"].as_str() {
                                                tool_desc.push_str(&format!("   👤 subagent: {}\n", clean_val(role)));
                                            }
                                        }
                                    }
                                }
                            }
                            tool_desc.push_str("──────────────────────────────────────────────────────────\n");
                            tool_desc.push_str("```");
                        }
                    }

                    if !text.is_empty() || !tool_desc.is_empty() {
                        let mut final_text = text;
                        if !tool_desc.is_empty() {
                            if !final_text.is_empty() {
                                final_text.push_str("\n\n");
                            }
                            final_text.push_str(&tool_desc);
                        }
                        history.push(ChatMessage {
                            is_user: false,
                            text: final_text,
                            timestamp,
                        });
                    }
                }
            }
        }
        if let Ok(mut lock) = cache.lock() {
            lock.insert(resolved_id, HistoryCacheEntry {
                modified,
                file_size,
                history: history.clone(),
            });
        }
        Ok(history)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn is_chat_session_busy(session_id: String) -> Result<bool, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let resolved_id = resolve_draft_id(&session_id);
        if resolved_id.is_empty() || resolved_id.starts_with("draft-") {
            return Ok(false);
        }
        Ok(backend::agy_runner::is_session_busy(&resolved_id))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}


#[server]
pub async fn get_chat_sessions() -> Result<Vec<ChatSession>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        load_chat_sessions().map_err(ServerFnError::new)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn create_chat_session() -> Result<String, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let ts = chrono::Utc::now().timestamp_millis();
        let count = DRAFT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let id = format!("draft-{}-{}", ts, count);
        let timestamp = chrono::Local::now().to_rfc3339();
        let new_session = ChatSession {
            id: id.clone(),
            title: "New Chat".to_string(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        
        let mut sessions = load_chat_sessions().unwrap_or_default();
        sessions.push(new_session);
        save_chat_sessions(&sessions).map_err(ServerFnError::new)?;
        
        let base_dir = backend::vault::get_base_dir();
        let _ = std::fs::write(base_dir.join("active_chat_session_id.txt"), &id);
        
        Ok(id)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn delete_chat_session(id: String) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        backend::agy_runner::terminate_persistent_session(&id);

        let mut sessions = load_chat_sessions().unwrap_or_default();
        sessions.retain(|s| s.id != id);
        save_chat_sessions(&sessions).map_err(ServerFnError::new)?;
        
        let brain_dir = backend::vault::get_brain_dir().join(&id);
        if brain_dir.exists() {
            let _ = std::fs::remove_dir_all(brain_dir);
        }
        
        let base_dir = backend::vault::get_base_dir();
        let active_path = base_dir.join("active_chat_session_id.txt");
        if active_path.exists() {
            if let Ok(active_id) = std::fs::read_to_string(&active_path) {
                if active_id.trim() == id {
                    let _ = std::fs::remove_file(&active_path);
                }
            }
        }
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn get_active_session_id() -> Result<Option<String>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let base_dir = backend::vault::get_base_dir();
        let active_path = base_dir.join("active_chat_session_id.txt");
        if !active_path.exists() {
            return Ok(None);
        }
        let id = std::fs::read_to_string(active_path)
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .trim()
            .to_string();
        if id.is_empty() {
            Ok(None)
        } else {
            Ok(Some(id))
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn set_active_session_id(id: String) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let base_dir = backend::vault::get_base_dir();
        let active_path = base_dir.join("active_chat_session_id.txt");
        std::fs::write(active_path, id.trim()).map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn trigger_remote_upgrade(download_url: String) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        backend::upgrade::run_remote_upgrade(&download_url).map_err(|e| ServerFnError::new(e.to_string()))?;
        
        // Spawn a background task to restart the dashboard after a short delay
        tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
            let stable_exe = std::path::PathBuf::from(home).join(".local/bin/agy-orchestrator");
            let spawn_exe = if stable_exe.exists() {
                stable_exe
            } else if let Ok(curr) = std::env::current_exe() {
                curr
            } else {
                std::process::exit(0);
            };

            let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
            let mut cmd = std::process::Command::new(&spawn_exe);
            cmd.arg("dashboard").arg("--port").arg(&port);
                
                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    extern "C" {
                        fn setsid() -> i32;
                    }
                    unsafe {
                        cmd.pre_exec(|| {
                            setsid();
                            Ok(())
                        });
                    }
                }
                
                let _ = cmd.spawn();
            std::process::exit(0);
        });
        
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn resolve_issue_fn(id: u32) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut issues = backend::issue::load_issues();
        if let Some(issue) = issues.iter_mut().find(|i| i.id == id) {
            issue.status = "resolved".to_string();
            issue.resolved_at = Some(chrono::Local::now().to_rfc3339());
            backend::issue::save_issues(&issues).map_err(|e| ServerFnError::new(e.to_string()))?;
            log_notification(&format!("INFO: Issue #{} was manually resolved.", id));
            Ok(())
        } else {
            Err(ServerFnError::new("Issue not found"))
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
pub async fn run_evolution_harness_fn(id: u32) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut issues = backend::issue::load_issues();
        if let Some(issue) = issues.iter_mut().find(|i| i.id == id) {
            issue.status = "in-progress".to_string();
            backend::issue::save_issues(&issues).map_err(|e| ServerFnError::new(e.to_string()))?;
        } else {
            return Err(ServerFnError::new("Issue not found"));
        }

        tokio::spawn(async move {
            log_notification(&format!("INFO: Starting evolution-harness in background for Issue #{}", id));
            match backend::upgrade::run_evolution_harness(id) {
                Ok(_) => {
                    log_notification(&format!("INFO: Evolution-harness for Issue #{} completed successfully!", id));
                }
                Err(e) => {
                    log_notification(&format!("ERROR: Evolution-harness for Issue #{} failed: {}", id, e));
                }
            }
        });
        Ok(())
    }
}

#[server]
pub async fn send_chat_message(session_id: String, message: String) -> Result<ChatResponse, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let msg_trimmed = message.trim();
        if msg_trimmed.is_empty() {
            return Ok(ChatResponse {
                reply: "Please enter a non-empty message.".to_string(),
                actual_session_id: session_id,
            });
        }
        if session_id.is_empty() {
            return Err(ServerFnError::new("No active session ID provided"));
        }

        let actual_session_id = promote_session_if_draft(&session_id);
        
        if backend::agy_runner::is_session_busy(&actual_session_id) {
            return Ok(ChatResponse {
                reply: "⚠️ The orchestrator is currently processing a task. Please wait until the active task is completed.".to_string(),
                actual_session_id,
            });
        }

        let lower_msg = msg_trimmed.to_lowercase();

        // Short-circuit: direct agy-orchestrator commands execution
        let is_direct_cmd = {
            let parts: Vec<&str> = msg_trimmed.split_whitespace().collect();
            if let Some(first) = parts.first() {
                let first_lower = first.to_lowercase();
                first_lower == "agy-orchestrator" || 
                first_lower.ends_with("/agy-orchestrator") ||
                first_lower.ends_with("\\agy-orchestrator")
            } else {
                false
            }
        };
        if is_direct_cmd {
            let cmd_parts: Vec<&str> = msg_trimmed.split_whitespace().collect();
            if !cmd_parts.is_empty() {
                let mut args = Vec::new();
                let mut found_bin = false;
                for part in cmd_parts {
                    if found_bin {
                        args.push(part);
                    } else if part.contains("agy-orchestrator") {
                        found_bin = true;
                    }
                }
                
                let mut exec_cmd = std::process::Command::new("/home/wimvm/.local/bin/agy-orchestrator");
                exec_cmd.args(&args);
                exec_cmd.env_remove("PORT");
                exec_cmd.env_remove("ADDR");
                exec_cmd.env_remove("IP");
                exec_cmd.env_remove("DIOXUS_ACTIVE");
                
                if let Ok(cmd_out) = exec_cmd.output() {
                    let stdout_str = String::from_utf8_lossy(&cmd_out.stdout).to_string();
                    let stderr_str = String::from_utf8_lossy(&cmd_out.stderr).to_string();
                    
                    let mut reply = String::new();
                    if !stdout_str.is_empty() {
                        reply.push_str("```\n");
                        reply.push_str(&stdout_str);
                        reply.push_str("\n```");
                    }
                    if !stderr_str.is_empty() {
                        if !reply.is_empty() {
                            reply.push_str("\n\n");
                        }
                        reply.push_str("⚠️ **Stderr Output**:\n```\n");
                        reply.push_str(&stderr_str);
                        reply.push_str("\n```");
                    }
                    if reply.is_empty() {
                        reply = "Command executed successfully with no output.".to_string();
                    }
                    
                    let _ = append_mock_transcript_line(&actual_session_id, "USER_INPUT", msg_trimmed);
                    let _ = append_mock_transcript_line(&actual_session_id, "PLANNER_RESPONSE", &reply);
                    
                    return Ok(ChatResponse {
                        reply,
                        actual_session_id,
                    });
                }
            }
        }

        if lower_msg.starts_with("create task:") || lower_msg.starts_with("add task:") || lower_msg.starts_with("new task:") {
            let prefix_len = if lower_msg.starts_with("create task:") {
                "create task:".len()
            } else if lower_msg.starts_with("add task:") {
                "add task:".len()
            } else {
                "new task:".len()
            };
            let title = msg_trimmed[prefix_len..].trim().to_string();
            if title.is_empty() {
                return Ok(ChatResponse {
                    reply: "Please specify a task title (e.g. `create task: Fix layout`).".to_string(),
                    actual_session_id,
                });
            }

            let mut issues = backend::issue::load_issues();
            let next_id = issues.iter().map(|i| i.id).max().unwrap_or(0) + 1;
            issues.push(Issue {
                id: next_id,
                title: title.clone(),
                body: format!("Automatically created via chat: {}", title),
                status: "open".to_string(),
                created_at: chrono::Local::now().to_rfc3339(),
                resolved_at: None,
            });
            backend::issue::save_issues(&issues).map_err(|e| ServerFnError::new(e.to_string()))?;
            
            let _ = check_and_rename_session(&actual_session_id, &format!("Create Task: {}", title));

            return Ok(ChatResponse {
                reply: format!("I have automatically created the task: **{}** (#{})! You can find it on your Kanban board.", title, next_id),
                actual_session_id,
            });
        }

        if lower_msg == "help" {
            return Ok(ChatResponse {
                reply: "I am your AGY Orchestrator Assistant! Here are the commands you can use:\n\n- **create task: [Title]** - Automate task creation.\n- **add task: [Title]** - Automate task creation.\n- **reset session** / **reset chat** - Reset conversation history.\n\nType conversational requests like *'I need to fix X'* to talk to the AI (runs using `agy` command).".to_string(),
                actual_session_id,
            });
        }

        if lower_msg == "clear session" || lower_msg == "reset session" || lower_msg == "reset chat" {
            let brain_dir = backend::vault::get_brain_dir().join(&actual_session_id);
            if brain_dir.exists() {
                let _ = std::fs::remove_dir_all(brain_dir);
            }
            return Ok(ChatResponse {
                reply: "This chat session has been reset. The next message will start a new conversation.".to_string(),
                actual_session_id,
            });
        }

        let transcript_path = backend::vault::get_brain_dir()
            .join(&actual_session_id)
            .join(".system_generated/logs/transcript_full.jsonl");
        let is_first_turn = !transcript_path.exists()
            || std::fs::metadata(&transcript_path).map(|m| m.len()).unwrap_or(0) == 0;

        let prompt_payload = if msg_trimmed.starts_with('/') {
            msg_trimmed.to_string()
        } else if is_first_turn {
            // Check if simple chat
            let is_simple_chat = {
                let lower = msg_trimmed.to_lowercase();
                let is_short = msg_trimmed.chars().count() < 40;
                
                let has_orchestration_keywords = 
                    lower.contains("task") || 
                    lower.contains("issue") || 
                    lower.contains("project") || 
                    lower.contains("status") || 
                    lower.contains("health") || 
                    lower.contains("upgrade") || 
                    lower.contains("update") || 
                    lower.contains("list") || 
                    lower.contains("harness") || 
                    lower.contains("daemon") || 
                    lower.contains("log") || 
                    lower.contains("context") ||
                    lower.contains("일감") ||
                    lower.contains("이슈") ||
                    lower.contains("프로젝트") ||
                    lower.contains("상태") ||
                    lower.contains("업그레이드") ||
                    lower.contains("업데이트") ||
                    lower.contains("목록") ||
                    lower.contains("하네스") ||
                    lower.contains("데몬") ||
                    lower.contains("로그");
                
                is_short && !has_orchestration_keywords
            };

            let system_instruction = if is_simple_chat {
                "You are a friendly personal secretary AI assistant. Respond to the user's message directly, briefly, and instantly in the same language. Do not plan, do not write code, and do not use tools.".to_string()
            } else {
                "You are the Central Orchestrator (Personal Secretary) AI assistant. \
                CRITICAL FIRST STEP: You must immediately read your system guidelines and operational commands from the local file \
                '/home/wimvm/.agy_orchestrator/memory/system_instructions.md' using your view_file tool to understand your specialized role, commands, and rules. \
                Then, process the user's message accordingly.".to_string()
            };

            let tool_format_instruction = "\n\n==================================================\n\
CRITICAL TOOL CALL FORMATTING RULES:\n\
When calling platform tools (e.g., view_file, list_dir, grep_search, write_to_file, replace_file_content):\n\
- Do NOT wrap string arguments (like paths or queries) in nested or escaped double quotes.\n\
- Correct: \"AbsolutePath\": \"/path/to/file\"\n\
- Incorrect: \"AbsolutePath\": \"\\\"/path/to/file\\\"\"\n\
Failure to follow this will cause sandbox permission validation to time out and fail!\n\
==================================================\n";

            format!(
                "[System Instruction: {}]\n\nUser Message: {}{}",
                system_instruction,
                msg_trimmed,
                tool_format_instruction
            )
        } else {
            msg_trimmed.to_string()
        };

        // Replace newlines with spaces to avoid PTY command line separation/race issues.
        let prompt_payload = prompt_payload.replace("\r\n", " ").replace("\n", " ");

        if session_id != actual_session_id {
            let old_dir = backend::vault::get_brain_dir().join(&session_id);
            if old_dir.exists() {
                let new_dir = backend::vault::get_brain_dir().join(&actual_session_id);
                let _ = std::fs::rename(old_dir, new_dir);
            }
            register_draft_mapping(session_id.clone(), actual_session_id.clone());
        }

        // 1. Ensure the persistent process is initialized and active
        backend::agy_runner::get_or_create_persistent_session(&actual_session_id)
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // 2. Mark the session as busy
        backend::agy_runner::set_session_busy(&actual_session_id, true);

        // 3. Spawn background thread to run agy execution and auto-approve permissions
        let final_session_id = actual_session_id.clone();
        let prompt_payload_clone = prompt_payload.clone();
        let msg_trimmed_clone = msg_trimmed.to_string();
        let session_id_clone = session_id.clone();

        tokio::spawn(async move {
            let brain_dir = backend::vault::get_brain_dir().join(&final_session_id);
            let log_file_path = brain_dir.join(".system_generated/logs/agy_stdout.log");

            let res = backend::agy_runner::send_interactive_message(&final_session_id, &prompt_payload_clone, Some(&log_file_path));

            // Clear busy flag
            backend::agy_runner::set_session_busy(&final_session_id, false);

            // Session promotion migration and renaming
            remove_draft_mapping(&session_id_clone);
            let _ = check_and_rename_session(&final_session_id, &msg_trimmed_clone);

            if let Err(e) = res {
                log_notification(&format!("ERROR running agy in background: {}", e));
            } else {
                // Post-process response to handle any [CREATE_TASK: ...] tags
                if let Ok(clean_reply) = get_transcript_content_by_id(&final_session_id) {
                    if let Some(start_idx) = clean_reply.find("[CREATE_TASK:") {
                        if let Some(end_idx) = clean_reply[start_idx..].find(']') {
                            let full_tag = &clean_reply[start_idx..start_idx + end_idx + 1];
                            let content = &clean_reply[start_idx + "[CREATE_TASK:".len()..start_idx + end_idx];
                            let parts: Vec<&str> = content.split('|').collect();
                            let title = parts.first().unwrap_or(&"").trim().to_string();
                            let body = parts.get(1).unwrap_or(&"").trim().to_string();

                            if !title.is_empty() {
                                let mut issues = backend::issue::load_issues();
                                let next_id = issues.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                                issues.push(Issue {
                                    id: next_id,
                                    title: title.clone(),
                                    body: if body.is_empty() { format!("Automatically created via chat: {}", title) } else { body },
                                    status: "open".to_string(),
                                    created_at: chrono::Local::now().to_rfc3339(),
                                    resolved_at: None,
                                });
                                let _ = backend::issue::save_issues(&issues);

                                let mut updated_reply = clean_reply.replace(full_tag, "").trim().to_string();
                                updated_reply.push_str(&format!("\n\n*(Created task: **{}** [#{}])*", title, next_id));
                                
                                // Rewrite the transcript response line to update it with the formatted task text
                                let transcript_path = backend::vault::get_brain_dir()
                                    .join(&final_session_id)
                                    .join(".system_generated/logs/transcript_full.jsonl");
                                if let Ok(file_content) = std::fs::read_to_string(&transcript_path) {
                                    let mut lines: Vec<String> = file_content.lines().map(|l| l.to_string()).collect();
                                    for line in lines.iter_mut().rev() {
                                        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(line) {
                                            if json["source"] == "MODEL" && json["type"] == "PLANNER_RESPONSE" {
                                                json["content"] = serde_json::json!(updated_reply);
                                                if let Ok(new_line) = serde_json::to_string(&json) {
                                                    *line = new_line;
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    let _ = std::fs::write(&transcript_path, lines.join("\n") + "\n");
                                }
                            }
                        }
                    }
                }
            }
        });

        // 4. Return immediately to the frontend to prevent HTTP timeout
        Ok(ChatResponse {
            reply: "".to_string(),
            actual_session_id: actual_session_id,
        })
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}
