use std::fs::{self, File};
use std::io::{self, Write, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use chrono::Local;

use crate::models::ProjectInfo;
use crate::backend::vault::get_base_dir;
use crate::backend::state::{load_state, save_state, check_project_status};
use crate::backend::cli::CliResult;

pub fn execute(name: String, path: String, goal: String) -> io::Result<CliResult> {
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

    let base_dir = get_base_dir();

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
                 (The following procedures are available in your system. To load the full step-by-step instructions for any skill, execute: `{} load-skill --name <skill_name>`)\n\n\
                 {}\
                 ==================================================\n\n",
                current_exe,
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
                if path.extension().is_some_and(|ext| ext == "md") {
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

    Ok(CliResult::Exit)
}
