use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use chrono::Local;

const SYSTEM_INSTRUCTIONS_TEMPLATE: &str = include_str!("system_instructions_template.md");

const VAULT_README: &str = "\
# 🗂️ Personal Knowledge Vault

This vault stores modular markdown notes containing your assistant's learned memory and habits.
The assistant queries this database dynamically based on your instructions to load only relevant context on-demand.
";

const DEFAULT_CODING_PREFS: &str = "\
# 🎨 Coding Preferences

- **Default stack**: Node.js/JavaScript, TypeScript, Python.
- **Testing**: Write test cases for critical paths. Prefer TDD.
";

const DEFAULT_WORKFLOW: &str = "\
# ⚙️ Workflow Delegation & Approvals

- **Auto-approve**: Dependency installs, compile/build commands, test runs, minor code fixes.
- **Escalate**: External billing, cloud infrastructure costs, API credentials, unrecoverable system failures.
";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ProjectInfo {
    path: String,
    goal: String,
    pid: u32,
    status: String,
    spawned_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Issue {
    id: u32,
    title: String,
    body: String,
    status: String, // "open", "in-progress", "resolved", "failed"
    created_at: String,
    resolved_at: Option<String>,
}

#[derive(Parser, Debug)]
#[command(name = "orchestrate")]
#[command(about = "JIT Memory Agent Orchestrator & Knowledge Vault", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Spawn a new project task in the background
    Spawn {
        #[arg(long)]
        name: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        goal: String,
    },
    /// List all registered projects and their status
    List,
    /// Get status and show report for a specific project
    Status {
        #[arg(long)]
        name: String,
    },
    /// Fetch the target project's path and local context for JIT load
    GetContext {
        #[arg(long)]
        name: String,
    },
    /// Consolidate the report.md into context.md and update project status
    Consolidate {
        #[arg(long)]
        name: String,
    },
    /// Search the personal knowledge vault for relevant context notes
    QueryMemory {
        #[arg(long)]
        query: String,
    },
    /// Create or update a note in the personal knowledge vault
    UpdateMemory {
        #[arg(long)]
        topic: String,
        #[arg(long)]
        content: String,
    },
    /// Manage the background orchestrator daemon
    Daemon {
        /// Start the daemon in the background
        #[arg(long)]
        start: bool,
        /// Stop the running background daemon
        #[arg(long)]
        stop: bool,
        /// Check if the daemon is currently running
        #[arg(long)]
        status: bool,
        /// Run the daemon in the foreground (blocking loop)
        #[arg(long)]
        run: bool,
    },
    /// Test, compile, and hot-reload/upgrade the orchestrator binary and daemon
    SelfUpgrade {
        /// Resolve a specific issue ID upon successful upgrade
        #[arg(long)]
        resolve_issue: Option<u32>,
    },
    /// Manage and register self-evolution issues
    Issue {
        /// Create a new issue with a title
        #[arg(long)]
        create: Option<String>,
        /// Body/details of the new issue
        #[arg(long)]
        body: Option<String>,
        /// List registered issues
        #[arg(long)]
        list: bool,
        /// Mark a specific issue as resolved by ID
        #[arg(long)]
        resolve: Option<u32>,
    },
}

fn get_base_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
    PathBuf::from(home).join(".agy_orchestrator")
}

fn bootstrap_if_needed() -> io::Result<()> {
    let base_dir = get_base_dir();
    fs::create_dir_all(&base_dir)?;
    fs::create_dir_all(base_dir.join("logs"))?;
    fs::create_dir_all(base_dir.join("memory"))?;
    
    let vault_dir = base_dir.join("memory/vault");
    fs::create_dir_all(&vault_dir)?;

    // 1. Static System Instructions: Always force-overwrite to sync system updates
    let sys_instructions_path = base_dir.join("memory/system_instructions.md");
    let mut file = File::create(sys_instructions_path)?;
    file.write_all(SYSTEM_INSTRUCTIONS_TEMPLATE.as_bytes())?;

    // 2. Vault Default notes (write only if they don't exist to preserve user updates)
    let readme_path = vault_dir.join("README.md");
    let mut f_readme = File::create(readme_path)?;
    f_readme.write_all(VAULT_README.as_bytes())?;

    let coding_prefs_path = vault_dir.join("coding_preferences.md");
    if !coding_prefs_path.exists() {
        let mut file = File::create(coding_prefs_path)?;
        file.write_all(DEFAULT_CODING_PREFS.as_bytes())?;
    }

    let workflow_path = vault_dir.join("workflow_delegation.md");
    if !workflow_path.exists() {
        let mut file = File::create(workflow_path)?;
        file.write_all(DEFAULT_WORKFLOW.as_bytes())?;
    }

    // 3. Projects JSON state file
    let projects_path = base_dir.join("projects.json");
    if !projects_path.exists() {
        let mut file = File::create(projects_path)?;
        file.write_all(b"{}")?;
    }

    // 4. Issues JSON state file
    let issues_path = base_dir.join("issues.json");
    if !issues_path.exists() {
        let mut file = File::create(issues_path)?;
        file.write_all(b"[]")?;
    }

    Ok(())
}

fn load_state() -> HashMap<String, ProjectInfo> {
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

fn save_state(state: &HashMap<String, ProjectInfo>) -> io::Result<()> {
    let path = get_base_dir().join("projects.json");
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, state)?;
    Ok(())
}

fn load_issues() -> Vec<Issue> {
    let path = get_base_dir().join("issues.json");
    if !path.exists() {
        return Vec::new();
    }
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    serde_json::from_reader(file).unwrap_or_else(|_| Vec::new())
}

fn save_issues(issues: &[Issue]) -> io::Result<()> {
    let path = get_base_dir().join("issues.json");
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, issues)?;
    Ok(())
}


fn is_pid_alive(pid: u32) -> bool {
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

fn check_project_status(_name: &str, info: &mut ProjectInfo) -> String {
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

fn get_daemon_pid() -> Option<u32> {
    let pid_path = get_base_dir().join("daemon.pid");
    if !pid_path.exists() {
        return None;
    }
    let mut file = File::open(pid_path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    contents.trim().parse::<u32>().ok()
}

fn is_daemon_running() -> bool {
    if let Some(pid) = get_daemon_pid() {
        is_pid_alive(pid)
    } else {
        false
    }
}

fn run_daemon_loop() -> io::Result<()> {
    let base_dir = get_base_dir();
    let notifications_path = base_dir.join("notifications.log");

    println!("Orchestrator daemon started in foreground. Monitoring projects...");

    loop {
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
                }
            }
        }

        // Check for open issues to evolve
        let mut evolution_running = false;
        for (name, info) in state.iter() {
            if name.starts_with("self_evolution_issue_") && info.status == "running" {
                evolution_running = true;
                break;
            }
        }

        if !evolution_running {
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
            save_state(&state)?;
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

fn find_workspace_root() -> io::Result<PathBuf> {
    let mut current_dir = std::env::current_exe()?;
    while current_dir.pop() {
        if current_dir.join("Cargo.toml").exists() {
            return Ok(current_dir);
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "Workspace Cargo.toml not found"))
}

fn rollback_upgrade(current_exe: &Path, backup_exe: &Path, restart_daemon: bool, reason: &str) -> io::Result<()> {
    eprintln!("CRITICAL ERROR: {}. Initiating rollback...", reason);

    if backup_exe.exists() {
        println!("Restoring backup binary...");
        let _ = fs::remove_file(current_exe);
        if let Err(e) = fs::rename(backup_exe, current_exe) {
            eprintln!("Failed to restore backup binary: {}", e);
            return Err(e);
        }
    }

    if restart_daemon {
        println!("Restarting old daemon...");
        let _ = Command::new(current_exe)
            .arg("daemon")
            .arg("--start")
            .status();
    }

    Err(io::Error::new(io::ErrorKind::Other, format!("Upgrade failed and rolled back: {}", reason)))
}

fn run_self_upgrade(resolve_issue: Option<u32>) -> io::Result<()> {
    let workspace_root = find_workspace_root()?;
    println!("Found workspace root: {}", workspace_root.display());

    println!("Running tests via 'cargo test'...");
    let test_status = Command::new("cargo")
        .arg("test")
        .current_dir(&workspace_root)
        .status()?;

    if !test_status.success() {
        if let Some(issue_id) = resolve_issue {
            let mut issues = load_issues();
            if let Some(issue) = issues.iter_mut().find(|i| i.id == issue_id) {
                issue.status = "failed".to_string();
                let _ = save_issues(&issues);
            }
            let _ = Command::new("git").arg("reset").arg("--hard").arg("HEAD").current_dir(&workspace_root).status();
            let _ = Command::new("git").arg("clean").arg("-fd").current_dir(&workspace_root).status();
        }
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Tests failed. Upgrade aborted.",
        ));
    }
    println!("Tests passed successfully!");

    let current_exe = std::env::current_exe()?;
    let backup_exe = current_exe.with_extension("bak");
    let new_exe = workspace_root.join("target/release/agy-orchestrator");

    println!("Backing up active binary to {}...", backup_exe.display());
    if backup_exe.exists() {
        fs::remove_file(&backup_exe)?;
    }
    fs::copy(&current_exe, &backup_exe)?;

    println!("Compiling release binary via 'cargo build --release'...");
    let build_status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&workspace_root)
        .status()?;

    if !build_status.success() {
        let _ = fs::remove_file(&backup_exe);
        if let Some(issue_id) = resolve_issue {
            let mut issues = load_issues();
            if let Some(issue) = issues.iter_mut().find(|i| i.id == issue_id) {
                issue.status = "failed".to_string();
                let _ = save_issues(&issues);
            }
            let _ = Command::new("git").arg("reset").arg("--hard").arg("HEAD").current_dir(&workspace_root).status();
            let _ = Command::new("git").arg("clean").arg("-fd").current_dir(&workspace_root).status();
        }
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Compilation failed. Upgrade aborted.",
        ));
    }
    println!("Compilation completed successfully!");

    if current_exe != new_exe {
        println!("Installing upgraded binary...");
        let old_exe = current_exe.with_extension("old");
        if old_exe.exists() {
            fs::remove_file(&old_exe)?;
        }
        let _ = fs::rename(&current_exe, &old_exe);
        if let Err(e) = fs::copy(&new_exe, &current_exe) {
            eprintln!("Failed to copy upgraded binary: {}", e);
            println!("Restoring stable backup...");
            let _ = fs::rename(&old_exe, &current_exe);
            let _ = fs::remove_file(&backup_exe);
            if let Some(issue_id) = resolve_issue {
                let mut issues = load_issues();
                if let Some(issue) = issues.iter_mut().find(|i| i.id == issue_id) {
                    issue.status = "failed".to_string();
                    let _ = save_issues(&issues);
                }
                let _ = Command::new("git").arg("reset").arg("--hard").arg("HEAD").current_dir(&workspace_root).status();
                let _ = Command::new("git").arg("clean").arg("-fd").current_dir(&workspace_root).status();
            }
            return Err(e);
        }
        let _ = fs::remove_file(&old_exe);
    }

    let daemon_was_running = is_daemon_running();
    let old_pid = get_daemon_pid();

    let rollback_and_fail_issue = |reason: &str| -> io::Result<()> {
        if let Some(issue_id) = resolve_issue {
            let mut issues = load_issues();
            if let Some(issue) = issues.iter_mut().find(|i| i.id == issue_id) {
                issue.status = "failed".to_string();
                let _ = save_issues(&issues);
            }
            let _ = Command::new("git").arg("reset").arg("--hard").arg("HEAD").current_dir(&workspace_root).status();
            let _ = Command::new("git").arg("clean").arg("-fd").current_dir(&workspace_root).status();
        }
        rollback_upgrade(&current_exe, &backup_exe, daemon_was_running, reason)
    };

    // Run basic sanity check on the new binary
    println!("Performing sanity checks on the new binary...");
    let sanity_status = Command::new(&current_exe)
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match sanity_status {
        Ok(status) if status.success() => {}
        _ => {
            return rollback_and_fail_issue("New binary failed basic sanity check --help");
        }
    }

    if daemon_was_running {
        println!("Stopping currently running daemon (PID: {:?})...", old_pid);
        let stop_status = Command::new(&backup_exe)
            .arg("daemon")
            .arg("--stop")
            .status()?;
        if !stop_status.success() {
            eprintln!("Warning: Failed to cleanly stop old daemon process.");
        }

        println!("Starting upgraded daemon...");
        let start_status = Command::new(&current_exe)
            .arg("daemon")
            .arg("--start")
            .status()?;

        if !start_status.success() {
            return rollback_and_fail_issue("Failed to launch new daemon");
        }

        println!("Waiting 3 seconds for health check...");
        std::thread::sleep(std::time::Duration::from_secs(3));

        if !is_daemon_running() {
            return rollback_and_fail_issue("Upgraded daemon crashed immediately on boot");
        }

        println!("Upgraded daemon is healthy (PID: {:?}).", get_daemon_pid());
    }

    // Clean up backup binary on successful upgrade
    if backup_exe.exists() {
        let _ = fs::remove_file(&backup_exe);
    }

    // Handle post-upgrade issue resolution and Git staging/committing
    if let Some(issue_id) = resolve_issue {
        let mut issues = load_issues();
        if let Some(issue) = issues.iter_mut().find(|i| i.id == issue_id) {
            let issue_title = issue.title.clone();
            issue.status = "resolved".to_string();
            issue.resolved_at = Some(Local::now().to_rfc3339());
            save_issues(&issues)?;

            println!("Staging and committing evolution changes to Git...");
            let _ = Command::new("git").arg("add").arg(".").current_dir(&workspace_root).status();
            let commit_msg = format!("Auto-evolution: Resolves Issue #{}: {}", issue_id, issue_title);
            let _ = Command::new("git").arg("commit").arg("-m").arg(&commit_msg).current_dir(&workspace_root).status();
            
            if let Ok(output) = Command::new("git").arg("remote").current_dir(&workspace_root).output() {
                let remote_str = String::from_utf8_lossy(&output.stdout);
                if !remote_str.trim().is_empty() {
                    let _ = Command::new("git").arg("push").current_dir(&workspace_root).status();
                }
            }
        }
    }

    println!("Successfully upgraded to the new version!");
    Ok(())
}

fn main() -> io::Result<()> {
    bootstrap_if_needed()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Spawn { name, path, goal } => {
            let base_dir = get_base_dir();
            let project_path = fs::canonicalize(Path::new(&path))
                .unwrap_or_else(|_| {
                    let p = Path::new(&path);
                    let _ = fs::create_dir_all(p);
                    fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from(&path))
                });
            let project_path_str = project_path.to_string_lossy().to_string();

            let mut state = load_state();
            if let Some(info) = state.get_mut(&name) {
                if check_project_status(&name, info) == "running" {
                    eprintln!("Error: Project '{}' is already running with PID {}.", name, info.pid);
                    std::process::exit(1);
                }
            }

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
                project_path_str
            );

            let full_prompt = format!("{}{}", goal, report_instruction);
            let log_file_path = base_dir.join("logs").join(format!("{}.log", name));
            let log_file = File::create(&log_file_path)?;

            let mut cmd = Command::new("agy");
            cmd.arg("--add-dir")
                .arg(&project_path_str)
                .arg("--dangerously-skip-permissions")
                .arg("--print")
                .arg(&full_prompt)
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
                        name.clone(),
                        ProjectInfo {
                            path: project_path_str.clone(),
                            goal: goal.clone(),
                            pid,
                            status: "running".to_string(),
                            spawned_at: Local::now().to_rfc3339(),
                        },
                    );
                    save_state(&state)?;

                    println!("Successfully spawned project '{}' in background.", name);
                    println!("PID: {}", pid);
                    println!("Logs: {}", log_file_path.canonicalize()?.to_string_lossy());
                    println!("Target Directory: {}", project_path_str);
                }
                Err(e) => {
                    eprintln!("Failed to spawn agy command: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::List => {
            let mut state = load_state();
            if state.is_empty() {
                println!("No projects registered.");
                return Ok(());
            }

            println!(
                "{:<15} | {:<6} | {:<10} | {:<20} | {}",
                "Project Name", "PID", "Status", "Spawned At", "Path"
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
        }
        Commands::Status { name } => {
            let mut state = load_state();
            
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
                    let base_dir = get_base_dir();
                    println!("Note: Project failed. Check logs for details: {}", base_dir.join("logs").join(format!("{}.log", name)).display());
                }
            }
        }
        Commands::GetContext { name } => {
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

            let context_path = Path::new(&path_str).join("context.md");
            if context_path.exists() {
                println!("\n--- [context.md Content] ---");
                let mut context_content = String::new();
                File::open(context_path)?.read_to_string(&mut context_content)?;
                println!("{}", context_content);
            } else {
                println!("\nNo context.md file exists yet in the project directory.");
            }
        }
        Commands::Consolidate { name } => {
            let mut state = load_state();
            
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

            let context_path = Path::new(&path_str).join("context.md");
            let mut context_file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&context_path)?;

            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(
                context_file,
                "\n\n# 📅 History log from {} (Spawned at {})\n\n{}",
                timestamp, spawned_at, report_content
            )?;

            save_state(&state)?;

            println!("Successfully consolidated report.md into context.md for project '{}'.", name);
            println!("Updated status to 'completed' in projects.json.");
        }
        Commands::QueryMemory { query } => {
            let vault_dir = get_base_dir().join("memory/vault");
            if !vault_dir.exists() {
                println!("Memory vault directory not found.");
                return Ok(());
            }

            let query_lower = query.to_lowercase();
            let mut found_any = false;

            let entries = fs::read_dir(vault_dir)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                
                if path.extension().map_or(false, |ext| ext == "md") {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let mut file_content = String::new();
                    
                    if let Ok(mut file) = File::open(&path) {
                        if file.read_to_string(&mut file_content).is_ok() {
                            let match_in_name = filename.to_lowercase().contains(&query_lower);
                            let match_in_content = file_content.to_lowercase().contains(&query_lower);
                            
                            if match_in_name || match_in_content {
                                found_any = true;
                                println!("\n==================================================");
                                println!("📂 Note: {}", filename);
                                println!("--------------------------------------------------");
                                println!("{}", file_content.trim());
                            }
                        }
                    }
                }
            }

            if !found_any {
                println!("No matching memory notes found in the vault for query: '{}'", query);
            }
        }
        Commands::UpdateMemory { topic, content } => {
            let vault_dir = get_base_dir().join("memory/vault");
            fs::create_dir_all(&vault_dir)?;

            let sanitized_topic = topic
                .trim()
                .to_lowercase()
                .replace(' ', "_")
                .replace(|c: char| !c.is_alphanumeric() && c != '_', "");
            
            if sanitized_topic.is_empty() {
                eprintln!("Error: Invalid topic name.");
                std::process::exit(1);
            }

            let file_path = vault_dir.join(format!("{}.md", sanitized_topic));
            let mut file = File::create(&file_path)?;
            file.write_all(content.as_bytes())?;

            println!("Successfully updated memory note: {}", file_path.display());
        }
        Commands::Daemon { start, stop, status, run } => {
            let base_dir = get_base_dir();
            let pid_path = base_dir.join("daemon.pid");

            if run {
                run_daemon_loop()?;
            } else if start {
                if is_daemon_running() {
                    let pid = get_daemon_pid().unwrap();
                    println!("Daemon is already running with PID: {}", pid);
                    return Ok(());
                }

                let current_exe = std::env::current_exe()?;
                let mut cmd = Command::new(&current_exe);
                cmd.arg("daemon").arg("--run");

                // Redirect stdout/stderr to a daemon log file to avoid panic on broken pipe
                let daemon_log_path = base_dir.join("daemon.log");
                let daemon_log_file = File::create(&daemon_log_path)?;
                cmd.stdout(Stdio::from(daemon_log_file.try_clone()?));
                cmd.stderr(Stdio::from(daemon_log_file));
                cmd.stdin(Stdio::null());

                // Detach process on Unix
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

                let child = cmd.spawn()?;
                let pid = child.id();

                let mut file = File::create(&pid_path)?;
                write!(file, "{}", pid)?;

                println!("Orchestrator daemon started in background (PID: {}).", pid);
            } else if stop {
                if !is_daemon_running() {
                    println!("Daemon is not running.");
                    return Ok(());
                }

                let pid = get_daemon_pid().unwrap();
                println!("Stopping daemon (PID: {})...", pid);

                // Kill process on Unix
                let _ = Command::new("kill").arg(pid.to_string()).status();
                let _ = fs::remove_file(&pid_path);

                println!("Daemon stopped successfully.");
            } else if status {
                if is_daemon_running() {
                    let pid = get_daemon_pid().unwrap();
                    println!("Daemon is RUNNING (PID: {}).", pid);
                } else {
                    println!("Daemon is STOPPED.");
                }
            } else {
                println!("Please specify --start, --stop, or --status.");
            }
        }
        Commands::SelfUpgrade { resolve_issue } => {
            run_self_upgrade(resolve_issue)?;
        }
        Commands::Issue { create, body, list, resolve } => {
            let mut issues = load_issues();
            if list {
                if issues.is_empty() {
                    println!("No registered issues found.");
                } else {
                    println!("{:<5} | {:<25} | {:<12} | {:<20} | {}", "ID", "Title", "Status", "Created At", "Body");
                    println!("{}", "-".repeat(95));
                    for issue in &issues {
                        let created = issue.created_at.get(..19).unwrap_or(&issue.created_at).replace('T', " ");
                        let body_truncated = if issue.body.len() > 30 {
                            format!("{}...", &issue.body[..27])
                        } else {
                            issue.body.clone()
                        };
                        println!(
                            "{:<5} | {:<25} | {:<12} | {:<20} | {}",
                            issue.id, issue.title, issue.status, created, body_truncated
                        );
                    }
                }
            } else if let Some(title) = create {
                let next_id = issues.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                let body_str = body.unwrap_or_else(|| "".to_string());
                let new_issue = Issue {
                    id: next_id,
                    title: title.clone(),
                    body: body_str,
                    status: "open".to_string(),
                    created_at: Local::now().to_rfc3339(),
                    resolved_at: None,
                };
                issues.push(new_issue);
                save_issues(&issues)?;
                println!("Successfully registered issue #{} '{}'.", next_id, title);
            } else if let Some(id) = resolve {
                if let Some(issue) = issues.iter_mut().find(|i| i.id == id) {
                    issue.status = "resolved".to_string();
                    issue.resolved_at = Some(Local::now().to_rfc3339());
                    save_issues(&issues)?;
                    println!("Successfully marked issue #{} as resolved.", id);
                } else {
                    eprintln!("Error: Issue #{} not found.", id);
                    std::process::exit(1);
                }
            } else {
                println!("Please specify --create, --list, or --resolve.");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_alive() {
        assert!(is_pid_alive(std::process::id()));
    }

    #[test]
    fn test_evolution_comment() {
        let content = std::fs::read_to_string("src/main.rs").expect("Failed to read src/main.rs");
        assert!(content.contains("// Evolution verified!"));
    }
}

// Evolution verified!
