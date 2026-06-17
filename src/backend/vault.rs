#![cfg(not(target_arch = "wasm32"))]

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

pub const SYSTEM_INSTRUCTIONS_TEMPLATE: &str = include_str!("system_instructions_template.md");

pub const VAULT_README: &str = "\
# 🗂️ Personal Knowledge Vault

This vault stores modular markdown notes containing your assistant's learned memory and habits.
The assistant queries this database dynamically based on your instructions to load only relevant context on-demand.
";

pub const DEFAULT_CODING_PREFS: &str = "\
# 🎨 Coding Preferences

- **Default stack**: Node.js/JavaScript, TypeScript, Python.
- **Testing**: Write test cases for critical paths. Prefer TDD.
";

pub const DEFAULT_WORKFLOW: &str = "\
# ⚙️ Workflow Delegation & Approvals

- **Auto-approve**: Dependency installs, compile/build commands, test runs, minor code fixes.
- **Escalate**: External billing, cloud infrastructure costs, API credentials, unrecoverable system failures.
";

pub const DEFAULT_SKILL_RUST: &str = "\
---
name: rust_testing
description: Standard procedure for running and writing tests in a Rust cargo project.
version: 1.0.0
---

# Rust Testing Guidelines

## When to Use
Use this skill whenever you write new Rust logic or modify existing Rust crates to ensure regressions are caught.

## Procedure
1. Create unit tests in the same file using the `#[cfg(test)]` module pattern.
2. For integration tests, use the `tests/` directory.
3. Run tests using `cargo test`.
";

pub fn get_base_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
    PathBuf::from(home).join(".agy_orchestrator")
}

pub fn bootstrap_if_needed() -> io::Result<()> {
    let base_dir = get_base_dir();
    fs::create_dir_all(&base_dir)?;
    fs::create_dir_all(base_dir.join("logs"))?;
    fs::create_dir_all(base_dir.join("memory"))?;
    
    let vault_dir = base_dir.join("memory/vault");
    fs::create_dir_all(&vault_dir)?;

    let skills_dir = base_dir.join("memory/skills");
    fs::create_dir_all(&skills_dir)?;

    // 1. Static System Instructions: Always force-overwrite to sync system updates
    let sys_instructions_path = base_dir.join("memory/system_instructions.md");
    let mut file = File::create(sys_instructions_path)?;
    let current_exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "agy-orchestrator".to_string());
    let sys_content = SYSTEM_INSTRUCTIONS_TEMPLATE.replace("{{ORCHESTRATOR_BIN}}", &current_exe);
    file.write_all(sys_content.as_bytes())?;

    // Create default skill if missing
    let default_skill_path = skills_dir.join("rust_testing.md");
    if !default_skill_path.exists() {
        let mut f_skill = File::create(default_skill_path)?;
        f_skill.write_all(DEFAULT_SKILL_RUST.as_bytes())?;
    }

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

pub fn prepare_command(cmd: &mut std::process::Command) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
    let home_path = PathBuf::from(&home);
    
    // Start with the current PATH
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = std::env::split_paths(&current_path).collect();
    
    // Add cargo bin
    let cargo_bin = home_path.join(".cargo/bin");
    if cargo_bin.exists() && !paths.contains(&cargo_bin) {
        paths.insert(0, cargo_bin);
    }
    
    // Add local bin
    let local_bin = home_path.join(".local/bin");
    if local_bin.exists() && !paths.contains(&local_bin) {
        paths.insert(0, local_bin);
    }
    
    // Add NVM node bins dynamically
    let nvm_node_dir = home_path.join(".nvm/versions/node");
    if nvm_node_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&nvm_node_dir) {
            for entry in entries.flatten() {
                let bin_path = entry.path().join("bin");
                if bin_path.exists() && !paths.contains(&bin_path) {
                    paths.insert(0, bin_path);
                }
            }
        }
    }
    
    // Add standard paths if not present
    let std_paths = vec![
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        PathBuf::from("/usr/local/sbin"),
        PathBuf::from("/usr/sbin"),
        PathBuf::from("/sbin"),
    ];
    for p in std_paths {
        if p.exists() && !paths.contains(&p) {
            paths.push(p);
        }
    }
    
    // Join back and set environment variable
    if let Ok(new_path) = std::env::join_paths(paths) {
        cmd.env("PATH", &new_path);
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var("PATH", &new_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_prepare_command_adds_paths() {
        let mut cmd = Command::new("dummy");
        // Clear env to isolate test
        cmd.env_clear();
        
        prepare_command(&mut cmd);
        
        let mut path_found = false;
        for (key, val) in cmd.get_envs() {
            if key == "PATH" {
                path_found = true;
                let val_str = val.unwrap().to_string_lossy();
                assert!(!val_str.is_empty(), "PATH environment variable should not be empty");
                
                let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
                let cargo_bin = format!("{}/.cargo/bin", home);
                if std::path::Path::new(&cargo_bin).exists() {
                    assert!(val_str.contains(&cargo_bin), "PATH should contain cargo bin path: {}", val_str);
                }
            }
        }
        assert!(path_found, "PATH environment variable should be set on the command");
    }

    #[test]
    fn test_prepare_command_resolves_relative_command() {
        let mut cmd = Command::new("cargo");
        cmd.arg("--version");
        prepare_command(&mut cmd);

        // Run the command to ensure it can successfully execute cargo without os error 2!
        let status = cmd.status();
        assert!(status.is_ok(), "Command should find and run cargo successfully: {:?}", status);
    }
}
