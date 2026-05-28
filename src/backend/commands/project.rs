use std::fs::{self, File};
use std::io::{self, Write, Read};
use std::path::{Path, PathBuf};
use chrono::Local;

use crate::backend::vault::get_base_dir;
use crate::backend::state::{load_state, save_state, check_project_status};
use crate::backend::health::run_health_checks;
use crate::backend::cli::CliResult;
use crate::models::{ProjectInfo, HealthCheckResult};

pub struct ProjectStatusDetails {
    pub name: String,
    pub path: String,
    pub status: String,
    pub pid: u32,
    pub spawned_at: String,
    pub goal: String,
    pub report_content: Option<String>,
    pub log_suggestion: Option<PathBuf>,
}

pub struct ProjectContextDetails {
    pub name: String,
    pub path: String,
    pub status: String,
    pub playbook: Option<String>,
    pub hot_memory: Option<String>,
    pub cold_memory_size: Option<u64>,
    pub cold_memory_exists: bool,
}

/// Formats the RFC3339 timestamp of project spawning for CLI tabular display.
///
/// Converts "YYYY-MM-DDT..." to "YYYY-MM-DD HH:MM:SS" (or similar prefix).
pub fn format_spawned_at(spawned_at: &str) -> String {
    spawned_at.get(..19).unwrap_or(spawned_at).replace('T', " ")
}

/// Renders the projects list table to the specified writer.
pub fn render_projects_table<W: std::io::Write>(
    w: &mut W,
    state: &mut std::collections::HashMap<String, ProjectInfo>,
) -> io::Result<()> {
    if state.is_empty() {
        writeln!(w, "No projects registered.")?;
        return Ok(());
    }

    writeln!(
        w,
        "{:<15} | {:<6} | {:<10} | {:<20} | Path",
        "Project Name", "PID", "Status", "Spawned At"
    )?;
    writeln!(w, "{}", "-".repeat(80))?;

    for (name, info) in state.iter_mut() {
        let status = check_project_status(name, info);
        let spawned = format_spawned_at(&info.spawned_at);
        writeln!(
            w,
            "{:<15} | {:<6} | {:<10} | {:<20} | {}",
            name, info.pid, status, spawned, info.path
        )?;
    }
    Ok(())
}

/// Renders detailed project status report to the specified writer.
pub fn render_project_status<W: std::io::Write>(
    w: &mut W,
    details: &ProjectStatusDetails,
) -> io::Result<()> {
    writeln!(w, "Project: {}", details.name)?;
    writeln!(w, "Path: {}", details.path)?;
    writeln!(w, "Status: {}", details.status)?;
    writeln!(w, "PID: {}", details.pid)?;
    writeln!(w, "Spawned At: {}", details.spawned_at)?;
    writeln!(w, "Goal: {}", details.goal)?;

    if let Some(ref report_content) = details.report_content {
        writeln!(w, "\n--- [report.md Content] ---")?;
        writeln!(w, "{}", report_content)?;
    } else {
        writeln!(w, "\nReport file not found at: {}/report.md", details.path)?;
        if details.status == "failed" {
            if let Some(ref log_path) = details.log_suggestion {
                writeln!(w, "Note: Project failed. Check logs for details: {}", log_path.display())?;
            }
        }
    }
    Ok(())
}

/// Renders detailed project context (playbook, hot memory, cold memory size) to the specified writer.
pub fn render_project_context<W: std::io::Write>(
    w: &mut W,
    details: &ProjectContextDetails,
) -> io::Result<()> {
    writeln!(w, "Project: {}", details.name)?;
    writeln!(w, "Path: {}", details.path)?;
    writeln!(w, "Status: {}", details.status)?;

    if let Some(ref playbook) = details.playbook {
        writeln!(w, "\n--- [AGENTS.md Content (Project Playbook)] ---")?;
        writeln!(w, "{}", playbook)?;
    } else {
        writeln!(w, "\nNo AGENTS.md (Project Playbook) file exists yet in the project directory.")?;
    }

    if let Some(ref hot_memory) = details.hot_memory {
        writeln!(w, "\n--- [context.md Content (Hot Memory)] ---")?;
        writeln!(w, "{}", hot_memory)?;
    } else {
        writeln!(w, "\nNo context.md (Hot Memory) file exists yet in the project directory.")?;
    }

    writeln!(w, "\n--- [context_history.md Status (Cold Memory)] ---")?;
    if details.cold_memory_exists {
        if let Some(size) = details.cold_memory_size {
            writeln!(w, "Archive file exists. Size: {} bytes", size)?;
        } else {
            writeln!(w, "Archive file exists.")?;
        }
    } else {
        writeln!(w, "No context_history.md (Cold Memory) file exists yet.")?;
    }
    Ok(())
}

/// Renders health check results to the specified writer.
pub fn render_health_checks<W: std::io::Write>(
    w: &mut W,
    results: &[HealthCheckResult],
) -> io::Result<()> {
    writeln!(w, "{:<25} | {:<8} | {:<20} | Message", "Target", "Status", "Checked At")?;
    writeln!(w, "{}", "-".repeat(90))?;
    for r in results {
        let status = if r.healthy { "✅ OK" } else { "❌ FAIL" };
        writeln!(w, "{:<25} | {:<8} | {:<20} | {}", r.target, status, r.checked_at, r.message)?;
    }
    let healthy_count = results.iter().filter(|r| r.healthy).count();
    let failed_count = results.len() - healthy_count;
    writeln!(w, "\nSummary: {} passed, {} failed.", healthy_count, failed_count)?;
    Ok(())
}

/// Parses the "Lessons Learned" section out of report.md.
///
/// Looks for headings matching "Lessons Learned", "교훈", or "지식".
pub fn parse_lessons_learned(content: &str) -> String {
    // Parse Lessons Learned / 교훈 / 지식 Section
    let mut lessons_content = String::new();
    let mut in_lessons = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let header_title = trimmed.trim_start_matches('#').trim().to_lowercase();
            if header_title.contains("lessons learned") || header_title == "교훈" || header_title == "지식" {
                in_lessons = true;
                continue;
            } else {
                in_lessons = false;
            }
        }
        
        if in_lessons {
            lessons_content.push_str(line);
            lessons_content.push('\n');
        }
    }
    lessons_content.trim().to_string()
}



// Lists all registered projects in the global state projects.json
// by writing a formatted table layout to stdout.
pub fn execute_list() -> io::Result<CliResult> {
    let mut state = load_state();
    render_projects_table(&mut io::stdout(), &mut state)?;
    save_state(&state)?;
    Ok(CliResult::Exit)
}

pub fn execute_status(name: String) -> io::Result<CliResult> {
    let mut state = load_state();
    let base_dir = get_base_dir();
    
    let info = state.get_mut(&name).ok_or_else(|| {
        eprintln!("Error: Project '{}' not found.", name);
        io::Error::new(io::ErrorKind::NotFound, format!("Project '{}' not found.", name))
    })?;
    
    let status = check_project_status(&name, info);
    let path_str = info.path.clone();
    let report_path = Path::new(&path_str).join("report.md");
    
    let report_content = if report_path.exists() {
        let mut content = String::new();
        File::open(report_path)?.read_to_string(&mut content)?;
        Some(content)
    } else {
        None
    };

    let details = ProjectStatusDetails {
        name: name.clone(),
        path: path_str,
        status: status.clone(),
        pid: info.pid,
        spawned_at: info.spawned_at.clone(),
        goal: info.goal.clone(),
        report_content,
        log_suggestion: Some(base_dir.join("logs").join(format!("{}.log", name))),
    };
    
    save_state(&state)?;
    render_project_status(&mut io::stdout(), &details)?;
    Ok(CliResult::Exit)
}

pub fn execute_get_context(name: String) -> io::Result<CliResult> {
    let mut state = load_state();
    
    let info = state.get_mut(&name).ok_or_else(|| {
        eprintln!("Error: Project '{}' not found.", name);
        io::Error::new(io::ErrorKind::NotFound, format!("Project '{}' not found.", name))
    })?;
    
    let status = check_project_status(&name, info);
    let path_str = info.path.clone();
    
    let agents_path = Path::new(&path_str).join("AGENTS.md");
    let playbook = if agents_path.exists() {
        let mut content = String::new();
        File::open(&agents_path).and_then(|mut f| f.read_to_string(&mut content)).ok().map(|_| content)
    } else {
        None
    };

    let context_path = Path::new(&path_str).join("context.md");
    let hot_memory = if context_path.exists() {
        let mut content = String::new();
        File::open(context_path)?.read_to_string(&mut content)?;
        Some(content)
    } else {
        None
    };

    let history_path = Path::new(&path_str).join("context_history.md");
    let (cold_memory_exists, cold_memory_size) = if history_path.exists() {
        let size = fs::metadata(&history_path).ok().map(|m| m.len());
        (true, size)
    } else {
        (false, None)
    };

    let details = ProjectContextDetails {
        name,
        path: path_str,
        status,
        playbook,
        hot_memory,
        cold_memory_size,
        cold_memory_exists,
    };
    
    save_state(&state)?;
    render_project_context(&mut io::stdout(), &details)?;
    Ok(CliResult::Exit)
}

// Consolidates the completed subtask report (report.md) into the project's history log.
// It extracts lessons learned, appends the formatted report to history logs,
// initializes active context (context.md) if missing, and removes the temporary report.md.
pub fn execute_consolidate(name: String) -> io::Result<CliResult> {
    let mut state = load_state();
    let base_dir = get_base_dir();
    
    let (path_str, spawned_at) = {
        let info = state.get_mut(&name).ok_or_else(|| {
            eprintln!("Error: Project '{}' not found.", name);
            io::Error::new(io::ErrorKind::NotFound, format!("Project '{}' not found.", name))
        })?;

        let status = check_project_status(&name, info);
        if status == "running" {
            eprintln!("Error: Cannot consolidate project '{}' while it is still running.", name);
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("Cannot consolidate project '{}' while it is still running.", name)
            ));
        }

        info.status = "completed".to_string();

        (info.path.clone(), info.spawned_at.clone())
    };

    let report_path = Path::new(&path_str).join("report.md");
    if !report_path.exists() {
        eprintln!("Error: report.md not found at {}. Cannot consolidate.", report_path.display());
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("report.md not found at {}.", report_path.display())
        ));
    }

    let mut report_content = String::new();
    File::open(&report_path)?.read_to_string(&mut report_content)?;

    let lessons_trimmed = parse_lessons_learned(&report_content);
    if !lessons_trimmed.is_empty() {
        let vault_dir = base_dir.join("memory/vault");
        fs::create_dir_all(&vault_dir)?;
        let lessons_file_path = vault_dir.join(format!("{}_lessons.md", name));
        let mut file = File::create(&lessons_file_path)?;
        writeln!(
            file,
            "# 🧠 Lessons Learned from Project '{}'\n\n*Saved on: {}*\n\n{}",
            name,
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            lessons_trimmed
        )?;
        println!("Extracted lessons learned and saved to {}", lessons_file_path.display());

        // Log notification
        if let Ok(mut log_file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(base_dir.join("notifications.log"))
        {
            let _ = writeln!(
                log_file,
                "[{}] INFO: Extracted lessons learned from '{}' and saved to vault.",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                name
            );
        }
    }

    let history_path = Path::new(&path_str).join("context_history.md");
    let mut history_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_path)?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(
        history_file,
        "\n\n# 📅 History log from {} (Spawned at {})\n\n{}",
        timestamp, spawned_at, report_content
    )?;

    // Fallback: If context.md (Hot Memory) does not exist, initialize it with report contents
    let context_path = Path::new(&path_str).join("context.md");
    if !context_path.exists() {
        let mut context_file = File::create(&context_path)?;
        writeln!(
            context_file,
            "# Active Project Context\n\n\
             ## Project Name: {}\n\
             ## Status: Completed (Initialized from fallback at {})\n\n\
             ### Last Task Summary\n\
             {}",
            name, timestamp, report_content
        )?;
        println!("Hot Memory: Initialized context.md from report fallback.");
    }

    // Clean up report.md as it is consolidated into context_history.md
    if let Err(e) = fs::remove_file(&report_path) {
        eprintln!("Warning: Failed to remove report.md at {}: {}", report_path.display(), e);
    } else {
        println!("Cleaned up report.md after consolidation.");
    }



    save_state(&state)?;

    println!("Successfully consolidated report.md into context_history.md for project '{}'.", name);
    println!("Updated status to 'completed' in projects.json.");
    Ok(CliResult::Exit)
}

// Executes health checks on all registered projects to verify that
// their background processes are still running and directories exist.
pub fn execute_health_check() -> io::Result<CliResult> {
    println!("Running health checks on all registered targets...\n");
    let results = run_health_checks().map_err(|e| {
        eprintln!("Health check error: {}", e);
        io::Error::other(format!("Health check error: {}", e))
    })?;
    
    render_health_checks(&mut io::stdout(), &results)?;
    Ok(CliResult::Exit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::env;

    #[test]
    fn test_format_spawned_at() {
        assert_eq!(format_spawned_at("2026-05-28T22:13:58Z"), "2026-05-28 22:13:58");
        assert_eq!(format_spawned_at("short"), "short");
    }

    #[test]
    fn test_parse_lessons_learned() {
        let content = "\
# My Report
This is a report.

## Lessons Learned
- Don't do that.
- Do this instead.

# Next Steps
More info.
";
        let parsed = parse_lessons_learned(content);
        assert!(parsed.contains("- Don't do that."));
        assert!(parsed.contains("- Do this instead."));
        assert!(!parsed.contains("More info."));

        let content_kr = "\
# 보고서
## 교훈
- 이렇게 하시오.
### 지식
- 저렇게 하시오.
";
        let parsed_kr = parse_lessons_learned(content_kr);
        assert!(parsed_kr.contains("- 이렇게 하시오."));
    }



    #[test]
    fn test_render_projects_table() {
        let mut state = HashMap::new();
        state.insert(
            "test_proj".to_string(),
            ProjectInfo {
                path: "/path/to/test".to_string(),
                pid: 5678,
                spawned_at: "2026-05-28T12:00:00Z".to_string(),
                status: "running".to_string(),
                goal: "make test".to_string(),
            },
        );

        let mut buf = Vec::new();
        render_projects_table(&mut buf, &mut state).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("test_proj"));
        assert!(output.contains("5678"));
    }

    #[test]
    fn test_render_project_status() {
        let details = ProjectStatusDetails {
            name: "status_proj".to_string(),
            path: "/path/to/status".to_string(),
            status: "failed".to_string(),
            pid: 999,
            spawned_at: "2026-05-28 12:00:00".to_string(),
            goal: "status check".to_string(),
            report_content: Some("All tasks resolved".to_string()),
            log_suggestion: Some(PathBuf::from("/path/to/logs/status_proj.log")),
        };

        let mut buf = Vec::new();
        render_project_status(&mut buf, &details).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Project: status_proj"));
        assert!(output.contains("PID: 999"));
        assert!(output.contains("All tasks resolved"));
    }

    #[test]
    fn test_execute_status_not_found() {
        let _lock = crate::backend::vault::TEST_MUTEX.lock().unwrap();
        let test_home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test_home_project_status_not_found");
        let _ = fs::remove_dir_all(&test_home);
        fs::create_dir_all(test_home.join(".agy_orchestrator")).unwrap();

        env::set_var("HOME", &test_home);

        let res = execute_status("non_existent".to_string());
        assert!(res.is_err());
        if let Err(e) = res {
            assert_eq!(e.kind(), io::ErrorKind::NotFound);
        }

        let _ = fs::remove_dir_all(&test_home);
    }
}
