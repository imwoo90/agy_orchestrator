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
    // 1. Try finding from current executable path
    if let Ok(mut current_dir) = std::env::current_exe() {
        while current_dir.pop() {
            if current_dir.join("Cargo.toml").exists() {
                return Ok(current_dir);
            }
        }
    }
    
    // 2. Fallback: Try finding from current working directory
    if let Ok(mut current_working_dir) = std::env::current_dir() {
        loop {
            if current_working_dir.join("Cargo.toml").exists() {
                return Ok(current_working_dir);
            }
            if !current_working_dir.pop() {
                break;
            }
        }
    }

    // 3. Fallback: Try finding from projects.json state
    let projects = load_state();
    if let Some(info) = projects.get("agy_orchestrator") {
        let p = PathBuf::from(&info.path);
        if p.join("Cargo.toml").exists() {
            return Ok(p);
        }
    }
    for info in projects.values() {
        let p = PathBuf::from(&info.path);
        if p.join("Cargo.toml").exists() {
            return Ok(p);
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

    let state = load_state();
    
    // Find all canonicalized paths that have currently running tasks/projects
    let running_paths: std::collections::HashSet<PathBuf> = state
        .values()
        .filter(|info| info.status == "running")
        .map(|info| {
            let path = Path::new(&info.path);
            fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        })
        .collect();

    // Cache of health check results per canonicalized path to avoid duplicate checks
    let mut checked_paths: std::collections::HashMap<PathBuf, (bool, String)> = std::collections::HashMap::new();

    // 1. Orchestrator self-check
    if let Ok(workspace_root) = find_workspace_root() {
        let canonical_root = fs::canonicalize(&workspace_root).unwrap_or_else(|_| workspace_root.clone());
        
        let (healthy, msg) = if running_paths.contains(&canonical_root) {
            (true, "skipped (workspace is active)".to_string())
        } else {
            let check_status = Command::new("cargo")
                .arg("check")
                .current_dir(&workspace_root)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            let res = match check_status {
                Ok(s) if s.success() => (true, "cargo check passed".to_string()),
                Ok(s) => (false, format!("cargo check failed (exit code: {:?})", s.code())),
                Err(e) => (false, format!("cargo check error: {}", e)),
            };
            
            checked_paths.insert(canonical_root, res.clone());
            res
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
    for (name, info) in state.iter() {
        let raw_project_path = Path::new(&info.path);
        let project_path = fs::canonicalize(raw_project_path).unwrap_or_else(|_| raw_project_path.to_path_buf());

        let (healthy, msg) = if running_paths.contains(&project_path) {
            (true, "skipped (project is running)".to_string())
        } else if let Some(cached_res) = checked_paths.get(&project_path) {
            cached_res.clone()
        } else {
            let res = if raw_project_path.join("Cargo.toml").exists() {
                let check_status = Command::new("cargo")
                    .arg("check")
                    .current_dir(raw_project_path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                match check_status {
                    Ok(s) if s.success() => (true, "cargo check passed".to_string()),
                    Ok(s) => (false, format!("cargo check failed (exit code: {:?})", s.code())),
                    Err(e) => (false, format!("cargo check error: {}", e)),
                }
            } else if raw_project_path.join("package.json").exists() {
                let check_status = Command::new("npm")
                    .arg("test")
                    .arg("--if-present")
                    .current_dir(raw_project_path)
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
            
            checked_paths.insert(project_path, res.clone());
            res
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::frontend::app::ProjectInfo;

    #[test]
    fn test_health_checks_skips_running_projects() {
        // Setup a temporary directory for HOME
        let temp_dir = std::env::temp_dir().join(format!(
            "agy_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_dir);

        // Bootstrap directories
        crate::backend::vault::bootstrap_if_needed().unwrap();

        // Let's populate projects.json
        let base_dir = crate::backend::vault::get_base_dir();
        let projects_path = base_dir.join("projects.json");

        let mut state = std::collections::HashMap::new();
        // A running project (should be skipped)
        state.insert(
            "running_project".to_string(),
            ProjectInfo {
                path: temp_dir.to_string_lossy().to_string(),
                goal: "do something".to_string(),
                pid: 12345,
                status: "running".to_string(),
                spawned_at: chrono::Local::now().to_rfc3339(),
            },
        );
        // A completed project sharing the SAME path (should also be skipped!)
        state.insert(
            "completed_shared_project".to_string(),
            ProjectInfo {
                path: temp_dir.to_string_lossy().to_string(),
                goal: "do something else".to_string(),
                pid: 12346,
                status: "completed".to_string(),
                spawned_at: chrono::Local::now().to_rfc3339(),
            },
        );

        let data = serde_json::to_string_pretty(&state).unwrap();
        fs::write(&projects_path, data).unwrap();

        // Run health checks
        let results = run_health_checks().unwrap();

        // Restore HOME
        if let Some(h) = old_home {
            std::env::set_var("HOME", h);
        } else {
            std::env::remove_var("HOME");
        }

        // Clean up temp dir
        let _ = fs::remove_dir_all(&temp_dir);

        // Verify results
        assert!(!results.is_empty());
        
        let running_res = results.iter().find(|r| r.target == "running_project").unwrap();
        assert!(running_res.healthy);
        assert_eq!(running_res.message, "skipped (project is running)");

        let completed_res = results.iter().find(|r| r.target == "completed_shared_project").unwrap();
        assert!(completed_res.healthy);
        assert_eq!(completed_res.message, "skipped (project is running)");
    }
}
