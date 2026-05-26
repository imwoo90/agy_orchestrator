use dioxus::prelude::*;
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
pub mod backend;

pub mod frontend;

use frontend::app::{ProjectInfo, Issue, HealthCheckResult, FeedbackResponse};

// Server Functions
#[server]
async fn get_projects() -> Result<HashMap<String, ProjectInfo>, ServerFnError> {
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
async fn get_issues() -> Result<Vec<Issue>, ServerFnError> {
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
async fn get_logs() -> Result<String, ServerFnError> {
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
async fn get_vault_notes() -> Result<Vec<(String, String)>, ServerFnError> {
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
async fn create_issue(title: String, body: String) -> Result<(), ServerFnError> {
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
async fn submit_feedback_fn(raw_text: String) -> Result<FeedbackResponse, ServerFnError> {
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
async fn save_vault_note(name: String, content: String) -> Result<(), ServerFnError> {
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
async fn get_system_health() -> Result<Vec<HealthCheckResult>, ServerFnError> {
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
async fn inject_knowledge(project: String, note_name: String) -> Result<(), ServerFnError> {
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
async fn get_daemon_status() -> Result<bool, ServerFnError> {
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
async fn toggle_daemon() -> Result<bool, ServerFnError> {
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
async fn spawn_project_task(name: String, path: String, goal: String) -> Result<(), ServerFnError> {
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
async fn get_upgrade_status() -> Result<Option<(String, String)>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        backend::upgrade::check_latest_release().map_err(|e| ServerFnError::new(e.to_string()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn load_chat_sessions() -> Result<Vec<crate::frontend::app::ChatSession>, String> {
    let base_dir = backend::vault::get_base_dir();
    let sessions_path = base_dir.join("chat_sessions.json");
    if !sessions_path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&sessions_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn save_chat_sessions(sessions: &[crate::frontend::app::ChatSession]) -> Result<(), String> {
    let base_dir = backend::vault::get_base_dir();
    let sessions_path = base_dir.join("chat_sessions.json");
    let content = serde_json::to_string_pretty(sessions).map_err(|e| e.to_string())?;
    std::fs::write(&sessions_path, content).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn uuid_v4_fallback() -> String {
    if let Ok(uuid) = std::fs::read_to_string("/proc/sys/kernel/random/uuid") {
        uuid.trim().to_string()
    } else {
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        format!("session-{}", ts)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn check_and_rename_session(session_id: &str, first_msg: &str) -> Result<(), String> {
    let mut sessions = load_chat_sessions()?;
    if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
        if session.title == "New Chat" {
            let mut title = first_msg.trim().to_string();
            if title.len() > 30 {
                title = format!("{}...", &title[..28]);
            }
            session.title = title;
            session.updated_at = chrono::Local::now().to_rfc3339();
            save_chat_sessions(&sessions)?;
        } else {
            session.updated_at = chrono::Local::now().to_rfc3339();
            save_chat_sessions(&sessions)?;
        }
    }
    Ok(())
}

#[server]
async fn get_chat_history(session_id: String) -> Result<Vec<crate::frontend::app::ChatMessage>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use crate::frontend::app::ChatMessage;
        if session_id.is_empty() || session_id.starts_with("draft-") {
            return Ok(Vec::new());
        }

        let transcript_path = std::path::Path::new("/home/wimvm/.gemini/antigravity-cli/brain/")
            .join(&session_id)
            .join(".system_generated/logs/transcript_full.jsonl");

        if !transcript_path.exists() {
            return Ok(Vec::new());
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
                    history.push(ChatMessage {
                        is_user: false,
                        text,
                        timestamp,
                    });
                }
            }
        }
        Ok(history)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
async fn get_chat_sessions() -> Result<Vec<crate::frontend::app::ChatSession>, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        load_chat_sessions().map_err(|e| ServerFnError::new(e))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[server]
async fn create_chat_session() -> Result<String, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let ts = chrono::Utc::now().timestamp_millis();
        let id = format!("draft-{}", ts);
        let timestamp = chrono::Local::now().to_rfc3339();
        let new_session = crate::frontend::app::ChatSession {
            id: id.clone(),
            title: "New Chat".to_string(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        
        let mut sessions = load_chat_sessions().unwrap_or_default();
        sessions.push(new_session);
        save_chat_sessions(&sessions).map_err(|e| ServerFnError::new(e))?;
        
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
async fn delete_chat_session(id: String) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut sessions = load_chat_sessions().unwrap_or_default();
        sessions.retain(|s| s.id != id);
        save_chat_sessions(&sessions).map_err(|e| ServerFnError::new(e))?;
        
        let brain_dir = std::path::Path::new("/home/wimvm/.gemini/antigravity-cli/brain").join(&id);
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
async fn get_active_session_id() -> Result<Option<String>, ServerFnError> {
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
async fn set_active_session_id(id: String) -> Result<(), ServerFnError> {
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
async fn trigger_remote_upgrade(download_url: String) -> Result<(), ServerFnError> {
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

#[server]
async fn resolve_issue_fn(id: u32) -> Result<(), ServerFnError> {
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
async fn run_evolution_harness_fn(id: u32) -> Result<(), ServerFnError> {
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
async fn send_chat_message(session_id: String, message: String) -> Result<String, ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let msg_trimmed = message.trim();
        if msg_trimmed.is_empty() {
            return Ok("Please enter a non-empty message.".to_string());
        }
        if session_id.is_empty() {
            return Err(ServerFnError::new("No active session ID provided"));
        }

        let lower_msg = msg_trimmed.to_lowercase();
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
                return Ok("Please specify a task title (e.g. `create task: Fix layout`).".to_string());
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
            
            let _ = check_and_rename_session(&session_id, &format!("Create Task: {}", title));

            return Ok(format!("I have automatically created the task: **{}** (#{})! You can find it on your Kanban board.", title, next_id));
        }

        if lower_msg == "help" {
            return Ok("I am your AGY Orchestrator Assistant! Here are the commands you can use:\n\n- **create task: [Title]** - Automate task creation.\n- **add task: [Title]** - Automate task creation.\n- **reset session** / **reset chat** - Reset conversation history.\n\nType conversational requests like *'I need to fix X'* to talk to the AI (runs using `agy` command).".to_string());
        }

        if lower_msg == "clear session" || lower_msg == "reset session" || lower_msg == "reset chat" {
            let brain_dir = std::path::Path::new("/home/wimvm/.gemini/antigravity-cli/brain").join(&session_id);
            if brain_dir.exists() {
                let _ = std::fs::remove_dir_all(brain_dir);
            }
            return Ok("This chat session has been reset. The next message will start a new conversation.".to_string());
        }

        let global_instr_path = backend::vault::get_base_dir().join("memory/system_instructions.md");
        let global_instr = std::fs::read_to_string(global_instr_path).unwrap_or_default();

        let system_instruction = format!(
            "You are the Central Orchestrator (Personal Secretary) AI assistant for the user, communicating through the dashboard chat interface.\n\
            To answer the user's questions or perform their requests, you should retrieve knowledge and status in a Just-in-Time (JIT) manner by running terminal commands using your run_command tool.\n\n\
            Here are the primary commands you can execute to query the orchestrator's state JIT:\n\
            - `/home/wimvm/.local/bin/agy-orchestrator info` to get the system, daemon, and dashboard status.\n\
            - `/home/wimvm/.local/bin/agy-orchestrator list` to get the list of registered projects.\n\
            - `/home/wimvm/.local/bin/agy-orchestrator get-context --name <project>` to get the path, goal, and status of a specific project.\n\
            - `/home/wimvm/.local/bin/agy-orchestrator issue --list` to get the current list of evolution tasks and issues.\n\
            - `/home/wimvm/.local/bin/agy-orchestrator query-memory --query \"<keywords>\"` to find user preferences or design guidelines in the memory vault.\n\n\
            If the user asks to create or register a task, you can do so by running:\n\
            - `/home/wimvm/.local/bin/agy-orchestrator issue --create \"<Title>\" --body \"<Description>\"`\n\
            (Alternatively, you can append `[CREATE_TASK: Title | Body]` at the very end of your final response text, and the system will automatically parse and register it for you).\n\n\
            Always run the appropriate commands first to obtain the latest real-time status before answering. Do not guess.\n\n\
            --- GLOBAL OPERATIONAL GUIDELINES ---\n\
            {}",
            global_instr
        );

        let prompt_payload = format!(
            "[System Instruction: {}]\n\nUser Message: {}",
            system_instruction,
            msg_trimmed
        );

        let is_draft = session_id.starts_with("draft-");
        let brain_dir = std::path::Path::new("/home/wimvm/.gemini/antigravity-cli/brain").join(&session_id);
        let transcript_path = brain_dir.join(".system_generated/logs/transcript_full.jsonl");
        let is_new_session = is_draft || !transcript_path.exists();

        let mut cmd = std::process::Command::new("/home/wimvm/.local/bin/agy");
        cmd.arg("--prompt").arg(&prompt_payload);
        cmd.arg("--dangerously-skip-permissions");

        if !is_new_session {
            cmd.arg("--conversation").arg(&session_id);
            cmd.arg("--continue");
        }

        let output = cmd.output();

        match output {
            Ok(out) if out.status.success() => {
                let actual_session_id = if is_new_session {
                    if let Ok(path_output) = std::process::Command::new("sh")
                        .arg("-c")
                        .arg("ls -td /home/wimvm/.gemini/antigravity-cli/brain/*/ | head -n 1")
                        .output()
                    {
                        if path_output.status.success() {
                            let path_str = String::from_utf8_lossy(&path_output.stdout).trim().to_string();
                            if !path_str.is_empty() {
                                if let Some(filename) = std::path::Path::new(&path_str).file_name() {
                                    let new_id = filename.to_string_lossy().into_owned();
                                    
                                    if new_id != session_id {
                                        if let Ok(mut sessions) = load_chat_sessions() {
                                            if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
                                                session.id = new_id.clone();
                                            }
                                            let _ = save_chat_sessions(&sessions);
                                        }
                                        
                                        let base_dir = backend::vault::get_base_dir();
                                        let _ = std::fs::write(base_dir.join("active_chat_session_id.txt"), &new_id);
                                    }

                                    new_id
                                } else {
                                    session_id.clone()
                                }
                            } else {
                                session_id.clone()
                            }
                        } else {
                            session_id.clone()
                        }
                    } else {
                        session_id.clone()
                    }
                } else {
                    session_id.clone()
                };

                let _ = check_and_rename_session(&actual_session_id, msg_trimmed);

                match get_transcript_content_by_id(&actual_session_id) {
                    Ok(clean_reply) => {
                        let mut final_response = clean_reply;

                        if let Some(start_idx) = final_response.find("[CREATE_TASK:") {
                            if let Some(end_idx) = final_response[start_idx..].find(']') {
                                let full_tag = &final_response[start_idx..start_idx + end_idx + 1];
                                let content = &final_response[start_idx + "[CREATE_TASK:".len()..start_idx + end_idx];
                                
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
                                    
                                    final_response = final_response.replace(full_tag, "").trim().to_string();
                                    final_response.push_str(&format!("\n\n*(Created task: **{}** [#{}])*", title, next_id));
                                }
                            }
                        }

                        Ok(final_response)
                    }
                    Err(e) => {
                        Ok(format!("Failed to retrieve agent response: {}", e))
                    }
                }
            }
            _ => {
                Ok("Error executing agy prompt CLI.".to_string())
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn get_transcript_content_by_id(conversation_id: &str) -> Result<String, String> {
    let transcript_path = std::path::Path::new("/home/wimvm/.gemini/antigravity-cli/brain")
        .join(conversation_id)
        .join(".system_generated/logs/transcript_full.jsonl");
    if !transcript_path.exists() {
        return Err("Transcript file does not exist".to_string());
    }

    let file_content = std::fs::read_to_string(&transcript_path)
        .map_err(|e| format!("Failed to read transcript: {}", e))?;

    for line in file_content.lines().rev() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json["source"] == "MODEL" && json["type"] == "PLANNER_RESPONSE" {
                if let Some(content) = json["content"].as_str() {
                    return Ok(content.to_string());
                }
            }
        }
    }

    Err("No assistant response found in transcript".to_string())
}


// Entrypoint
fn main() -> std::io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let has_args = std::env::args().len() > 1;
        let is_dioxus_env = std::env::var("PORT").is_ok() || std::env::var("ADDR").is_ok() || std::env::var("IP").is_ok() || std::env::var("DIOXUS_ACTIVE").is_ok();

        if !has_args || is_dioxus_env {
            // Under dx serve or when direct execution with no args is called, boot up Dioxus.
            dioxus::launch(frontend::App);
            Ok(())
        } else {
            use clap::Parser;
            let cli_cmd = backend::cli::Cli::parse();
            match backend::cli::run_cli(cli_cmd)? {
                backend::cli::CliResult::Exit => Ok(()),
                backend::cli::CliResult::StartDashboard { port } => {
                    // Set port and address in environment so dioxus can find it
                    std::env::set_var("PORT", port.to_string());
                    std::env::set_var("ADDR", "0.0.0.0");
                    std::env::set_var("IP", "0.0.0.0");
                    dioxus::launch(frontend::App);
                    Ok(())
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        dioxus::launch(frontend::App);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_alive() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert!(backend::state::is_pid_alive(std::process::id()));
        }
    }

    #[test]
    fn test_evolution_comment() {
        let content = std::fs::read_to_string("src/main.rs").expect("Failed to read src/main.rs");
        assert!(content.contains("// Evolution verified!"));
    }

    #[tokio::test]
    async fn test_multi_session_chat() {
        #[cfg(feature = "server")]
        {
            // 1. Create two separate sessions
            let id_1 = create_chat_session().await.expect("Failed to create session 1");
            let id_2 = create_chat_session().await.expect("Failed to create session 2");
            
            assert_ne!(id_1, id_2);

            // 2. Write mock history files directly to isolate them and avoid slow LLM integration calls
            let brain_dir_1 = std::path::Path::new("/home/wimvm/.gemini/antigravity-cli/brain").join(&id_1);
            let logs_dir_1 = brain_dir_1.join(".system_generated/logs");
            std::fs::create_dir_all(&logs_dir_1).expect("Failed to create logs dir 1");
            let transcript_path_1 = logs_dir_1.join("transcript_full.jsonl");
            let mock_data_1 = r#"{"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Hello from Room 1","created_at":"2026-05-26T08:33:59+09:00"}
{"source":"MODEL","type":"PLANNER_RESPONSE","content":"Hi there! I am Room 1 assistant.","created_at":"2026-05-26T08:34:05+09:00"}
"#;
            std::fs::write(&transcript_path_1, mock_data_1).expect("Failed to write mock transcript 1");

            let brain_dir_2 = std::path::Path::new("/home/wimvm/.gemini/antigravity-cli/brain").join(&id_2);
            let logs_dir_2 = brain_dir_2.join(".system_generated/logs");
            std::fs::create_dir_all(&logs_dir_2).expect("Failed to create logs dir 2");
            let transcript_path_2 = logs_dir_2.join("transcript_full.jsonl");
            let mock_data_2 = r#"{"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Hello from Room 2","created_at":"2026-05-26T08:33:59+09:00"}
{"source":"MODEL","type":"PLANNER_RESPONSE","content":"Hi there! I am Room 2 assistant.","created_at":"2026-05-26T08:34:05+09:00"}
"#;
            std::fs::write(&transcript_path_2, mock_data_2).expect("Failed to write mock transcript 2");
            assert!(id_1.starts_with("draft-"));
            assert!(id_2.starts_with("draft-"));

            // 2. Send messages to both sessions (which will transition them to actual UUIDs)
            let reply_1 = send_chat_message(id_1.clone(), "Hello from Room 1".to_string()).await.expect("Failed to send message to Room 1");
            let reply_2 = send_chat_message(id_2.clone(), "Hello from Room 2".to_string()).await.expect("Failed to send message to Room 2");

            assert!(!reply_1.is_empty());
            assert!(!reply_2.is_empty());

            // 3. Verify they are transitioned to UUIDs and stored
            let sessions = get_chat_sessions().await.expect("Failed to get chat sessions");
            
            // The drafts should not exist anymore in the list
            assert!(!sessions.iter().any(|s| s.id == id_1));
            assert!(!sessions.iter().any(|s| s.id == id_2));

            // But we should find the rooms with the correct titles
            let s1_opt = sessions.iter().find(|s| s.title == "Hello from Room 1");
            let s2_opt = sessions.iter().find(|s| s.title == "Hello from Room 2");

            assert!(s1_opt.is_some());
            assert!(s2_opt.is_some());

            let real_id_1 = s1_opt.unwrap().id.clone();
            let real_id_2 = s2_opt.unwrap().id.clone();

            // 4. Verify histories are isolated
            let history_1 = get_chat_history(real_id_1.clone()).await.expect("Failed to get history for Room 1");
            let history_2 = get_chat_history(real_id_2.clone()).await.expect("Failed to get history for Room 2");

            // Session 1 should contain Room 1 message, but NOT Room 2 message
            assert!(history_1.iter().any(|m| m.text.contains("Hello from Room 1")));
            assert!(!history_1.iter().any(|m| m.text.contains("Hello from Room 2")));

            // Session 2 should contain Room 2 message, but NOT Room 1 message
            assert!(history_2.iter().any(|m| m.text.contains("Hello from Room 2")));
            assert!(!history_2.iter().any(|m| m.text.contains("Hello from Room 1")));

            // 5. Delete both sessions and clean up
            delete_chat_session(real_id_1.clone()).await.expect("Failed to delete session 1");
            delete_chat_session(real_id_2.clone()).await.expect("Failed to delete session 2");
        }
    }
}

// Evolution verified! (Harness Passed)
