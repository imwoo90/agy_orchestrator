import os

def replace_in_file(filepath, target, replacement):
    with open(filepath, 'r') as f:
        content = f.read()
    
    if target not in content:
        raise ValueError(f"Target not found in {filepath}:\n{target}")
    
    # Ensure it only occurs once or replace it once
    new_content = content.replace(target, replacement, 1)
    
    with open(filepath, 'w') as f:
        f.write(new_content)
    print(f"Successfully replaced target in {filepath}")

# 1. skill.rs
replace_in_file(
    "src/backend/commands/skill.rs",
    """    if !skills_dir.exists() {
        eprintln!("Error: Skills registry not found.");
        std::process::exit(1);
    }""",
    """    if !skills_dir.exists() {
        eprintln!("Error: Skills registry not found.");
        return Err(io::Error::new(io::ErrorKind::NotFound, "Error: Skills registry not found."));
    }"""
)

replace_in_file(
    "src/backend/commands/skill.rs",
    """    if !file_path.exists() {
        eprintln!("Error: Skill '{}' not found.", name);
        std::process::exit(1);
    }""",
    """    if !file_path.exists() {
        eprintln!("Error: Skill '{}' not found.", name);
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Error: Skill '{}' not found.", name)));
    }"""
)

replace_in_file(
    "src/backend/commands/skill.rs",
    """    if sanitized_name.is_empty() {
        eprintln!("Error: Invalid skill name.");
        std::process::exit(1);
    }""",
    """    if sanitized_name.is_empty() {
        eprintln!("Error: Invalid skill name.");
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Error: Invalid skill name."));
    }"""
)

# 2. spawn.rs
replace_in_file(
    "src/backend/commands/spawn.rs",
    """    if let Some(info) = state.get_mut(&name) {
        if check_project_status(&name, info) == "running" {
            eprintln!("Error: Project '{}' is already running with PID {}.", name, info.pid);
            std::process::exit(1);
        }
    }""",
    """    if let Some(info) = state.get_mut(&name) {
        if check_project_status(&name, info) == "running" {
            eprintln!("Error: Project '{}' is already running with PID {}.", name, info.pid);
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("Error: Project '{}' is already running with PID {}.", name, info.pid)));
        }
    }"""
)

replace_in_file(
    "src/backend/commands/spawn.rs",
    """        Err(e) => {
            eprintln!("Failed to spawn agy command: {}", e);
            std::process::exit(1);
        }""",
    """        Err(e) => {
            eprintln!("Failed to spawn agy command: {}", e);
            return Err(e);
        }"""
)

# 3. utils.rs
replace_in_file(
    "src/backend/commands/utils.rs",
    """    if !log_file_path.exists() {
        eprintln!("Error: Log file for project '{}' not found.", name);
        std::process::exit(1);
    }""",
    """    if !log_file_path.exists() {
        eprintln!("Error: Log file for project '{}' not found.", name);
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Error: Log file for project '{}' not found.", name)));
    }"""
)

replace_in_file(
    "src/backend/commands/utils.rs",
    """        None => {
            eprintln!("Error: Project '{}' not found.", name);
            std::process::exit(1);
        }""",
    """        None => {
            eprintln!("Error: Project '{}' not found.", name);
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("Error: Project '{}' not found.", name)));
        }"""
)

# 4. memory.rs
replace_in_file(
    "src/backend/commands/memory.rs",
    """    if sanitized_topic.is_empty() {
        eprintln!("Error: Invalid topic name.");
        std::process::exit(1);
    }""",
    """    if sanitized_topic.is_empty() {
        eprintln!("Error: Invalid topic name.");
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Error: Invalid topic name."));
    }"""
)

replace_in_file(
    "src/backend/commands/memory.rs",
    """        None => {
            eprintln!("Error: Project '{}' not found in projects.json.", project);
            std::process::exit(1);
        }""",
    """        None => {
            eprintln!("Error: Project '{}' not found in projects.json.", project);
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("Error: Project '{}' not found in projects.json.", project)));
        }"""
)

replace_in_file(
    "src/backend/commands/memory.rs",
    """    let vault_dir = base_dir.join("memory/vault");
    if !vault_dir.exists() {
        eprintln!("Error: Memory vault directory not found.");
        std::process::exit(1);
    }""",
    """    let vault_dir = base_dir.join("memory/vault");
    if !vault_dir.exists() {
        eprintln!("Error: Memory vault directory not found.");
        return Err(io::Error::new(io::ErrorKind::NotFound, "Error: Memory vault directory not found."));
    }"""
)

# 5. upgrade.rs
replace_in_file(
    "src/backend/commands/upgrade.rs",
    """            Err(e) => {
                eprintln!("Error checking latest release: {}", e);
                std::process::exit(1);
            }""",
    """            Err(e) => {
                eprintln!("Error checking latest release: {}", e);
                return Err(e);
            }"""
)

print("All replacements completed successfully!")
