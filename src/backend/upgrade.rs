#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use chrono::Local;

use super::issue::{load_issues, save_issues};
use super::daemon::{is_daemon_running, get_daemon_pid};
use super::health::{find_workspace_root};

pub fn rollback_upgrade(current_exe: &Path, backup_exe: &Path, restart_daemon: bool, reason: &str) -> io::Result<()> {
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

pub fn run_self_upgrade(resolve_issue: Option<u32>) -> io::Result<()> {
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
