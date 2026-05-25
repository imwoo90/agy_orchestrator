#![cfg(not(target_arch = "wasm32"))]

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use chrono::Local;

use super::issue::{load_issues, save_issues, close_github_issue};
use super::daemon::{is_daemon_running, get_daemon_pid};
use super::health::{find_workspace_root};
use super::vault::get_base_dir;
use std::path::PathBuf;

pub fn get_active_current_exe() -> io::Result<PathBuf> {
    let current_exe = std::env::current_exe()?;
    let path_str = current_exe.to_string_lossy();
    if path_str.ends_with(" (deleted)") || !current_exe.exists() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
        let stable_exe = PathBuf::from(home).join(".local/bin/agy-orchestrator");
        if stable_exe.exists() {
            return Ok(stable_exe);
        }
    }
    Ok(current_exe)
}

pub fn restart_daemon_process(current_exe: &Path) -> io::Result<()> {
    let service_file = get_base_dir().parent().unwrap_or(&PathBuf::from("/home/wimvm")).join(".config/systemd/user/agy-orchestrator.service");
    if service_file.exists() {
        println!("Detected systemd user service. Restarting via systemctl...");
        let systemd_status = Command::new("systemctl")
            .arg("--user")
            .arg("restart")
            .arg("agy-orchestrator.service")
            .status();
        
        if let Ok(status) = systemd_status {
            if status.success() {
                println!("Systemd user service restarted successfully.");
                return Ok(());
            }
        }
        println!("Warning: Failed to restart via systemctl, falling back to legacy start...");
    }

    let start_status = Command::new(current_exe)
        .arg("daemon")
        .arg("--start")
        .env_remove("PORT")
        .env_remove("ADDR")
        .env_remove("IP")
        .env_remove("DIOXUS_ACTIVE")
        .status()?;
    if !start_status.success() {
        return Err(io::Error::other("Failed to launch daemon in background"));
    }
    Ok(())
}

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
        let _ = restart_daemon_process(current_exe);
    }

    Err(io::Error::other(format!("Upgrade failed and rolled back: {}", reason)))
}

pub fn run_self_upgrade(resolve_issue: Option<u32>) -> io::Result<()> {
    let workspace_root = match find_workspace_root() {
        Ok(root) => root,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Self-upgrade is only available in a git developer workspace: {}", e)
            ));
        }
    };
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
        return Err(io::Error::other(
            "Tests failed. Upgrade aborted.",
        ));
    }
    println!("Tests passed successfully!");

    let current_exe = get_active_current_exe()?;
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
        return Err(io::Error::other(
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
        .env_remove("PORT")
        .env_remove("ADDR")
        .env_remove("IP")
        .env_remove("DIOXUS_ACTIVE")
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
            .env_remove("PORT")
            .env_remove("ADDR")
            .env_remove("IP")
            .env_remove("DIOXUS_ACTIVE")
            .status()?;
        if !stop_status.success() {
            eprintln!("Warning: Failed to cleanly stop old daemon process.");
        }

        println!("Starting upgraded daemon...");
        if let Err(e) = restart_daemon_process(&current_exe) {
            return rollback_and_fail_issue(&format!("Failed to launch new daemon: {}", e));
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

pub fn check_latest_release() -> io::Result<Option<(String, String)>> {
    let output = Command::new("curl")
        .arg("-sI")
        .arg("https://github.com/imwoo90/agy_orchestrator/releases/latest")
        .output()?;

    if !output.status.success() {
        return Err(io::Error::other("Failed to check latest release via curl"));
    }

    let headers = String::from_utf8_lossy(&output.stdout);
    let mut tag_name = None;

    for line in headers.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.to_lowercase().starts_with("location:") {
            if let Some(pos) = line_trimmed.find("/tag/") {
                let tag = line_trimmed[pos + 5..].trim().to_string();
                if !tag.is_empty() {
                    tag_name = Some(tag);
                    break;
                }
            }
        }
    }

    let tag_name = match tag_name {
        Some(t) => t,
        None => return Ok(None),
    };

    let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
    if tag_name == current_version {
        return Ok(None);
    }

    let url = format!(
        "https://github.com/imwoo90/agy_orchestrator/releases/download/{}/agy-orchestrator-linux.tar.gz",
        tag_name
    );

    Ok(Some((tag_name, url)))
}

pub fn run_remote_upgrade(download_url: &str) -> io::Result<()> {
    let current_exe = get_active_current_exe()?;
    let parent_dir = current_exe.parent().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent directory of current exe not found"))?;
    
    let temp_tar = parent_dir.join("agy-orchestrator-new.tar.gz");

    println!("Downloading new package from {}...", download_url);
    let download_status = Command::new("curl")
        .arg("-L")
        .arg("-o")
        .arg(&temp_tar)
        .arg(download_url)
        .status()?;

    if !download_status.success() {
        return Err(io::Error::other("Failed to download new package via curl"));
    }

    println!("Extracting package...");
    let temp_extract_dir = parent_dir.join("agy_extract_temp");
    if temp_extract_dir.exists() {
        fs::remove_dir_all(&temp_extract_dir)?;
    }
    fs::create_dir_all(&temp_extract_dir)?;

    let tar_status = Command::new("tar")
        .arg("-xzf")
        .arg(&temp_tar)
        .arg("-C")
        .arg(&temp_extract_dir)
        .status()?;

    let _ = fs::remove_file(&temp_tar);

    if !tar_status.success() {
        let _ = fs::remove_dir_all(&temp_extract_dir);
        return Err(io::Error::other("Failed to extract package via tar"));
    }

    let temp_exe = temp_extract_dir.join("agy-orchestrator");
    let temp_public = temp_extract_dir.join("public");

    if !temp_exe.exists() {
        let _ = fs::remove_dir_all(&temp_extract_dir);
        return Err(io::Error::other("Package does not contain agy-orchestrator binary"));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_exe)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_exe, perms)?;
    }

    println!("Performing sanity checks on the downloaded binary...");
    let sanity_status = Command::new(&temp_exe)
        .arg("--help")
        .env_remove("PORT")
        .env_remove("ADDR")
        .env_remove("IP")
        .env_remove("DIOXUS_ACTIVE")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match sanity_status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("SANITY CHECK FAILED: exit status {:?}", status);
            let _ = fs::remove_dir_all(&temp_extract_dir);
            return Err(io::Error::other(format!("Downloaded binary failed basic sanity check --help: {:?}", status)));
        }
        Err(e) => {
            eprintln!("SANITY CHECK FAILED to start: {}", e);
            let _ = fs::remove_dir_all(&temp_extract_dir);
            return Err(io::Error::other(format!("Downloaded binary failed basic sanity check --help to start: {}", e)));
        }
    }

    let backup_exe = current_exe.with_extension("bak");
    println!("Backing up active binary to {}...", backup_exe.display());
    if backup_exe.exists() {
        fs::remove_file(&backup_exe)?;
    }
    fs::copy(&current_exe, &backup_exe)?;

    let active_public = parent_dir.join("public");
    let backup_public = parent_dir.join("public.bak");
    if active_public.exists() {
        if backup_public.exists() {
            fs::remove_dir_all(&backup_public)?;
        }
        let _ = fs::rename(&active_public, &backup_public);
    }

    println!("Installing upgraded binary and assets...");
    let old_exe = current_exe.with_extension("old");
    if old_exe.exists() {
        fs::remove_file(&old_exe)?;
    }
    
    let _ = fs::rename(&current_exe, &old_exe);
    if let Err(e) = fs::copy(&temp_exe, &current_exe) {
        eprintln!("Failed to copy upgraded binary: {}", e);
        println!("Restoring stable backup...");
        let _ = fs::rename(&old_exe, &current_exe);
        if backup_public.exists() {
            let _ = fs::remove_dir_all(&active_public);
            let _ = fs::rename(&backup_public, &active_public);
        }
        let _ = fs::remove_file(&backup_exe);
        let _ = fs::remove_dir_all(&temp_extract_dir);
        return Err(e);
    }

    if temp_public.exists() {
        if active_public.exists() {
            let _ = fs::remove_dir_all(&active_public);
        }
        if let Err(e) = fs::create_dir_all(&active_public) {
            eprintln!("Failed to create public directory: {}", e);
        }
        let copy_status = Command::new("cp")
            .arg("-r")
            .arg(format!("{}/.", temp_public.to_string_lossy()))
            .arg(&active_public)
            .status();
        if let Err(e) = copy_status {
            eprintln!("Failed to copy public assets: {}", e);
        }
    }

    let _ = fs::remove_file(&old_exe);
    if backup_public.exists() {
        let _ = fs::remove_dir_all(&backup_public);
    }
    let _ = fs::remove_dir_all(&temp_extract_dir);

    let daemon_was_running = is_daemon_running();
    let old_pid = get_daemon_pid();

    if daemon_was_running {
        println!("Stopping currently running daemon (PID: {:?})...", old_pid);
        let stop_status = Command::new(&backup_exe)
            .arg("daemon")
            .arg("--stop")
            .env_remove("PORT")
            .env_remove("ADDR")
            .env_remove("IP")
            .env_remove("DIOXUS_ACTIVE")
            .status()?;
        if !stop_status.success() {
            eprintln!("Warning: Failed to cleanly stop old daemon process.");
        }

        println!("Starting upgraded daemon...");
        if let Err(e) = restart_daemon_process(&current_exe) {
            println!("Launch failed, rolling back daemon...");
            let _ = fs::remove_file(&current_exe);
            let _ = fs::rename(&backup_exe, &current_exe);
            let _ = restart_daemon_process(&current_exe);
            return Err(io::Error::other(format!("Failed to launch upgraded daemon, rolled back: {}", e)));
        }

        println!("Waiting 3 seconds for health check...");
        std::thread::sleep(std::time::Duration::from_secs(3));

        if !is_daemon_running() {
            println!("Launch crashed, rolling back daemon...");
            let _ = fs::remove_file(&current_exe);
            let _ = fs::rename(&backup_exe, &current_exe);
            let _ = restart_daemon_process(&current_exe);
            return Err(io::Error::other("Upgraded daemon crashed immediately, rolled back"));
        }

        println!("Upgraded daemon is healthy (PID: {:?}).", get_daemon_pid());
    }

    if backup_exe.exists() {
        let _ = fs::remove_file(&backup_exe);
    }

    println!("Successfully upgraded to the new release!");
    Ok(())
}

pub fn run_evolution_harness(issue_id: u32) -> io::Result<()> {
    let workspace_root = match find_workspace_root() {
        Ok(root) => root,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Evolution harness is only available in a git workspace: {}", e)
            ));
        }
    };
    println!("Evolution Harness: Working directory: {}", workspace_root.display());

    let rollback_and_fail = |reason: &str| -> io::Result<()> {
        let logs_dir = get_base_dir().join("logs");
        let _ = fs::create_dir_all(&logs_dir);
        let failed_log_path = logs_dir.join(format!("evolution_failed_issue_{}.log", issue_id));

        let diff_content = Command::new("git")
            .arg("diff")
            .current_dir(&workspace_root)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_else(|_| "Failed to collect git diff".to_string());

        if let Ok(mut f) = File::create(&failed_log_path) {
            let _ = writeln!(f, "==================================================");
            let _ = writeln!(f, "🚨 EVOLUTION HARNESS FAILURE DIAGNOSTICS");
            let _ = writeln!(f, "Issue ID     : #{}", issue_id);
            let _ = writeln!(f, "Failure Time : {}", Local::now().to_rfc3339());
            let _ = writeln!(f, "Reason       : {}", reason);
            let _ = writeln!(f, "--------------------------------------------------");
            let _ = writeln!(f, "WORK SPACE DIFF AT FAILURE:");
            let _ = writeln!(f, "{}", diff_content);
            let _ = writeln!(f, "==================================================");
        }

        eprintln!(
            "HARNESS FAILURE: {}. Detailed diagnostics saved to {}. Initiating rollback...",
            reason,
            failed_log_path.display()
        );

        let mut issues = load_issues();
        if let Some(issue) = issues.iter_mut().find(|i| i.id == issue_id) {
            issue.status = "failed".to_string();
            let _ = save_issues(&issues);
        }
        let _ = Command::new("git").arg("reset").arg("--hard").arg("HEAD").current_dir(&workspace_root).status();
        let _ = Command::new("git").arg("clean").arg("-fd").current_dir(&workspace_root).status();
        Err(io::Error::other(format!("Evolution Harness rejected changes: {}", reason)))
    };

    // 0. Static Integrity Check (Comment Preservation Gate)
    println!("Harness Step 0: Checking static integrity (comment preservation)...");
    // Stage all changes temporarily to include untracked/new files in the diff
    let _ = Command::new("git").arg("add").arg(".").current_dir(&workspace_root).status();
    if let Ok(diff_output) = Command::new("git")
        .arg("diff")
        .arg("--cached")
        .current_dir(&workspace_root)
        .output()
    {
        let diff_str = String::from_utf8_lossy(&diff_output.stdout);
        let removed_comments = diff_str.lines()
            .filter(|line| line.starts_with("-") && (line.contains("//") || line.contains("///") || line.contains("/*")))
            .count();
        let added_comments = diff_str.lines()
            .filter(|line| line.starts_with("+") && (line.contains("//") || line.contains("///") || line.contains("/*")))
            .count();
        // Unstage to let git reset clean up if we rollback
        let _ = Command::new("git").arg("reset").current_dir(&workspace_root).status();

        if removed_comments > 10 && added_comments * 2 < removed_comments {
            return rollback_and_fail("Static Integrity Violation: Too many code comments were deleted without replacement.");
        }
    }
    println!("Static integrity checks passed successfully!");

    // 1. Run cargo clippy (Lint Gate)
    println!("Harness Step 1: Running cargo clippy --all-targets -- -D warnings...");
    let clippy_status = Command::new("cargo")
        .arg("clippy")
        .arg("--all-targets")
        .arg("--")
        .arg("-D")
        .arg("warnings")
        .current_dir(&workspace_root)
        .status()?;

    if !clippy_status.success() {
        return rollback_and_fail("Clippy warnings or compiler issues detected");
    }
    println!("Clippy checks passed successfully!");

    // 2. Run cargo test (Test Gate)
    println!("Harness Step 2: Running cargo test...");
    let test_status = Command::new("cargo")
        .arg("test")
        .current_dir(&workspace_root)
        .status()?;

    if !test_status.success() {
        return rollback_and_fail("Unit tests failed");
    }
    println!("All unit tests passed successfully!");

    // 3. Promote & Push changes to Remote (Success Gate)
    let mut issues = load_issues();
    if let Some(issue) = issues.iter_mut().find(|i| i.id == issue_id) {
        let issue_title = issue.title.clone();
        let issue_body = issue.body.clone();
        issue.status = "resolved".to_string();
        issue.resolved_at = Some(Local::now().to_rfc3339());
        save_issues(&issues)?;

        if let Some(start_idx) = issue_body.find("<!-- github_issue_url: ") {
            let rest = &issue_body[start_idx + "<!-- github_issue_url: ".len()..];
            if let Some(end_idx) = rest.find(" -->") {
                let github_url = &rest[..end_idx];
                if let Err(e) = close_github_issue(github_url) {
                    eprintln!("Warning: Failed to auto-close GitHub issue {}: {}", github_url, e);
                }
            }
        }

        println!("Harness Step 3: Staging and committing changes to Git...");
        let _ = Command::new("git").arg("add").arg(".").current_dir(&workspace_root).status();
        let commit_msg = format!("Auto-evolution: Resolves Issue #{}: {}", issue_id, issue_title);
        let _ = Command::new("git").arg("commit").arg("-m").arg(&commit_msg).current_dir(&workspace_root).status();

        if let Ok(output) = Command::new("git").arg("remote").current_dir(&workspace_root).output() {
            let remote_str = String::from_utf8_lossy(&output.stdout);
            if !remote_str.trim().is_empty() {
                println!("Pushing changes to remote...");
                let _ = Command::new("git").arg("push").current_dir(&workspace_root).status();
            }
        }
        println!("Successfully pushed changes and marked issue #{} as resolved!", issue_id);
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Issue #{} not found in issues.json", issue_id)
        ));
    }

    Ok(())
}
