use std::fs::{self, File};
use std::io::{self, Write, Read};
use std::path::Path;
use chrono::Local;

use crate::backend::vault::get_base_dir;
use crate::backend::state::{load_state, save_state, check_project_status};
use crate::backend::health::run_health_checks;
use crate::backend::cli::CliResult;

pub fn execute_list() -> io::Result<CliResult> {
    let mut state = load_state();
    if state.is_empty() {
        println!("No projects registered.");
        return Ok(CliResult::Exit);
    }

    println!(
        "{:<15} | {:<6} | {:<10} | {:<20} | Path",
        "Project Name", "PID", "Status", "Spawned At"
    );
    println!("{}", "-".repeat(80));

    for (name, info) in state.iter_mut() {
        let status = check_project_status(name, info);
        let spawned = info.spawned_at.get(..19).unwrap_or(&info.spawned_at).replace('T', " ");
        println!(
            "{:<15} | {:<6} | {:<10} | {:<20} | {}",
            name, info.pid, status, spawned, info.path
        );
    }
    save_state(&state)?;
    Ok(CliResult::Exit)
}

pub fn execute_status(name: String) -> io::Result<CliResult> {
    let mut state = load_state();
    let base_dir = get_base_dir();
    
    let (status, path_str, pid, spawned_at, goal) = {
        let info = match state.get_mut(&name) {
            Some(i) => i,
            None => {
                eprintln!("Error: Project '{}' not found.", name);
                std::process::exit(1);
            }
        };
        let status = check_project_status(&name, info);
        (status, info.path.clone(), info.pid, info.spawned_at.clone(), info.goal.clone())
    };
    
    save_state(&state)?;

    println!("Project: {}", name);
    println!("Path: {}", path_str);
    println!("Status: {}", status);
    println!("PID: {}", pid);
    println!("Spawned At: {}", spawned_at);
    println!("Goal: {}", goal);

    let report_path = Path::new(&path_str).join("report.md");
    if report_path.exists() {
        println!("\n--- [report.md Content] ---");
        let mut report_content = String::new();
        File::open(report_path)?.read_to_string(&mut report_content)?;
        println!("{}", report_content);
    } else {
        println!("\nReport file not found at: {}", report_path.display());
        if status == "failed" {
            println!("Note: Project failed. Check logs for details: {}", base_dir.join("logs").join(format!("{}.log", name)).display());
        }
    }
    Ok(CliResult::Exit)
}

pub fn execute_get_context(name: String) -> io::Result<CliResult> {
    let mut state = load_state();
    
    let (status, path_str) = {
        let info = match state.get_mut(&name) {
            Some(i) => i,
            None => {
                eprintln!("Error: Project '{}' not found.", name);
                std::process::exit(1);
            }
        };
        let status = check_project_status(&name, info);
        (status, info.path.clone())
    };
    
    save_state(&state)?;

    println!("Project: {}", name);
    println!("Path: {}", path_str);
    println!("Status: {}", status);

    let agents_path = Path::new(&path_str).join("AGENTS.md");
    if agents_path.exists() {
        let mut agents_content = String::new();
        if File::open(&agents_path).and_then(|mut f| f.read_to_string(&mut agents_content)).is_ok() {
            println!("\n--- [AGENTS.md Content (Project Playbook)] ---");
            println!("{}", agents_content);
        }
    } else {
        println!("\nNo AGENTS.md (Project Playbook) file exists yet in the project directory.");
    }

    let context_path = Path::new(&path_str).join("context.md");
    if context_path.exists() {
        let mut context_content = String::new();
        File::open(context_path)?.read_to_string(&mut context_content)?;
        println!("\n--- [context.md Content (Hot Memory)] ---");
        println!("{}", context_content);
    } else {
        println!("\nNo context.md (Hot Memory) file exists yet in the project directory.");
    }

    let history_path = Path::new(&path_str).join("context_history.md");
    if history_path.exists() {
        println!("\n--- [context_history.md Status (Cold Memory)] ---");
        if let Ok(metadata) = fs::metadata(&history_path) {
            println!("Archive file exists. Size: {} bytes", metadata.len());
        } else {
            println!("Archive file exists.");
        }
    } else {
        println!("\nNo context_history.md (Cold Memory) file exists yet.");
    }
    Ok(CliResult::Exit)
}

pub fn execute_consolidate(name: String) -> io::Result<CliResult> {
    let mut state = load_state();
    let base_dir = get_base_dir();
    
    let (path_str, spawned_at) = {
        let info = match state.get_mut(&name) {
            Some(i) => i,
            None => {
                eprintln!("Error: Project '{}' not found.", name);
                std::process::exit(1);
            }
        };

        let status = check_project_status(&name, info);
        if status == "running" {
            eprintln!("Error: Cannot consolidate project '{}' while it is still running.", name);
            std::process::exit(1);
        }

        info.status = "completed".to_string();
        (info.path.clone(), info.spawned_at.clone())
    };

    let report_path = Path::new(&path_str).join("report.md");
    if !report_path.exists() {
        eprintln!("Error: report.md not found at {}. Cannot consolidate.", report_path.display());
        std::process::exit(1);
    }

    let mut report_content = String::new();
    File::open(&report_path)?.read_to_string(&mut report_content)?;

    // Parse Lessons Learned / 교훈 / 지식 Section
    let lines = report_content.lines();
    let mut lessons_content = String::new();
    let mut in_lessons = false;
    
    for line in lines {
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

    let lessons_trimmed = lessons_content.trim();
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

    // Sub-agent report feedback loop (Child -> Parent context feedback)
    if name.contains("_sub_") {
        if let Some(sub_idx) = name.rfind("_sub_") {
            let parent_name = &name[..sub_idx];
            let subtask_name = &name[sub_idx + 5..];
            
            if let Some(parent_info) = state.get(parent_name) {
                let parent_context_path = Path::new(&parent_info.path).join("context.md");
                if let Ok(mut parent_context_file) = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&parent_context_path)
                {
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                    let _ = writeln!(
                        parent_context_file,
                        "\n\n==================================================\n\
                         # 📢 Subtask Completed: '{}' at {}\n\
                         - Sub-agent Name: {}\n\
                         - Completed Report:\n\n\
                         {}\n\
                         ==================================================\n",
                        subtask_name, timestamp, name, report_content.trim()
                    );
                    println!("Feedback Loop: Auto-injected subtask completed report into parent '{}' context.md", parent_name);
                }
            }
        }
    }

    save_state(&state)?;

    println!("Successfully consolidated report.md into context_history.md for project '{}'.", name);
    println!("Updated status to 'completed' in projects.json.");
    Ok(CliResult::Exit)
}

pub fn execute_health_check() -> io::Result<CliResult> {
    println!("Running health checks on all registered targets...\n");
    match run_health_checks() {
        Ok(results) => {
            println!("{:<25} | {:<8} | {:<20} | Message", "Target", "Status", "Checked At");
            println!("{}", "-".repeat(90));
            for r in &results {
                let status = if r.healthy { "✅ OK" } else { "❌ FAIL" };
                println!("{:<25} | {:<8} | {:<20} | {}", r.target, status, r.checked_at, r.message);
            }
            let healthy_count = results.iter().filter(|r| r.healthy).count();
            let failed_count = results.len() - healthy_count;
            println!("\nSummary: {} passed, {} failed.", healthy_count, failed_count);
        }
        Err(e) => {
            eprintln!("Health check error: {}", e);
            std::process::exit(1);
        }
    }
    Ok(CliResult::Exit)
}
