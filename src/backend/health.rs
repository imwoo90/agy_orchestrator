use crate::frontend::app::{HealthCheckResult, Issue};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use chrono::Local;
use super::vault::get_base_dir;
use super::state::load_state;
use super::issue::{load_issues, save_issues};

pub fn find_workspace_root() -> io::Result<PathBuf> {
    let mut current_dir = std::env::current_exe()?;
    while current_dir.pop() {
        if current_dir.join("Cargo.toml").exists() {
            return Ok(current_dir);
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "Workspace Cargo.toml not found"))
}

pub fn save_health_results(results: &[HealthCheckResult]) -> io::Result<()> {
    let path = get_base_dir().join("health.json");
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, results)?;
    Ok(())
}

pub fn run_health_checks() -> io::Result<Vec<HealthCheckResult>> {
    let base_dir = get_base_dir();
    let notifications_path = base_dir.join("notifications.log");
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut results: Vec<HealthCheckResult> = Vec::new();

    // 1. Orchestrator self-check
    if let Ok(workspace_root) = find_workspace_root() {
        let check_status = Command::new("cargo")
            .arg("check")
            .current_dir(&workspace_root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        let (healthy, msg) = match check_status {
            Ok(s) if s.success() => (true, "cargo check passed".to_string()),
            Ok(s) => (false, format!("cargo check failed (exit code: {:?})", s.code())),
            Err(e) => (false, format!("cargo check error: {}", e)),
        };

        results.push(HealthCheckResult {
            target: "orchestrator".to_string(),
            healthy,
            message: msg.clone(),
            checked_at: timestamp.clone(),
        });

        if let Ok(mut log_file) = fs::OpenOptions::new().create(true).append(true).open(&notifications_path) {
            let _ = writeln!(log_file, "[{}] INFO: Health check {} for orchestrator.", timestamp, if healthy { "PASSED" } else { "FAILED" });
        }

        // Auto-register issue if orchestrator health check fails
        if !healthy {
            let mut issues = load_issues();
            let already_exists = issues.iter().any(|i| {
                i.title.starts_with("[Auto-Health] orchestrator") && (i.status == "open" || i.status == "in-progress")
            });
            if !already_exists {
                let next_id = issues.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                issues.push(Issue {
                    id: next_id,
                    title: "[Auto-Health] orchestrator build failure".to_string(),
                    body: format!("Automated health check detected build failure in the orchestrator codebase.\nError: {}\nPlease investigate and fix the compilation errors.", msg),
                    status: "open".to_string(),
                    created_at: Local::now().to_rfc3339(),
                    resolved_at: None,
                });
                let _ = save_issues(&issues);
                if let Ok(mut log_file) = fs::OpenOptions::new().create(true).append(true).open(&notifications_path) {
                    let _ = writeln!(log_file, "[{}] INFO: Auto-registered health issue for orchestrator.", timestamp);
                }
            }
        }
    }

    // 2. Check registered projects
    let state = load_state();
    for (name, info) in state.iter() {
        let project_path = Path::new(&info.path);
        let (healthy, msg) = if project_path.join("Cargo.toml").exists() {
            let check_status = Command::new("cargo")
                .arg("check")
                .current_dir(project_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            match check_status {
                Ok(s) if s.success() => (true, "cargo check passed".to_string()),
                Ok(s) => (false, format!("cargo check failed (exit code: {:?})", s.code())),
                Err(e) => (false, format!("cargo check error: {}", e)),
            }
        } else if project_path.join("package.json").exists() {
            let check_status = Command::new("npm")
                .arg("test")
                .arg("--if-present")
                .current_dir(project_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            match check_status {
                Ok(s) if s.success() => (true, "npm test passed".to_string()),
                Ok(s) => (false, format!("npm test failed (exit code: {:?})", s.code())),
                Err(e) => (false, format!("npm test error: {}", e)),
            }
        } else {
            (true, "no build system detected (skipped)".to_string())
        };

        results.push(HealthCheckResult {
            target: name.clone(),
            healthy,
            message: msg.clone(),
            checked_at: timestamp.clone(),
        });

        if let Ok(mut log_file) = fs::OpenOptions::new().create(true).append(true).open(&notifications_path) {
            let _ = writeln!(log_file, "[{}] INFO: Health check {} for project '{}'.", timestamp, if healthy { "PASSED" } else { "FAILED" }, name);
        }

        // Auto-register issue if project health check fails
        if !healthy {
            let mut issues = load_issues();
            let prefix = format!("[Auto-Health] {}", name);
            let already_exists = issues.iter().any(|i| {
                i.title.starts_with(&prefix) && (i.status == "open" || i.status == "in-progress")
            });
            if !already_exists {
                let next_id = issues.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                issues.push(Issue {
                    id: next_id,
                    title: format!("[Auto-Health] {} build failure", name),
                    body: format!("Automated health check detected build failure in project '{}'.\nPath: {}\nError: {}\nPlease investigate and fix the issue.", name, info.path, msg),
                    status: "open".to_string(),
                    created_at: Local::now().to_rfc3339(),
                    resolved_at: None,
                });
                let _ = save_issues(&issues);
                if let Ok(mut log_file) = fs::OpenOptions::new().create(true).append(true).open(&notifications_path) {
                    let _ = writeln!(log_file, "[{}] INFO: Auto-registered health issue for project '{}'.", timestamp, name);
                }
            }
        }
    }

    let _ = save_health_results(&results);
    Ok(results)
}
