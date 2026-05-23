use crate::frontend::app::ProjectInfo;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use super::vault::get_base_dir;

pub fn load_state() -> HashMap<String, ProjectInfo> {
    let path = get_base_dir().join("projects.json");
    if !path.exists() {
        return HashMap::new();
    }
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_reader(file).unwrap_or_else(|_| HashMap::new())
}

pub fn save_state(state: &HashMap<String, ProjectInfo>) -> io::Result<()> {
    let path = get_base_dir().join("projects.json");
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, state)?;
    Ok(())
}

pub fn is_pid_alive(pid: u32) -> bool {
    let status_path = format!("/proc/{}/status", pid);
    if let Ok(mut file) = File::open(status_path) {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            for line in contents.lines() {
                if line.starts_with("State:") {
                    return !line.contains('Z') && !line.contains("zombie");
                }
            }
        }
    }
    false
}

pub fn check_project_status(_name: &str, info: &mut ProjectInfo) -> String {
    if info.status != "running" {
        return info.status.clone();
    }

    if is_pid_alive(info.pid) {
        return "running".to_string();
    }

    // Process is no longer running, check if report.md exists
    let report_path = Path::new(&info.path).join("report.md");
    let status = if report_path.exists() {
        "completed".to_string()
    } else {
        "failed".to_string()
    };

    info.status = status.clone();
    status
}
