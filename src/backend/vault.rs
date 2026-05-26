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

pub fn get_settings_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
    PathBuf::from(home).join(".gemini/antigravity-cli/settings.json")
}

pub fn is_workspace_authorized(path: &str) -> bool {
    let settings_path = get_settings_path();
    if !settings_path.exists() {
        return false;
    }
    let content = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return false,
    };

    if let Some(permissions) = json.get("permissions") {
        if let Some(allow) = permissions.get("allow") {
            if let Some(arr) = allow.as_array() {
                let has_read = arr.iter().any(|v| v.as_str().is_some_and(|s| s == format!("read_file({})", path)));
                let has_write = arr.iter().any(|v| v.as_str().is_some_and(|s| s == format!("write_file({})", path)));
                let has_command = arr.iter().any(|v| v.as_str() == Some("command(*)"));
                return has_read && has_write && has_command;
            }
        }
    }
    false
}

pub fn authorize_workspace(path: &str) -> std::io::Result<()> {
    let settings_path = get_settings_path();
    if let Some(parent) = settings_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut json: serde_json::Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if json.get("permissions").is_none() {
        json["permissions"] = serde_json::json!({});
    }
    if json["permissions"].get("allow").is_none() {
        json["permissions"]["allow"] = serde_json::json!([]);
    }

    if json.get("trustedWorkspaces").is_none() {
        json["trustedWorkspaces"] = serde_json::json!([]);
    }

    if let Some(tw_arr) = json["trustedWorkspaces"].as_array_mut() {
        let path_val = serde_json::json!(path);
        if !tw_arr.contains(&path_val) {
            tw_arr.push(path_val);
        }
    }

    if let Some(allow_arr) = json["permissions"]["allow"].as_array_mut() {
        let read_perm = format!("read_file({})", path);
        let write_perm = format!("write_file({})", path);
        let cmd_perm = "command(*)".to_string();

        let read_val = serde_json::json!(read_perm);
        let write_val = serde_json::json!(write_perm);
        let cmd_val = serde_json::json!(cmd_perm);

        if !allow_arr.contains(&read_val) {
            allow_arr.push(read_val);
        }
        if !allow_arr.contains(&write_val) {
            allow_arr.push(write_val);
        }
        if !allow_arr.contains(&cmd_val) {
            allow_arr.push(cmd_val);
        }
    }

    let file = std::fs::File::create(&settings_path)?;
    serde_json::to_writer_pretty(file, &json)?;

    Ok(())
}

