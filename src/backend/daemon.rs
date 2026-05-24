#![cfg(not(target_arch = "wasm32"))]

use std::fs::{self, File};
use std::io::{self, Write, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use chrono::Local;

use crate::frontend::app::ProjectInfo;
use super::vault::get_base_dir;
use super::state::{load_state, save_state, is_pid_alive};
use super::issue::{load_issues, save_issues, sync_github_issues};
use super::health::{run_health_checks, find_workspace_root};

pub fn get_daemon_pid() -> Option<u32> {
    let pid_path = get_base_dir().join("daemon.pid");
    if !pid_path.exists() {
        return None;
    }
    let mut file = File::open(pid_path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    contents.trim().parse::<u32>().ok()
}

pub fn is_daemon_running() -> bool {
    if let Some(pid) = get_daemon_pid() {
        is_pid_alive(pid)
    } else {
        false
    }
}

pub fn run_daemon_loop() -> io::Result<()> {
    let base_dir = get_base_dir();
    let notifications_path = base_dir.join("notifications.log");

    // Write current PID to daemon.pid to allow info/status queries
    let pid_path = base_dir.join("daemon.pid");
    let mut pid_file = File::create(&pid_path)?;
    write!(pid_file, "{}", std::process::id())?;

    // RAII guard to clean up pid file on exit
    struct PidCleanup<'a>(&'a Path);
    impl<'a> Drop for PidCleanup<'a> {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.0);
        }
    }
    let _cleanup = PidCleanup(&pid_path);

    let is_evolution_mode = find_workspace_root().is_ok();
    if is_evolution_mode {
        println!("Orchestrator daemon started. [Mode: Self-Evolution]");
        println!("Monitoring active projects and evolution issues...");
    } else {
        println!("Orchestrator daemon started. [Mode: Standard]");
        println!("Monitoring active projects (Self-evolution features are disabled).");
    }

    let mut tick_count: u64 = 0;

    loop {
        tick_count += 1;
        
        // Periodically check for new releases on GitHub (every 1 hour / 720 ticks, or on startup)
        if tick_count == 1 || tick_count.is_multiple_of(720) {
            if let Ok(Some((tag_name, _download_url))) = super::upgrade::check_latest_release() {
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                if let Ok(mut log_file) = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&notifications_path)
                {
                    let _ = writeln!(
                        log_file,
                        "[{}] WARN: A new release '{}' is available on GitHub! Run 'self-upgrade --remote' to upgrade.",
                        timestamp, tag_name
                    );
                }
                println!("[Daemon] A new release '{}' is available on GitHub! Please run 'self-upgrade --remote'.", tag_name);
            }
        }

        // Periodically sync GitHub issues (every 10 minutes / 120 ticks, or on startup)
        if tick_count == 1 || tick_count.is_multiple_of(120) {
            println!("[Daemon] Syncing issues from GitHub repository...");
            if let Err(e) = sync_github_issues() {
                eprintln!("[Daemon] Failed to sync GitHub issues: {}", e);
            }
        }

        let mut state = load_state();
        let mut state_changed = false;

        for (name, info) in state.iter_mut() {
            if info.status == "running" {
                if !is_pid_alive(info.pid) {
                    // Reap zombie process on Unix
                    #[cfg(unix)]
                    {
                        extern "C" {
                            fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
                        }
                        let mut status = 0;
                        unsafe {
                            waitpid(info.pid as i32, &mut status, 0);
                        }
                    }

                    // Process finished!
                    let report_path = Path::new(&info.path).join("report.md");
                    let status = if report_path.exists() {
                        "completed".to_string()
                    } else {
                        "failed".to_string()
                    };

                    info.status = status.clone();
                    state_changed = true;

                    // Log notification
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                    let mut log_file = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&notifications_path)?;

                    if status == "completed" {
                        let _ = writeln!(
                            log_file,
                            "[{}] INFO: Project '{}' task completed successfully.",
                            timestamp, name
                        );
                        
                        if name.starts_with("self_evolution_issue_") {
                            if let Some(id_str) = name.strip_prefix("self_evolution_issue_") {
                                if let Ok(issue_id) = id_str.parse::<u32>() {
                                    println!("Self-evolution task completed for issue #{}. Launching detached self-upgrade process...", issue_id);
                                    
                                    // Clean up report.md
                                    let _ = fs::remove_file(&report_path);

                                    let current_exe = std::env::current_exe().unwrap_or_default();
                                    let mut cmd = Command::new(&current_exe);
                                    cmd.arg("self-upgrade").arg("--resolve-issue").arg(issue_id.to_string());
                                    
                                    // Redirect stdout/stderr of upgrade to log to make sure we capture any output
                                    let upgrade_log_path = base_dir.join("logs").join(format!("self_upgrade_issue_{}.log", issue_id));
                                    if let Ok(upgrade_log_file) = File::create(&upgrade_log_path) {
                                        cmd.stdout(Stdio::from(upgrade_log_file.try_clone().unwrap()));
                                        cmd.stderr(Stdio::from(upgrade_log_file));
                                    }
                                    cmd.stdin(Stdio::null());

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

                                    match cmd.spawn() {
                                        Ok(_) => {
                                            let _ = writeln!(
                                                log_file,
                                                "[{}] INFO: Triggered detached self-upgrade for issue #{}.",
                                                timestamp, issue_id
                                            );
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to spawn self-upgrade process: {}", e);
                                            let _ = writeln!(
                                                log_file,
                                                "[{}] ERROR: Failed to spawn self-upgrade process for issue #{}: {}.",
                                                timestamp, issue_id, e
                                            );
                                        }
                                    }
                                }
                            }
                        } else {
                            // Auto-consolidate memory
                            if let Ok(mut report_file) = File::open(&report_path) {
                                let mut report_content = String::new();
                                if report_file.read_to_string(&mut report_content).is_ok() {
                                    let context_path = Path::new(&info.path).join("context.md");
                                    if let Ok(mut context_file) = fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open(&context_path)
                                    {
                                        let _ = writeln!(
                                            context_file,
                                            "\n\n# 📅 History log from {} (Auto-consolidated)\n\n{}",
                                            timestamp, report_content
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        let _ = writeln!(
                            log_file,
                            "[{}] ERROR: Project '{}' task failed (no report.md found).",
                            timestamp, name
                        );

                        if name.starts_with("self_evolution_issue_") {
                            if let Some(id_str) = name.strip_prefix("self_evolution_issue_") {
                                if let Ok(issue_id) = id_str.parse::<u32>() {
                                    let mut issues = load_issues();
                                    if let Some(issue) = issues.iter_mut().find(|i| i.id == issue_id) {
                                        issue.status = "failed".to_string();
                                        let _ = save_issues(&issues);

                                        // Git rollback
                                        if let Ok(workspace_root) = find_workspace_root() {
                                            let _ = Command::new("git").arg("reset").arg("--hard").arg("HEAD").current_dir(&workspace_root).status();
                                            let _ = Command::new("git").arg("clean").arg("-fd").current_dir(&workspace_root).status();
                                        }

                                        let _ = writeln!(
                                            log_file,
                                            "[{}] ERROR: Self-evolution task failed (no report.md) for issue #{}.",
                                            timestamp, issue_id
                                        );
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Check log size and auto-compress if running PID is alive
                    let log_file_path = base_dir.join("logs").join(format!("{}.log", name));
                    if log_file_path.exists() {
                        if let Ok(metadata) = fs::metadata(&log_file_path) {
                            let file_size = metadata.len();
                            // 100KB threshold
                            if file_size > 100 * 1024 {
                                println!("[Daemon] Log file for running project '{}' is too large ({} bytes). Triggering auto-compression...", name, file_size);
                                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                                if let Ok(mut log_file) = fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open(&notifications_path)
                                {
                                    let _ = writeln!(
                                        log_file,
                                        "[{}] INFO: Auto-compressing large log file for running project '{}' (Size: {} bytes).",
                                        timestamp, name, file_size
                                    );
                                }
                                
                                if let Err(e) = crate::backend::commands::utils::compress_log_file(&log_file_path) {
                                    eprintln!("[Daemon] Failed to auto-compress log for '{}': {}", name, e);
                                } else if let Ok(new_metadata) = fs::metadata(&log_file_path) {
                                    println!("[Daemon] Auto-compression completed for '{}'. New size: {} bytes.", name, new_metadata.len());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for open issues to evolve (only in evolution mode)
        let mut evolution_running = false;
        if is_evolution_mode {
            for (name, info) in state.iter() {
                if name.starts_with("self_evolution_issue_") && info.status == "running" {
                    evolution_running = true;
                    break;
                }
            }
        } else {
            // Standard mode: prevent evolution scans
            evolution_running = true;
        }

        if is_evolution_mode && !evolution_running {
            let mut issues = load_issues();
            if let Some(open_issue) = issues.iter_mut().find(|i| i.status == "open") {
                let issue_id = open_issue.id;
                let issue_title = open_issue.title.clone();
                let issue_body = open_issue.body.clone();

                println!("Daemon detected open issue #{}: '{}'. Spawning self-evolution task...", issue_id, issue_title);

                let task_name = format!("self_evolution_issue_{}", issue_id);
                if let Ok(workspace_path) = find_workspace_root() {
                    let workspace_path_str = workspace_path.to_string_lossy().to_string();
                    let goal = format!(
                        "You are maintaining the orchestrator codebase. Your task is to resolve the following issue:\n\
                         Title: {}\n\n\
                         Description:\n{}\n\n\
                         Please modify the codebase located at '{}' directly, make sure to add tests and run 'cargo test' to verify your solution compiles and passes tests. Once done, you MUST generate a 'report.md' file.",
                        issue_title, issue_body, workspace_path_str
                    );

                    let report_instruction = format!(
                        "\n\n==================================================\n\
                         SYSTEM INSTRUCTIONS FOR COMPLETION:\n\
                         Once you complete your task, you MUST generate a 'report.md' file in the root of the project directory ({})\n\
                         This report must contain:\n\
                         1. A summary of completed tasks.\n\
                         2. Crucial design/architectural choices made.\n\
                         3. Minor choices resolved autonomously.\n\
                         4. A section 'CRITICAL ITEMS FOR REVIEW' containing only items that require manual review or escalation (e.g. costs, API keys, blocker errors). If none, clearly state 'None'.\n\
                         Ensure this report is written before you finish.",
                        workspace_path_str
                    );

                    let full_prompt = format!("{}{}", goal, report_instruction);
                    let log_file_path = base_dir.join("logs").join(format!("{}.log", task_name));
                    
                    if let Ok(log_file) = File::create(&log_file_path) {
                        let mut cmd = Command::new("agy");
                        cmd.arg("--add-dir")
                            .arg(&workspace_path_str)
                            .arg("--dangerously-skip-permissions")
                            .arg("--print")
                            .arg(&full_prompt)
                            .stdout(Stdio::from(log_file.try_clone().unwrap()))
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

                        match cmd.spawn() {
                            Ok(child) => {
                                let pid = child.id();
                                open_issue.status = "in-progress".to_string();
                                let _ = save_issues(&issues);

                                state.insert(
                                    task_name.clone(),
                                    ProjectInfo {
                                        path: workspace_path_str.clone(),
                                        goal: goal.clone(),
                                        pid,
                                        status: "running".to_string(),
                                        spawned_at: Local::now().to_rfc3339(),
                                    },
                                );
                                state_changed = true;

                                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                                if let Ok(mut notify_file) = fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open(&notifications_path)
                                {
                                    let _ = writeln!(
                                        notify_file,
                                        "[{}] INFO: Spawned self-evolution task '{}' for issue #{} (PID: {}).",
                                        timestamp, task_name, issue_id, pid
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to spawn self-evolution task: {}", e);
                            }
                        }
                    }
                }
            }
        }

        if state_changed {
            let _ = save_state(&state);
        }

        // Run proactive health checks every 12 ticks (~60 seconds)
        if tick_count.is_multiple_of(12) {
            println!("[Daemon] Running periodic health checks (tick #{})...", tick_count);
            match run_health_checks() {
                Ok(results) => {
                    let healthy_count = results.iter().filter(|r| r.healthy).count();
                    let failed_count = results.len() - healthy_count;
                    if failed_count > 0 {
                        println!("[Daemon] Health check completed: {} passed, {} FAILED.", healthy_count, failed_count);
                    } else {
                        println!("[Daemon] Health check completed: all {} targets healthy.", healthy_count);
                    }
                }
                Err(e) => {
                    eprintln!("[Daemon] Health check error: {}", e);
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
