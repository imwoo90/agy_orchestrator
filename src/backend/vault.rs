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

/// Returns the dynamic path to the Antigravity CLI brain directory,
/// resolved from the HOME environment variable rather than a hardcoded path.
pub fn get_brain_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
    PathBuf::from(home).join(".gemini/antigravity-cli/brain")
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

