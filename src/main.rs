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
            .map_err(|e| ServerFnError::new(e))
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

#[server]
async fn trigger_remote_upgrade(download_url: String) -> Result<(), ServerFnError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        backend::upgrade::run_remote_upgrade(&download_url).map_err(|e| ServerFnError::new(e.to_string()))?;
        
        // Spawn a background task to restart the dashboard after a short delay
        tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            
            if let Ok(current_exe) = std::env::current_exe() {
                let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
                let mut cmd = std::process::Command::new(&current_exe);
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
            }
            
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
    #[cfg(target_arch = "wasm32")]
    {
        Err(ServerFnError::new("Only available on server"))
    }
}

// Entrypoint
fn main() -> std::io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let has_args = std::env::args().len() > 1;
        let is_dioxus_env = std::env::var("PORT").is_ok() || std::env::var("ADDR").is_ok() || std::env::var("DIOXUS_ACTIVE").is_ok();

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
                    // Set port in environment so dioxus can find it
                    std::env::set_var("PORT", port.to_string());
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
}

// Evolution verified! (Harness Passed)
