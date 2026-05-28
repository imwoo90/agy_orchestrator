use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::collections::{HashMap, HashSet};

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

const LOG_LINE_COMPRESSION_THRESHOLD: usize = 300;
const MIN_CARGO_SKIP_COUNT: usize = 3;
const MAX_TOOL_OUTPUT_LINES: usize = 60;
const TOOL_OUTPUT_BOUNDARY_LINES: usize = 15;

const DELEGATE_STOP_WORDS: &[&str] = &["this", "that", "with", "from", "for", "and", "the"];

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

struct SubagentPromptBuilder<'a> {
    parent: &'a str,
    project_path: &'a Path,
    base_dir: &'a Path,
    goal: &'a str,
}

impl<'a> SubagentPromptBuilder<'a> {
    fn build(&self) -> String {
        let agents_inject = get_parent_agents_injection(self.parent, self.project_path);
        let parent_context_inject = get_parent_context_injection(self.project_path);
        let report_instruction = get_report_instruction(&self.project_path.to_string_lossy());
        let skills_inject = get_skills_injection(self.base_dir, self.goal);

        format!(
            "{}{}{}{}{}{}",
            agents_inject,
            parent_context_inject,
            skills_inject,
            self.goal,
            report_instruction,
            TOOL_FORMAT_INSTRUCTION
        )
    }
}

pub fn execute_delegate(parent: String, subtask: String, goal: String) -> io::Result<CliResult> {
    let mut state = load_state();
    let base_dir = get_base_dir();

    // 1. Resolve and validate parent/child projects
    let (parent_info, child_name) = get_and_validate_projects(&mut state, &parent, &subtask);
    let project_path_str = parent_info.path.clone();

    // 2. Generate Prompt Components
    let prompt_builder = SubagentPromptBuilder {
        parent: &parent,
        project_path: Path::new(&project_path_str),
        base_dir: &base_dir,
        goal: &goal,
    };
    let final_prompt = prompt_builder.build();

    // 3. Spawn Subagent & Update State
    let log_file_path = base_dir.join("logs").join(format!("{}.log", child_name));
    let child_pid = spawn_subagent(&project_path_str, &final_prompt, &log_file_path)?;
    update_subagent_state(&mut state, &child_name, &project_path_str, &goal, child_pid)?;

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

fn extract_goal_keywords(goal: &str) -> HashSet<String> {
    let goal_lower = goal.to_lowercase();
    goal_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 4)
        .filter(|s| !DELEGATE_STOP_WORDS.contains(s))
        .map(|s| s.to_string())
        .collect()
}

fn parse_skill_metadata(content: &str) -> Option<(String, String)> {
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
        Some((skill_name, skill_desc))
    } else {
        None
    }
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
    let keywords = extract_goal_keywords(goal);

    if let Ok(entries) = fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some((skill_name, skill_desc)) = parse_skill_metadata(&content) {
                        let skill_name_lower = skill_name.to_lowercase();
                        let skill_desc_lower = skill_desc.to_lowercase();
                        let is_match = keywords.iter().any(|kw| {
                            skill_name_lower.contains(kw) || skill_desc_lower.contains(kw)
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

/// Spawns the sub-agent process in the background using `spawn_agy_background`.
fn spawn_subagent(
    project_path: &str,
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

    crate::backend::agy_runner::spawn_agy_background(
        agy_args,
        Some(log_file_path.to_path_buf()),
        None, // 기본 타임아웃 10분
    )
}

/// Records the spawned sub-agent status in state.
fn update_subagent_state(
    state: &mut HashMap<String, ProjectInfo>,
    child_name: &str,
    project_path: &str,
    goal: &str,
    child_pid: u32,
) -> io::Result<()> {
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
    Ok(())
}

pub fn execute_info() -> io::Result<CliResult> {
    print_info()?;
    Ok(CliResult::Exit)
}

fn is_cargo_log(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("Compiling ") || trimmed.starts_with("Checking ") || trimmed.contains("Downloading ")
}

fn skip_cargo_logs(lines: &[&str], current_index: usize) -> (usize, Option<&'static str>) {
    let mut skip_count = 0;
    while current_index + skip_count < lines.len() {
        if is_cargo_log(lines[current_index + skip_count]) {
            skip_count += 1;
        } else {
            break;
        }
    }
    if skip_count > MIN_CARGO_SKIP_COUNT {
        (skip_count, Some("[... Rust cargo dependency checking/compiling logs compressed ...]"))
    } else {
        (0, None)
    }
}

fn is_tool_block_start(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("[diff_block_start]") || (trimmed.contains("Showing lines ") && trimmed.contains(" of "))
}

fn is_tool_block_end(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("[diff_block_end]") || trimmed.contains("The above content shows the entire") || trimmed.contains("The above content does NOT show")
}

fn compress_tool_block(lines: &[&str], current_index: usize) -> (usize, Vec<String>) {
    let mut block_lines = Vec::new();
    let mut idx = current_index;
    let mut closed = false;
    
    while idx < lines.len() {
        let line = lines[idx];
        block_lines.push(line.to_string());
        if is_tool_block_end(line) {
            closed = true;
            idx += 1;
            break;
        }
        idx += 1;
    }
    
    if closed {
        let mut result = Vec::new();
        if block_lines.len() > MAX_TOOL_OUTPUT_LINES {
            for line in block_lines.iter().take(TOOL_OUTPUT_BOUNDARY_LINES) {
                result.push(line.clone());
            }
            result.push("[... (Tool output content truncated and compressed to optimize token usages) ...]".to_string());
            let len = block_lines.len();
            for line in block_lines.iter().skip(len - TOOL_OUTPUT_BOUNDARY_LINES) {
                result.push(line.clone());
            }
        } else {
            result = block_lines;
        }
        (idx - current_index, result)
    } else {
        (idx - current_index, Vec::new())
    }
}

pub fn compress_log_content(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < LOG_LINE_COMPRESSION_THRESHOLD {
        return content.to_string();
    }

    let mut compressed_lines = Vec::new();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i];
        if is_cargo_log(line) {
            let (skip_count, msg) = skip_cargo_logs(&lines, i);
            if let Some(compressed_msg) = msg {
                compressed_lines.push(compressed_msg.to_string());
                i += skip_count;
            } else {
                compressed_lines.push(line.to_string());
                i += 1;
            }
            continue;
        }

        if is_tool_block_start(line) {
            let (skip_count, block) = compress_tool_block(&lines, i);
            compressed_lines.extend(block);
            i += skip_count;
            continue;
        }

        compressed_lines.push(line.to_string());
        i += 1;
    }

    let mut output = compressed_lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    output
}

#[allow(clippy::needless_range_loop)]
pub fn compress_log_file(log_path: &std::path::Path) -> io::Result<()> {
    if !log_path.exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(log_path)?;
    let compressed = compress_log_content(&content);
    
    if compressed != content {
        let mut file = File::create(log_path)?;
        write!(file, "{}", compressed)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_log_content_small() {
        let content = "Hello World\nLine 2\n";
        assert_eq!(compress_log_content(content), content);
    }

    #[test]
    fn test_compress_log_content_cargo() {
        let mut content = String::new();
        for i in 0..100 {
            content.push_str(&format!("Line {}\n", i));
        }
        content.push_str("   Compiling dep1 v0.1.0\n");
        content.push_str("   Compiling dep2 v0.1.0\n");
        content.push_str("   Checking dep3 v0.1.0\n");
        content.push_str("   Compiling dep4 v0.1.0\n");
        content.push_str(" Downloading dep5 v0.1.0\n");
        
        for i in 100..300 {
            content.push_str(&format!("Line {}\n", i));
        }

        let compressed = compress_log_content(&content);
        assert!(compressed.contains("[... Rust cargo dependency checking/compiling logs compressed ...]"));
        assert!(!compressed.contains("Compiling dep1"));
    }

    #[test]
    fn test_compress_log_content_tool_block() {
        let mut content = String::new();
        for i in 0..100 {
            content.push_str(&format!("Line {}\n", i));
        }
        content.push_str("[diff_block_start]\n");
        for i in 0..70 {
            content.push_str(&format!("tool block line {}\n", i));
        }
        content.push_str("[diff_block_end]\n");
        
        for i in 100..300 {
            content.push_str(&format!("Line {}\n", i));
        }

        let compressed = compress_log_content(&content);
        assert!(compressed.contains("[... (Tool output content truncated and compressed to optimize token usages) ...]"));
        assert!(compressed.contains("tool block line 0\n"));
        assert!(compressed.contains("tool block line 13\n"));
        assert!(!compressed.contains("tool block line 14\n"));
        assert!(compressed.contains("tool block line 69\n"));
    }

    #[test]
    fn test_extract_goal_keywords() {
        let goal = "implement test suite for the server functions with rust_testing skill";
        let keywords = extract_goal_keywords(goal);
        assert!(keywords.contains("implement"));
        assert!(keywords.contains("suite"));
        assert!(keywords.contains("server"));
        assert!(keywords.contains("functions"));
        assert!(keywords.contains("rust"));
        assert!(keywords.contains("testing"));
        assert!(keywords.contains("skill"));
        assert!(!keywords.contains("for"));
        assert!(!keywords.contains("the"));
        assert!(!keywords.contains("with"));
    }

    #[test]
    fn test_parse_skill_metadata() {
        let content = "name: rust_testing\ndescription: standard procedure for running tests\nother field\n";
        let metadata = parse_skill_metadata(content);
        assert_eq!(metadata, Some(("rust_testing".to_string(), "standard procedure for running tests".to_string())));
    }
}
