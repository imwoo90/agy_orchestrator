use std::fs::{self, File};
use std::io::{self, Read, Write};

use crate::backend::vault::get_base_dir;
use crate::backend::cli::CliResult;

pub fn execute_load_skill(name: String) -> io::Result<CliResult> {
    let base_dir = get_base_dir();
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
    Ok(CliResult::Exit)
}

pub fn execute_learn_skill(name: String, description: String, content: String) -> io::Result<CliResult> {
    let base_dir = get_base_dir();
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
    Ok(CliResult::Exit)
}
