#![cfg(not(target_arch = "wasm32"))]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use crate::models::ChatSession;
use super::vault::get_base_dir;

static DRAFT_MAPPINGS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn get_draft_mappings() -> &'static Mutex<HashMap<String, String>> {
    DRAFT_MAPPINGS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_draft_mapping(draft_id: String, real_id: String) {
    if let Ok(mut lock) = get_draft_mappings().lock() {
        lock.insert(draft_id, real_id);
    }
}

pub fn resolve_draft_id(id: &str) -> String {
    if let Ok(lock) = get_draft_mappings().lock() {
        if let Some(mapped) = lock.get(id) {
            return mapped.clone();
        }
    }
    id.to_string()
}

pub fn remove_draft_mapping(draft_id: &str) {
    if let Ok(mut lock) = get_draft_mappings().lock() {
        lock.remove(draft_id);
    }
}

pub fn load_chat_sessions() -> Result<Vec<ChatSession>, String> {
    let base_dir = get_base_dir();
    let sessions_path = base_dir.join("chat_sessions.json");
    if !sessions_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&sessions_path).map_err(|e| e.to_string())?;
    let mut sessions: Vec<ChatSession> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    
    let mut seen = HashSet::new();
    sessions.retain(|s| seen.insert(s.id.clone()));
    
    Ok(sessions)
}

pub fn save_chat_sessions(sessions: &[ChatSession]) -> Result<(), String> {
    let base_dir = get_base_dir();
    let sessions_path = base_dir.join("chat_sessions.json");
    let content = serde_json::to_string_pretty(sessions).map_err(|e| e.to_string())?;
    fs::write(&sessions_path, content).map_err(|e| e.to_string())
}

pub fn get_brain_sessions() -> HashSet<String> {
    let mut dirs = HashSet::new();
    if let Ok(entries) = fs::read_dir("/home/wimvm/.gemini/antigravity-cli/brain") {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !name.starts_with("draft-") {
                        dirs.insert(name);
                    }
                }
            }
        }
    }
    dirs
}

pub fn find_newest_brain_session(diff: &HashSet<String>) -> Option<String> {
    let mut newest_name = None;
    let mut newest_time = std::time::SystemTime::UNIX_EPOCH;
    for name in diff {
        let path = Path::new("/home/wimvm/.gemini/antigravity-cli/brain").join(name);
        if let Ok(meta) = fs::metadata(path) {
            if let Ok(modified) = meta.modified() {
                if modified > newest_time {
                    newest_time = modified;
                    newest_name = Some(name.clone());
                }
            }
        }
    }
    newest_name
}

pub fn uuid_v4_fallback() -> String {
    if let Ok(uuid) = fs::read_to_string("/proc/sys/kernel/random/uuid") {
        uuid.trim().to_string()
    } else {
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        format!("session-{}", ts)
    }
}

pub fn check_and_rename_session(session_id: &str, first_msg: &str) -> Result<(), String> {
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

pub fn promote_session_if_draft(session_id: &str) -> String {
    if session_id.starts_with("draft-") {
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let new_uuid = format!("session-{}", ts);
        
        if let Ok(mut sessions) = load_chat_sessions() {
            if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
                session.id = new_uuid.clone();
            }
            let _ = save_chat_sessions(&sessions);
        }
        let base_dir = get_base_dir();
        let _ = fs::write(base_dir.join("active_chat_session_id.txt"), &new_uuid);
        new_uuid
    } else {
        session_id.to_string()
    }
}

pub fn append_mock_transcript_line(session_id: &str, msg_type: &str, content: &str) -> Result<(), String> {
    let actual_id = promote_session_if_draft(session_id);

    let brain_dir = Path::new("/home/wimvm/.gemini/antigravity-cli/brain").join(&actual_id);
    let logs_dir = brain_dir.join(".system_generated/logs");
    let _ = fs::create_dir_all(&logs_dir);
    let transcript_path = logs_dir.join("transcript_full.jsonl");

    let timestamp = chrono::Local::now().to_rfc3339();
    let source = if msg_type == "USER_INPUT" { "USER_EXPLICIT" } else { "MODEL" };
    
    let line_data = serde_json::json!({
        "source": source,
        "type": msg_type,
        "content": content,
        "created_at": timestamp
    });

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&transcript_path)
        .map_err(|e| e.to_string())?;
        
    writeln!(file, "{}", line_data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_transcript_content_by_id(conversation_id: &str) -> Result<String, String> {
    let transcript_path = Path::new("/home/wimvm/.gemini/antigravity-cli/brain")
        .join(conversation_id)
        .join(".system_generated/logs/transcript_full.jsonl");
    if !transcript_path.exists() {
        return Err("Transcript file does not exist".to_string());
    }

    let file_content = fs::read_to_string(&transcript_path)
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
