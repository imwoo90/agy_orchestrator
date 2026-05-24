use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use chrono::Local;

use crate::backend::vault::get_base_dir;
use crate::backend::state::{load_state, save_state};
use crate::backend::cli::CliResult;

pub fn execute_query_memory(query: String) -> io::Result<CliResult> {
    let base_dir = get_base_dir();
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
        
        if path.extension().is_some_and(|ext| ext == "md") {
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
    Ok(CliResult::Exit)
}

pub fn execute_update_memory(topic: String, content: String) -> io::Result<CliResult> {
    let base_dir = get_base_dir();
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
    Ok(CliResult::Exit)
}

pub fn execute_inject_memory(project: String, query: String) -> io::Result<CliResult> {
    let mut state = load_state();
    let base_dir = get_base_dir();
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
        if path.extension().is_some_and(|ext| ext == "md") {
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
    save_state(&state)?;
    Ok(CliResult::Exit)
}
