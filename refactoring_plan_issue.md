# Refactoring Plan: Issue Command Modernization

This document outlines the detailed refactoring plan for `src/backend/commands/issue.rs` in order to improve modularity, simplify maintenance, extract formatting logic, and replace the process exit code smell with standard idiomatic Rust error handling.

---

## 1. Objectives & Architectural Decisions

* **Split Action Logic**: Extract separate logical branches (`list`, `create`, `resolve`, and `sync`) into individual helper functions to improve readability and isolation of concerns.
* **Extract Presentation and Formatting Details**: Extract timestamp parsing, body truncation, and table rendering into dedicated formatting helpers to simplify CLI display adjustments.
* **Fix process-level Code Smell**: Replace standard `std::process::exit(1)` with standard Rust error handling by returning `std::io::Error`. This allows the caller (like CLI entrypoint or API controllers) to handle failures cleanly and improves unit-testability.

---

## 2. Refactoring Outline & Helper Signatures

### 2.1. Formatting & Presentation Helpers

We will extract the details of formatting issue attributes and table rendering into pure helper functions:

```rust
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
```

### 2.2. Command-Action Helpers

We will encapsulate command logic in dedicated helper functions:

#### A. Syncing GitHub Issues
```rust
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
```

#### B. Listing Registered Issues
```rust
/// Handles the tabular listing of registered issues.
pub fn handle_list(issues: &[Issue]) -> io::Result<()> {
    render_issues_table(issues);
    Ok(())
}
```

#### C. Creating a New Issue
```rust
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
```

#### D. Resolving an Issue
```rust
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
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Error: Issue #{} not found.", id),
        ))
    }
}
```

---

## 3. Simplified Main Execute Function

The main `execute` entrypoint will be simplified to a clean orchestrating routine:

```rust
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
```

---

## 4. Verification & Testing Strategy

1. **Verify No Regression**:
   Verify that existing build patterns and dependencies compile successfully.
2. **Clippy Compliance**:
   Run `cargo clippy --all-targets -- -D warnings` to ensure there are no style or syntax compiler errors.
3. **Evolution-Harness Validation**:
   Validate via `agy-orchestrator evolution-harness` to ensure all structural tests and integrity constraints pass.
