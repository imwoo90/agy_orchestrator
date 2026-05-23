#![cfg(not(target_arch = "wasm32"))]

use clap::{Parser, Subcommand};
use std::fs::{self, File};
use std::io::{self, Write, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use chrono::Local;

use crate::frontend::app::{ProjectInfo, Issue};
use super::vault::{get_base_dir, bootstrap_if_needed};
use super::state::{load_state, save_state, check_project_status};
use super::issue::{load_issues, save_issues};
use super::health::run_health_checks;
use super::daemon::{is_daemon_running, get_daemon_pid, run_daemon_loop};
use super::upgrade::run_self_upgrade;

#[derive(Parser, Debug)]
#[command(name = "orchestrate")]
#[command(about = "JIT Memory Agent Orchestrator & Knowledge Vault", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
    /// Inject a knowledge note from the vault into a project's context.md
    InjectMemory {
        #[arg(long)]
        project: String,
        #[arg(long)]
        query: String,
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
    /// Start the embedded web dashboard server
    Dashboard {
        /// Port to bind the dashboard web server to
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// Run a proactive health check on all registered targets
    HealthCheck,
    /// Load a specific procedural skill's full details
    LoadSkill {
        #[arg(long)]
        name: String,
    },
    /// Learn and register a new procedural skill
    LearnSkill {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        content: String,
    },
    /// Compress the active execution log file for token optimization
    Compress {
        #[arg(long)]
        name: String,
    },
}

pub enum CliResult {
    Exit,
    StartDashboard { port: u16 },
}

pub fn run_cli(cli: Cli) -> io::Result<CliResult> {
    bootstrap_if_needed()?;

    let base_dir = get_base_dir();

    match cli.command {
        Commands::Spawn { name, path, goal } => {
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

            // 1. AGENTS.md handling (Project Playbook)
            let agents_path = Path::new(&project_path_str).join("AGENTS.md");
            if !agents_path.exists() {
                if let Ok(mut file) = File::create(&agents_path) {
                    let default_agents_content = format!(
                        "# Project Playbook - {}\n\n\
                         ## 📐 Project Architecture\n\
                         - Describe the folder layout, main files, and dependencies of this project.\n\n\
                         ## 🛠️ Coding Conventions\n\
                         - Define coding standards, framework versions, error handling practices, and lint configurations.\n\n\
                         ## ⚙️ Preferred Tools & Workflow\n\
                         - Document repo-specific commands (e.g. `cargo check`, `dx build`), ports, and deployment scripts.\n",
                        name
                    );
                    let _ = file.write_all(default_agents_content.as_bytes());
                    println!("Hot Memory: Initialized default AGENTS.md (Project Playbook) for '{}'.", name);
                }
            }

            let mut agents_inject = String::new();
            if let Ok(mut file) = File::open(&agents_path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() && !content.trim().is_empty() {
                    agents_inject = format!(
                        "\n\n==================================================\n\
                         [PROJECT-SPECIFIC ARCHITECTURE & CONVENTIONS - AGENTS.MD]\n\
                         (This is the playbook defining developer conventions and architecture rules for this project. Follow these guidelines!)\n\n\
                         {}\n\
                         ==================================================\n\n",
                        content.trim()
                    );
                    println!("JIT: Injected project playbook (AGENTS.md) into spawn prompt.");
                }
            }

            // 2. context.md handling (Hot Memory)
            let context_path = Path::new(&project_path_str).join("context.md");
            let mut context_inject = String::new();
            if context_path.exists() {
                if let Ok(mut file) = File::open(&context_path) {
                    let mut content = String::new();
                    if file.read_to_string(&mut content).is_ok() && !content.trim().is_empty() {
                        context_inject = format!(
                            "\n\n==================================================\n\
                             [ACTIVE PROJECT CONTEXT - HOT MEMORY]\n\
                             (This contains the current state, architecture, and constraints of the project. Please align with this context!)\n\n\
                             {}\n\
                             ==================================================\n\n",
                            content.trim()
                        );
                        println!("JIT: Injected active project context (Hot Memory) into spawn prompt.");
                    }
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
                 4. A section 'CRITICAL ITEMS FOR REVIEW' containing only items that require manual review or escalation (e.g. costs, API keys, blocker errors). If none, clearly state 'None'.\n\n\
                 In addition, you MUST update/overwrite the 'context.md' file in the project root.\n\
                 The 'context.md' acts as the high-density Hot Memory (max 2000 chars) for this project, detailing:\n\
                 - Overall project description and current architecture.\n\
                 - The latest completed changes.\n\
                 - Clear next steps / remaining Todo items.\n\
                 Keep 'context.md' concise and dense. (Detailed historical logs will be archived automatically in 'context_history.md' during consolidate, so do not accumulate old logs inside 'context.md').\n\n\
                 You may also update/refine the 'AGENTS.md' playbook if new permanent developer rules or structure conventions have been established.\n\
                 Ensure both 'report.md' and 'context.md' are updated before you finish.",
                project_path_str
            );

            // 3. JIT Skills Catalog Auto-Injection (Progressive Disclosure)
            let skills_dir = base_dir.join("memory/skills");
            let mut skills_inject = String::new();
            if skills_dir.exists() {
                let mut matched_skills = Vec::new();
                let goal_lower = goal.to_lowercase();
                
                // Extract keywords from goal for matching
                let keywords: std::collections::HashSet<String> = goal_lower
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|s| s.len() >= 4)
                    .map(|s| s.to_string())
                    .collect();

                if let Ok(entries) = fs::read_dir(&skills_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map_or(false, |ext| ext == "md") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                // Basic YAML parser to extract name and description
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
                                    
                                    // Match if skill name or description contains any of the goal keywords
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
                    skills_inject = format!(
                        "\n\n==================================================\n\
                         [AVAILABLE PROCEDURAL SKILLS (Level 0 Index)]\n\
                         (The following procedures are available in your system. To load the full step-by-step instructions for any skill, execute: `./target/release/agy-orchestrator load-skill --name <skill_name>`)\n\n\
                         {}\
                         ==================================================\n\n",
                        skills_list
                    );
                    println!("JIT: Auto-injected matching procedural skills catalog index.");
                }
            }

            let full_prompt = format!("{}{}{}{}{}", agents_inject, context_inject, skills_inject, goal, report_instruction);
            let log_file_path = base_dir.join("logs").join(format!("{}.log", name));
            let log_file = File::create(&log_file_path)?;

            // JIT Knowledge Auto-Injection on Spawn
            let vault_dir = base_dir.join("memory/vault");
            if vault_dir.exists() {
                let name_lower = name.to_lowercase();
                let mut keywords: std::collections::HashSet<String> = goal
                    .to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|s| s.len() >= 4)
                    .map(|s| s.to_string())
                    .collect();
                keywords.insert(name_lower.clone());

                let mut injected_notes = Vec::new();
                if let Ok(entries) = fs::read_dir(&vault_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map_or(false, |ext| ext == "md") {
                            let filename = path.file_name().unwrap().to_string_lossy().to_string();
                            let mut file_content = String::new();
                            if let Ok(mut file) = File::open(&path) {
                                if file.read_to_string(&mut file_content).is_ok() {
                                    let filename_lower = filename.to_lowercase();
                                    let content_lower = file_content.to_lowercase();
                                    
                                    let match_by_name = filename_lower.contains(&name_lower);
                                    let match_by_keyword = keywords.iter().any(|kw| {
                                        kw != "this" && kw != "that" && kw != "with" && kw != "from" &&
                                        (filename_lower.contains(kw) || content_lower.contains(kw))
                                    });

                                    if match_by_name || match_by_keyword {
                                        injected_notes.push((filename, file_content));
                                    }
                                }
                            }
                        }
                    }
                }

                if !injected_notes.is_empty() {
                    let context_path = Path::new(&project_path_str).join("context.md");
                    if let Ok(mut context_file) = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&context_path)
                    {
                        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                        for (filename, content) in injected_notes {
                            let _ = writeln!(
                                context_file,
                                "\n\n# 🧠 Auto-Injected Knowledge from Note '{}' at {}\n\n{}",
                                filename, timestamp, content.trim()
                            );
                            println!("JIT: Auto-injected matching knowledge card '{}' into project context.", filename);
                        }
                    }
                }
            }

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
                return Ok(CliResult::Exit);
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

            // Parse Lessons Learned / 교훈 / 지식 Section
            let mut lines = report_content.lines().peekable();
            let mut lessons_content = String::new();
            let mut in_lessons = false;
            
            while let Some(line) = lines.next() {
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
                    .open(&base_dir.join("notifications.log"))
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
        }
        Commands::QueryMemory { query } => {
            let vault_dir = base_dir.join("memory/vault");
            if !vault_dir.exists() {
                println!("Memory vault directory not found.");
                return Ok(CliResult::Exit);
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
                                text_decorations_helper();
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
            let vault_dir = base_dir.join("memory/vault");
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
        Commands::InjectMemory { project, query } => {
            let mut state = load_state();
            let info = match state.get_mut(&project) {
                Some(i) => i,
                None => {
                    eprintln!("Error: Project '{}' not found in projects.json.", project);
                    std::process::exit(1);
                }
            };

            let vault_dir = base_dir.join("memory/vault");
            if !vault_dir.exists() {
                eprintln!("Error: Memory vault directory not found.");
                std::process::exit(1);
            }

            let query_lower = query.to_lowercase();
            let mut matched_notes = Vec::new();

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
                                matched_notes.push((filename, file_content));
                            }
                        }
                    }
                }
            }

            if matched_notes.is_empty() {
                println!("No matching memory notes found in the vault for query: '{}'", query);
                return Ok(CliResult::Exit);
            }

            let context_path = Path::new(&info.path).join("context.md");
            let mut context_file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&context_path)?;

            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            for (filename, content) in matched_notes {
                writeln!(
                    context_file,
                    "\n\n# 🧠 Injected Knowledge from Note '{}' at {}\n\n{}",
                    filename, timestamp, content.trim()
                )?;
                println!("Injected knowledge from note '{}' into project '{}' context.md", filename, project);
            }
        }
        Commands::Daemon { start, stop, status, run } => {
            let pid_path = base_dir.join("daemon.pid");

            if run {
                run_daemon_loop()?;
            } else if start {
                if is_daemon_running() {
                    let pid = get_daemon_pid().unwrap();
                    println!("Daemon is already running with PID: {}", pid);
                    return Ok(CliResult::Exit);
                }

                let current_exe = std::env::current_exe()?;
                let mut cmd = Command::new(&current_exe);
                cmd.arg("daemon").arg("--run");

                let daemon_log_path = base_dir.join("daemon.log");
                let daemon_log_file = File::create(&daemon_log_path)?;
                cmd.stdout(Stdio::from(daemon_log_file.try_clone()?));
                cmd.stderr(Stdio::from(daemon_log_file));
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

                let child = cmd.spawn()?;
                let pid = child.id();

                let mut file = File::create(&pid_path)?;
                write!(file, "{}", pid)?;

                println!("Orchestrator daemon started in background (PID: {}).", pid);
            } else if stop {
                if !is_daemon_running() {
                    println!("Daemon is not running.");
                    return Ok(CliResult::Exit);
                }

                let pid = get_daemon_pid().unwrap();
                println!("Stopping daemon (PID: {})...", pid);

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
        Commands::Dashboard { port } => {
            return Ok(CliResult::StartDashboard { port });
        }
        Commands::HealthCheck => {
            println!("Running health checks on all registered targets...\n");
            match run_health_checks() {
                Ok(results) => {
                    println!("{:<25} | {:<8} | {:<20} | {}", "Target", "Status", "Checked At", "Message");
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
        }
        Commands::LoadSkill { name } => {
            let skills_dir = base_dir.join("memory/skills");
            if !skills_dir.exists() {
                eprintln!("Error: Skills registry not found.");
                std::process::exit(1);
            }

            let file_path = skills_dir.join(format!("{}.md", name));
            if !file_path.exists() {
                eprintln!("Error: Skill '{}' not found.", name);
                std::process::exit(1);
            }

            let mut content = String::new();
            File::open(&file_path)?.read_to_string(&mut content)?;
            println!("{}", content);
        }
        Commands::LearnSkill { name, description, content } => {
            let skills_dir = base_dir.join("memory/skills");
            fs::create_dir_all(&skills_dir)?;

            let sanitized_name = name
                .trim()
                .to_lowercase()
                .replace(' ', "_")
                .replace(|c: char| !c.is_alphanumeric() && c != '_', "");

            if sanitized_name.is_empty() {
                eprintln!("Error: Invalid skill name.");
                std::process::exit(1);
            }

            let file_path = skills_dir.join(format!("{}.md", sanitized_name));
            let mut file = File::create(&file_path)?;

            // Write YAML frontmatter followed by markdown content
            writeln!(
                file,
                "---\nname: {}\ndescription: {}\nversion: 1.0.0\n---\n\n{}",
                sanitized_name,
                description.trim(),
                content.trim()
            )?;

            println!("Successfully learned and registered skill: '{}' at {}", sanitized_name, file_path.display());
        }
        Commands::Compress { name } => {
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
        }
    }

    Ok(CliResult::Exit)
}

fn text_decorations_helper() {
    println!("--------------------------------------------------");
}

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
