use std::fs::{self, File};
use std::io::{self, Read, Write};

use crate::backend::vault::get_base_dir;
use crate::backend::state::load_state;
use crate::backend::daemon::{is_daemon_running, get_daemon_pid};
use crate::backend::cli::CliResult;

// Threshold parameter determining when a log file is considered large enough to run line compression checks.
const LOG_LINE_COMPRESSION_THRESHOLD: usize = 300;
// Minimum number of contiguous cargo compile/check logs required to trigger compression folding.
const MIN_CARGO_SKIP_COUNT: usize = 3;
// Maximum tool output line length boundary limit before applying inline content truncation.
const MAX_TOOL_OUTPUT_LINES: usize = 60;
// Number of lines preserved at the head and tail of truncated tool output logs for debugging reference.
const TOOL_OUTPUT_BOUNDARY_LINES: usize = 15;

// Compress target project logs by parsing and compressing verbose lines
// (e.g. redundant Cargo check logs, long CLI output tool blocks).
// This optimizes the overall context tokens usage.
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

// Search key content of the historical project logs (from context_history.md).
// It searches in case-insensitive mode and prints matches with headers,
// which enables JIT context retrieval by code assistants.
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



// Run system information retrieval and display execution statistics.
pub fn execute_info() -> io::Result<CliResult> {
    print_info()?;
    Ok(CliResult::Exit)
}

// Check if the given log line originates from Rust cargo compilation logs.
fn is_cargo_log(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("Compiling ") || trimmed.starts_with("Checking ") || trimmed.contains("Downloading ")
}

// Skip cargo-related build files logs by counting contiguous matches.
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

// Detect when a tool output block begins in the log.
fn is_tool_block_start(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("[diff_block_start]") || (trimmed.contains("Showing lines ") && trimmed.contains(" of "))
}

// Detect when a tool output block finishes in the log.
fn is_tool_block_end(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("[diff_block_end]") || trimmed.contains("The above content shows the entire") || trimmed.contains("The above content does NOT show")
}

// Extract and compress tool output blocks to preserve token budget.
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

// Perform log line compression filter to shrink verbose workspace logs.
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

// Reads a log file from the filesystem and applies the log content compression.
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

// Scans the proc directory for the running agy-orchestrator dashboard instance.
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

// Prints status, active modes, execution binary path, config directories.
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


}
