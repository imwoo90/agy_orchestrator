use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::collections::HashMap;

use chrono::Local;

use crate::models::ProjectInfo;
use crate::backend::vault::get_base_dir;
use crate::backend::state::{load_state, save_state, check_project_status};
use crate::backend::daemon::{is_daemon_running, get_daemon_pid};
use crate::backend::cli::CliResult;

const TOOL_FORMAT_INSTRUCTION: &str = "\n\n==================================================\n\
     CRITICAL TOOL CALL FORMATTING RULES:\n\
     When calling platform tools (e.g., view_file, list_dir, grep_search, write_to_file, replace_file_content):\n\
     - Do NOT wrap string arguments (like paths or queries) in nested or escaped double quotes.\n\
     - Correct: \"AbsolutePath\": \"/path/to/file\"\n\
     - Incorrect: \"AbsolutePath\": \"\\\"/path/to/file\\\"\"\n\
     Failure to follow this will cause sandbox permission validation to time out and fail!\n\
     ==================================================\n";

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

    // 1. Resolve and validate parent/child projects
    let (parent_info, child_name) = get_and_validate_projects(&mut state, &parent, &subtask);
    let project_path_str = parent_info.path.clone();

    // 2. Generate Prompt Components
    let agents_inject = get_parent_agents_injection(&parent, Path::new(&project_path_str));
    let parent_context_inject = get_parent_context_injection(Path::new(&project_path_str));
    let report_instruction = get_report_instruction(&project_path_str);
    let skills_inject = get_skills_injection(&base_dir, &goal);

    let final_prompt = format!(
        "{}{}{}{}{}{}",
        agents_inject,
        parent_context_inject,
        skills_inject,
        goal,
        report_instruction,
        TOOL_FORMAT_INSTRUCTION
    );

    // 3. Spawn Subagent & Update State
    let log_file_path = base_dir.join("logs").join(format!("{}.log", child_name));
    let _child_pid = spawn_subagent_and_update_state(
        &mut state,
        &child_name,
        &project_path_str,
        &goal,
        &final_prompt,
        &log_file_path,
    )?;

    let log_display = log_file_path.canonicalize()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| log_file_path.to_string_lossy().into_owned());

    println!("Successfully spawned sub-agent '{}' in background (PTY mode).", child_name);
    println!("Logs: {}", log_display);

    Ok(CliResult::Exit)
}

/// Validates and retrieves the parent project's details, and checks that the
/// child/sub-agent process is not already running.
/// If validation fails, prints an error message and exits with status 1.
fn get_and_validate_projects(
    state: &mut HashMap<String, ProjectInfo>,
    parent: &str,
    subtask: &str,
) -> (ProjectInfo, String) {
    let parent_info = match state.get(parent) {
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

    (parent_info, child_name)
}

/// Reads the parent's `AGENTS.md` file (if it exists) and formats it as system instruction injection.
fn get_parent_agents_injection(parent: &str, project_path: &Path) -> String {
    // AGENTS.md inheritance
    let parent_agents_path = project_path.join("AGENTS.md");
    if parent_agents_path.exists() {
        if let Ok(content) = fs::read_to_string(&parent_agents_path) {
            return format!(
                "\n\n==================================================\n\
                 [PARENT PROJECT PLAYBOOK - AGENTS.MD]\n\
                 (This subtask belongs to parent project '{}'. Follow these guidelines!)\n\n\
                 {}\n\
                 ==================================================\n\n",
                parent, content.trim()
            );
        }
    }
    String::new()
}

/// Reads the parent's `context.md` file (if it exists) and formats it as system instruction injection.
fn get_parent_context_injection(project_path: &Path) -> String {
    // Parent context.md JIT inject
    let parent_context_path = project_path.join("context.md");
    if parent_context_path.exists() {
        if let Ok(content) = fs::read_to_string(&parent_context_path) {
            return format!(
                "\n\n==================================================\n\
                 [PARENT ACTIVE CONTEXT - HOT MEMORY]\n\
                 (Current parent project state for your reference):\n\n\
                 {}\n\
                 ==================================================\n\n",
                content.trim()
            );
        }
    }
    String::new()
}

/// Formats the system instructions that prompt the sub-agent to generate a `report.md` on completion.
fn get_report_instruction(project_path_str: &str) -> String {
    format!(
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
    )
}

/// Searches the global skills directory for any skills matching keywords derived from the subtask goal.
/// If matches are found, compiles them into a system instruction block.
fn get_skills_injection(base_dir: &Path, goal: &str) -> String {
    // JIT Skills Catalog Auto-Injection
    let skills_dir = base_dir.join("memory/skills");
    if !skills_dir.exists() {
        return String::new();
    }

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
        format!(
            "\n\n==================================================\n\
             [AVAILABLE PROCEDURAL SKILLS (Level 0 Index)]\n\
             (To load full instructions, execute: `{} load-skill --name <skill_name>`)\n\n\
             {}\
             ==================================================\n\n",
            current_exe,
            skills_list
        )
    } else {
        String::new()
    }
}

/// Spawns the sub-agent process in the background using `spawn_agy_background` and records the status in state.
fn spawn_subagent_and_update_state(
    state: &mut HashMap<String, ProjectInfo>,
    child_name: &str,
    project_path: &str,
    goal: &str,
    final_prompt: &str,
    log_file_path: &Path,
) -> io::Result<u32> {
    // agy_runner를 통해 PTY 백그라운드로 실행.
    // rexpect가 invoke_subagent 서브에이전트 권한 팝업 등 unexpected interactive
    // 프롬프트를 자동 응답하여 hang 없이 완료되도록 보장합니다.
    let agy_args = vec![
        "--add-dir".to_string(),
        project_path.to_string(),
        "--dangerously-skip-permissions".to_string(),
        "--print".to_string(),
        final_prompt.to_string(),
    ];

    let child_pid = crate::backend::agy_runner::spawn_agy_background(
        agy_args,
        Some(log_file_path.to_path_buf()),
        None, // 기본 타임아웃 10분
    )?;

    state.insert(
        child_name.to_string(),
        ProjectInfo {
            path: project_path.to_string(),
            goal: goal.to_string(),
            pid: child_pid,
            status: "running".to_string(),
            spawned_at: Local::now().to_rfc3339(),
        },
    );
    save_state(state)?;

    Ok(child_pid)
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
