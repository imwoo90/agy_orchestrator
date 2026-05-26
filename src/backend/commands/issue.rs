use std::io;
use chrono::Local;

use crate::models::Issue;
use crate::backend::issue::{load_issues, save_issues, sync_github_issues};
use crate::backend::cli::CliResult;

pub fn execute(create: Option<String>, body: Option<String>, list: bool, resolve: Option<u32>, sync: bool) -> io::Result<CliResult> {
    let mut issues = load_issues();
    let mut performed_action = false;

    if sync {
        println!("Syncing issues from remote GitHub repository...");
        match sync_github_issues() {
            Ok(_) => {
                println!("Successfully synced issues from GitHub.");
                issues = load_issues();
            }
            Err(e) => eprintln!("Error syncing issues from GitHub: {}", e),
        }
        performed_action = true;
    }

    if list {
        if issues.is_empty() {
            println!("No registered issues found.");
        } else {
            println!("{:<5} | {:<25} | {:<12} | {:<20} | Body", "ID", "Title", "Status", "Created At");
            println!("{}", "-".repeat(95));
            for issue in &issues {
                let created = issue.created_at.get(..19).unwrap_or(&issue.created_at).replace('T', " ");
                let body_truncated = if issue.body.len() > 30 {
                    format!("{}...", &issue.body[..27])
                } else {
                    issue.body.clone()
                };
                println!(
                    "{:<5} | {:<25} | {:<12} | {:<20} | {}",
                    issue.id, issue.title, issue.status, created, body_truncated
                );
            }
        }
        performed_action = true;
    } else if let Some(title) = create {
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
        save_issues(&issues)?;
        println!("Successfully registered issue #{} '{}'.", next_id, title);
        performed_action = true;
    } else if let Some(id) = resolve {
        if let Some(issue) = issues.iter_mut().find(|i| i.id == id) {
            issue.status = "resolved".to_string();
            issue.resolved_at = Some(Local::now().to_rfc3339());
            save_issues(&issues)?;
            println!("Successfully marked issue #{} as resolved.", id);
        } else {
            eprintln!("Error: Issue #{} not found.", id);
            std::process::exit(1);
        }
        performed_action = true;
    }

    if !performed_action {
        println!("Please specify --create, --list, --resolve, or --sync.");
    }
    Ok(CliResult::Exit)
}
