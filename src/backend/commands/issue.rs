use std::io;
use chrono::Local;

use crate::models::Issue;
use crate::backend::issue::{load_issues, save_issues, sync_github_issues};
use crate::backend::cli::CliResult;

/// Formats the RFC3339 timestamp of an issue for CLI tabular display.
///
/// Converts "YYYY-MM-DDT..."" to "YYYY-MM-DD HH:MM:SS" (or similar prefix).
pub fn format_created_at(created_at: &str) -> String {
    created_at.get(..19).unwrap_or(created_at).replace('T', " ")
}

/// Truncates the issue body to fit inside the CLI table, appending "..." if needed.
pub fn truncate_body(body: &str, limit: usize) -> String {
    let char_count = body.chars().count();
    if char_count > limit {
        let sliced: String = body.chars().take(limit - 3).collect();
        format!("{}...", sliced)
    } else {
        body.to_string()
    }
}

/// Tabulates and renders the list of registered issues to standard output.
pub fn render_issues_table(issues: &[Issue]) {
    if issues.is_empty() {
        println!("No registered issues found.");
    } else {
        println!("{:<5} | {:<25} | {:<12} | {:<20} | Body", "ID", "Title", "Status", "Created At");
        println!("{}", "-".repeat(95));
        for issue in issues {
            let created = format_created_at(&issue.created_at);
            let body_truncated = truncate_body(&issue.body, 30);
            println!(
                "{:<5} | {:<25} | {:<12} | {:<20} | {}",
                issue.id, issue.title, issue.status, created, body_truncated
            );
        }
    }
}

/// Triggers GitHub synchronization and re-loads the latest list of issues.
pub fn handle_sync() -> io::Result<Vec<Issue>> {
    println!("Syncing issues from remote GitHub repository...");
    match sync_github_issues() {
        Ok(_) => {
            println!("Successfully synced issues from GitHub.");
            Ok(load_issues())
        }
        Err(e) => {
            eprintln!("Error syncing issues from GitHub: {}", e);
            // Return loaded issues even on sync failure to allow subsequent commands to work
            Ok(load_issues())
        }
    }
}

/// Handles the tabular listing of registered issues.
pub fn handle_list(issues: &[Issue]) -> io::Result<()> {
    render_issues_table(issues);
    Ok(())
}

/// Handles the creation and registration of a new issue.
pub fn handle_create(issues: &mut Vec<Issue>, title: String, body: Option<String>) -> io::Result<()> {
    let next_id = issues.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    let body_str = body.unwrap_or_default();
    let new_issue = Issue {
        id: next_id,
        title: title.clone(),
        body: body_str,
        status: "open".to_string(),
        created_at: Local::now().to_rfc3339(),
        resolved_at: None,
    };
    issues.push(new_issue);
    save_issues(issues)?;
    println!("Successfully registered issue #{} '{}'.", next_id, title);
    Ok(())
}

/// Handles marking an issue as resolved.
///
/// Returns an `io::Error` with kind `NotFound` if the specified issue does not exist,
/// eliminating the `std::process::exit(1)` code smell.
pub fn handle_resolve(issues: &mut [Issue], id: u32) -> io::Result<()> {
    if let Some(issue) = issues.iter_mut().find(|i| i.id == id) {
        issue.status = "resolved".to_string();
        issue.resolved_at = Some(Local::now().to_rfc3339());
        save_issues(issues)?;
        println!("Successfully marked issue #{} as resolved.", id);
        Ok(())
    } else {
        eprintln!("Error: Issue #{} not found.", id);
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Error: Issue #{} not found.", id),
        ))
    }
}

pub fn execute(create: Option<String>, body: Option<String>, list: bool, resolve: Option<u32>, sync: bool) -> io::Result<CliResult> {
    let mut issues = load_issues();
    let mut performed_action = false;

    if sync {
        issues = handle_sync()?;
        performed_action = true;
    }

    if list {
        handle_list(&issues)?;
        performed_action = true;
    } else if let Some(title) = create {
        handle_create(&mut issues, title, body)?;
        performed_action = true;
    } else if let Some(id) = resolve {
        handle_resolve(&mut issues, id)?;
        performed_action = true;
    }

    if !performed_action {
        println!("Please specify --create, --list, --resolve, or --sync.");
    }
    Ok(CliResult::Exit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::env;

    fn setup_test_env() -> (std::path::PathBuf, Vec<Issue>) {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let test_home = std::path::PathBuf::from(manifest_dir)
            .join("target")
            .join("test_home_issue");
        fs::create_dir_all(test_home.join(".agy_orchestrator")).unwrap();
        env::set_var("HOME", &test_home);

        let issues = vec![
            Issue {
                id: 1,
                title: "Test Issue 1".to_string(),
                body: "Body 1".to_string(),
                status: "open".to_string(),
                created_at: "2026-05-28T21:48:38+09:00".to_string(),
                resolved_at: None,
            },
            Issue {
                id: 2,
                title: "Test Issue 2".to_string(),
                body: "Body 2".to_string(),
                status: "open".to_string(),
                created_at: "2026-05-28T21:48:38+09:00".to_string(),
                resolved_at: None,
            },
        ];
        (test_home, issues)
    }

    #[test]
    fn test_format_created_at() {
        assert_eq!(format_created_at("2026-05-28T21:48:38+09:00"), "2026-05-28 21:48:38");
        assert_eq!(format_created_at("invalid"), "invalid");
    }

    #[test]
    fn test_truncate_body() {
        assert_eq!(truncate_body("Hello World", 20), "Hello World");
        assert_eq!(truncate_body("This is a very long body that needs truncation", 15), "This is a ve...");
    }

    #[test]
    fn test_handle_resolve_success() {
        let _lock = crate::backend::vault::TEST_MUTEX.lock().unwrap();
        let (_test_home, mut issues) = setup_test_env();
        let res = handle_resolve(&mut issues, 1);
        assert!(res.is_ok());
        assert_eq!(issues[0].status, "resolved");
        assert!(issues[0].resolved_at.is_some());
    }

    #[test]
    fn test_handle_resolve_not_found() {
        let _lock = crate::backend::vault::TEST_MUTEX.lock().unwrap();
        let (_test_home, mut issues) = setup_test_env();
        let res = handle_resolve(&mut issues, 99);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), io::ErrorKind::NotFound);
    }
}

