use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use chrono::Local;

use crate::models::ProjectInfo;
use crate::backend::vault::get_base_dir;
use crate::backend::state::{load_state, save_state, check_project_status};
use crate::backend::daemon::{is_daemon_running, get_daemon_pid};
use crate::backend::cli::CliResult;

pub fn execute_compress(name: String) -> io::Result<CliResult> {
    let base_dir = get_base_dir();
    let log_file_path = base_dir.join("logs").join(format!("{}.log", name));
    if !log_file_path.exists() {
        eprintln!("Error: Log file for project '{}' not found.", name);
        std::process::exit(1);
    }

    let initial_size = fs::metadata(&log_file_path)?.len();
    compress_log_file(&log_file_path)?;
    let final_size = fs::metadata(&log_file_path)?.len();

    println!(
        "Successfully compressed log file for '{}'. Size reduced from {} to {} bytes.",
        name, initial_size, final_size
    );
    Ok(CliResult::Exit)
}

pub fn execute_search_history(name: String, query: String) -> io::Result<CliResult> {
    let state = load_state();
    let path_str = match state.get(&name) {
        Some(info) => info.path.clone(),
        None => {
            eprintln!("Error: Project '{}' not found.", name);
            std::process::exit(1);
        }
    };

    let history_path = std::path::Path::new(&path_str).join("context_history.md");
    if !history_path.exists() {
        println!("No context_history.md (Cold Memory) file exists yet for project '{}'.", name);
        return Ok(CliResult::Exit);
    }

    let content = fs::read_to_string(&history_path)?;
    let query_lower = query.to_lowercase();
    
    let mut blocks = Vec::new();
    let mut current_block = String::new();
    
    for line in content.lines() {
        if line.trim().starts_with("# 📅 History log from") || line.trim().starts_with("# History log") {
            if !current_block.trim().is_empty() {
                blocks.push(current_block.clone());
            }
            current_block = line.to_string() + "\n";
        } else {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }
    if !current_block.trim().is_empty() {
        blocks.push(current_block);
    }

    let mut matched_blocks = Vec::new();
    for block in blocks {
        if block.to_lowercase().contains(&query_lower) {
            matched_blocks.push(block);
        }
    }

    if matched_blocks.is_empty() {
        println!("No historical logs matched the query: '{}'", query);
    } else {
        println!("Found {} matching historical log blocks (showing up to 3 latest matches):\n", matched_blocks.len());
        let start_idx = matched_blocks.len().saturating_sub(3);
        for (idx, block) in matched_blocks.iter().enumerate().skip(start_idx) {
            println!("--- Match {} ---", idx + 1);
            println!("{}", block.trim());
            println!("----------------\n");
        }
    }
    Ok(CliResult::Exit)
}

pub fn execute_delegate(parent: String, subtask: String, goal: String) -> io::Result<CliResult> {
    let mut state = load_state();
    let base_dir = get_base_dir();
    let parent_info = match state.get(&parent) {
        Some(info) => info.clone(),
        None => {
            eprintln!("Error: Parent project '{}' not found in projects.json.", parent);
            std::process::exit(1);
        }
    };

    let child_name = format!("{}_sub_{}", parent, subtask);
    
    if let Some(info) = state.get_mut(&child_name) {
        if check_project_status(&child_name, info) == "running" {
            eprintln!("Error: Sub-agent '{}' is already running with PID {}.", child_name, info.pid);
            std::process::exit(1);
        }
    }

    let project_path_str = parent_info.path.clone();
    
    // AGENTS.md inheritance
    let parent_agents_path = Path::new(&project_path_str).join("AGENTS.md");
    let mut agents_inject = String::new();
    if parent_agents_path.exists() {
        if let Ok(content) = fs::read_to_string(&parent_agents_path) {
            agents_inject = format!(
                "\n\n==================================================\n\
                 [PARENT PROJECT PLAYBOOK - AGENTS.MD]\n\
                 (This subtask belongs to parent project '{}'. Follow these guidelines!)\n\n\
                 {}\n\
                 ==================================================\n\n",
                parent, content.trim()
            );
        }
    }

    // Parent context.md JIT inject
    let parent_context_path = Path::new(&project_path_str).join("context.md");
    let mut parent_context_inject = String::new();
    if parent_context_path.exists() {
        if let Ok(content) = fs::read_to_string(&parent_context_path) {
            parent_context_inject = format!(
                "\n\n==================================================\n\
                 [PARENT ACTIVE CONTEXT - HOT MEMORY]\n\
                 (Current parent project state for your reference):\n\n\
                 {}\n\
                 ==================================================\n\n",
                content.trim()
            );
        }
    }

    let report_instruction = format!(
        "\n\n==================================================\n\
         SYSTEM INSTRUCTIONS FOR COMPLETION:\n\
         Once you complete this subtask, you MUST generate a 'report.md' file in the root of the project directory ({})\n\
         This report must contain:\n\
         1. A summary of completed tasks.\n\
         2. Crucial design/architectural choices made.\n\
         3. Minor choices resolved autonomously.\n\
         4. A section 'CRITICAL ITEMS FOR REVIEW' containing only items that require manual review or escalation. If none, clearly state 'None'.\n\n\
         Ensure this report is written before you finish. The orchestrator will automatically consolidate this subtask report back into the parent project context.",
        project_path_str
    );

    // JIT Skills Catalog Auto-Injection
    let skills_dir = base_dir.join("memory/skills");
    let mut skills_inject = String::new();
    if skills_dir.exists() {
        let mut matched_skills = Vec::new();
        let goal_lower = goal.to_lowercase();
        let keywords: std::collections::HashSet<String> = goal_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() >= 4)
            .map(|s| s.to_string())
            .collect();

        if let Ok(entries) = fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut skill_name = String::new();
                        let mut skill_desc = String::new();
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("name:") {
                                skill_name = trimmed.trim_start_matches("name:").trim().to_string();
                            } else if trimmed.starts_with("description:") {
                                skill_desc = trimmed.trim_start_matches("description:").trim().to_string();
                            }
                        }
                        if !skill_name.is_empty() {
                            let skill_name_lower = skill_name.to_lowercase();
                            let skill_desc_lower = skill_desc.to_lowercase();
                            let is_match = keywords.iter().any(|kw| {
                                kw != "this" && kw != "that" && kw != "with" && kw != "from" &&
                                (skill_name_lower.contains(kw) || skill_desc_lower.contains(kw))
                            });
                            if is_match || goal_lower.contains(&skill_name_lower) {
                                matched_skills.push((skill_name, skill_desc));
                            }
                        }
                    }
                }
            }
        }
        if !matched_skills.is_empty() {
            let mut skills_list = String::new();
            for (s_name, s_desc) in matched_skills {
                skills_list.push_str(&format!("- name: {}\n  description: {}\n", s_name, s_desc));
            }
            let current_exe = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "agy-orchestrator".to_string());
            skills_inject = format!(
                "\n\n==================================================\n\
                 [AVAILABLE PROCEDURAL SKILLS (Level 0 Index)]\n\
                 (To load full instructions, execute: `{} load-skill --name <skill_name>`)\n\n\
                 {}\
                 ==================================================\n\n",
                current_exe,
                skills_list
            );
        }
    }

    let final_prompt = format!(
        "{}{}{}{}{}",
        agents_inject,
        parent_context_inject,
        skills_inject,
        goal,
        report_instruction
    );

    let log_file_path = base_dir.join("logs").join(format!("{}.log", child_name));
    let log_file = File::create(&log_file_path)?;

    let mut cmd = Command::new("agy");
    cmd.arg("--add-dir")
        .arg(&project_path_str)
        .arg("--dangerously-skip-permissions")
        .arg("--print")
        .arg(&final_prompt)
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .stdin(Stdio::null());

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

    let child = cmd.spawn();

    match child {
        Ok(c) => {
            let pid = c.id();
            state.insert(
                child_name.clone(),
                ProjectInfo {
                    path: project_path_str.clone(),
                    goal: goal.clone(),
                    pid,
                    status: "running".to_string(),
                    spawned_at: Local::now().to_rfc3339(),
                },
            );
            save_state(&state)?;

            println!("Successfully spawned sub-agent '{}' in background.", child_name);
            println!("PID: {}", pid);
            println!("Logs: {}", log_file_path.canonicalize()?.to_string_lossy());
        }
        Err(e) => {
            eprintln!("Failed to spawn sub-agent command: {}", e);
            std::process::exit(1);
        }
    }
    Ok(CliResult::Exit)
}

pub fn execute_info() -> io::Result<CliResult> {
    print_info()?;
    Ok(CliResult::Exit)
}

#[allow(clippy::needless_range_loop)]
pub fn compress_log_file(log_path: &std::path::Path) -> io::Result<()> {
    if !log_path.exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(log_path)?;
    let lines: Vec<&str> = content.lines().collect();
    
    if lines.len() < 300 {
        return Ok(());
    }

    let mut compressed_lines = Vec::new();
    let mut i = 0;
    let mut in_long_block = false;
    let mut block_lines = Vec::new();
    
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        
        if trimmed.starts_with("Compiling ") || trimmed.starts_with("Checking ") || trimmed.contains("Downloading ") {
            let mut skip_count = 0;
            while i + skip_count < lines.len() {
                let next_line = lines[i + skip_count].trim();
                if next_line.starts_with("Compiling ") || next_line.starts_with("Checking ") || next_line.contains("Downloading ") {
                    skip_count += 1;
                } else {
                    break;
                }
            }
            if skip_count > 3 {
                compressed_lines.push("[... Rust cargo dependency checking/compiling logs compressed ...]");
                i += skip_count - 1;
            } else {
                compressed_lines.push(line);
            }
            i += 1;
            continue;
        }

        if trimmed.starts_with("[diff_block_start]") || (trimmed.contains("Showing lines ") && trimmed.contains(" of ")) {
            in_long_block = true;
            block_lines.clear();
            block_lines.push(line);
            i += 1;
            continue;
        }

        if in_long_block {
            block_lines.push(line);
            if trimmed.starts_with("[diff_block_end]") || trimmed.contains("The above content shows the entire") || trimmed.contains("The above content does NOT show") {
                in_long_block = false;
                if block_lines.len() > 60 {
                    for j in 0..15 {
                        compressed_lines.push(block_lines[j]);
                    }
                    compressed_lines.push("[... (Tool output content truncated and compressed to optimize token usages) ...]");
                    let len = block_lines.len();
                    for j in (len - 15)..len {
                        compressed_lines.push(block_lines[j]);
                    }
                } else {
                    for bl in &block_lines {
                        compressed_lines.push(*bl);
                    }
                }
            }
            i += 1;
            continue;
        }

        compressed_lines.push(line);
        i += 1;
    }

    let mut file = File::create(log_path)?;
    for cl in compressed_lines {
        writeln!(file, "{}", cl)?;
    }
    
    Ok(())
}

pub fn get_active_dashboard_info() -> Option<(u32, u16)> {
    let paths = std::fs::read_dir("/proc").ok()?;
    for path in paths {
        let entry = path.ok()?;
        let file_name = entry.file_name();
        let pid_str = file_name.to_string_lossy();
        if let Ok(pid) = pid_str.parse::<u32>() {
            let cmdline_path = format!("/proc/{}/cmdline", pid);
            if let Ok(mut file) = File::open(cmdline_path) {
                let mut contents = Vec::new();
                if file.read_to_end(&mut contents).is_ok() {
                    let args: Vec<String> = contents
                        .split(|&b| b == 0)
                        .filter_map(|s| {
                            let s_str = String::from_utf8_lossy(s).to_string();
                            if s_str.is_empty() {
                                None
                            } else {
                                Some(s_str)
                            }
                        })
                        .collect();

                    if args.is_empty() {
                        continue;
                    }

                    let is_orchestrator = args[0].contains("agy-orchestrator");
                    let has_dashboard = args.iter().any(|arg| arg == "dashboard");

                    if is_orchestrator && has_dashboard {
                        let mut port = 8080;
                        for i in 0..args.len() {
                            if args[i] == "--port" && i + 1 < args.len() {
                                if let Ok(p) = args[i+1].parse::<u16>() {
                                    port = p;
                                }
                            }
                        }
                        return Some((pid, port));
                    }
                }
            }
        }
    }
    None
}

pub fn print_info() -> io::Result<()> {
    let current_exe = std::env::current_exe()?;
    let version = env!("AGY_ORCHESTRATOR_VERSION");
    let base_dir = get_base_dir();
    
    let is_dev_mode = crate::backend::health::find_workspace_root().is_ok();
    let mode = if is_dev_mode {
        "Developer (Self-Evolution) Mode"
    } else {
        "Standard Mode"
    };

    let daemon_status = if is_daemon_running() {
        let pid = get_daemon_pid().unwrap_or(0);
        format!("RUNNING (PID: {})", pid)
    } else {
        "STOPPED".to_string()
    };

    let dashboard_status = if let Some((pid, port)) = get_active_dashboard_info() {
        format!("RUNNING (PID: {}, Port: {})", pid, port)
    } else {
        "STOPPED".to_string()
    };

    println!("==================================================");
    println!("🗼 AGY ORCHESTRATOR SYSTEM INFO");
    println!("--------------------------------------------------");
    println!("{:<20} : v{}", "Version", version);
    println!("{:<20} : {}", "Execution Mode", mode);
    println!("{:<20} : {}", "Daemon Status", daemon_status);
    println!("{:<20} : {}", "Dashboard Status", dashboard_status);
    println!("{:<20} : {}", "Binary Location", current_exe.display());
    println!("{:<20} : {}", "Global Config Path", base_dir.display());
    println!("==================================================");

    Ok(())
}
