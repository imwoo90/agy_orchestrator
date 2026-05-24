use crate::frontend::app::Issue;
use std::fs::File;
use std::io;
use serde::Deserialize;
use super::vault::get_base_dir;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct GithubIssue {
    number: u32,
    title: String,
    body: Option<String>,
    state: String,
    created_at: String,
    html_url: String,
}

pub fn load_issues() -> Vec<Issue> {
    let path = get_base_dir().join("issues.json");
    if !path.exists() {
        return Vec::new();
    }
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    serde_json::from_reader(file).unwrap_or_else(|_| Vec::new())
}

pub fn save_issues(issues: &[Issue]) -> io::Result<()> {
    let path = get_base_dir().join("issues.json");
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, issues)?;
    Ok(())
}

pub fn sync_github_issues() -> io::Result<()> {
    let mut cmd = std::process::Command::new("curl");
    cmd.arg("-s")
       .arg("-H")
       .arg("User-Agent: agy-orchestrator");
    
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            cmd.arg("-H").arg(format!("Authorization: Bearer {}", token.trim()));
        }
    }
    
    cmd.arg("https://api.github.com/repos/imwoo90/agy_orchestrator/issues?labels=evolution&state=open");
    
    let output = cmd.output()?;
    if !output.status.success() {
        return Err(io::Error::other("Failed to run curl to fetch github issues"));
    }
    
    let github_issues: Vec<GithubIssue> = match serde_json::from_slice(&output.stdout) {
        Ok(issues) => issues,
        Err(e) => {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Failed to parse GitHub API response: {}", e)));
        }
    };
    
    let mut local_issues = load_issues();
    let mut changed = false;
    
    let mut next_id = local_issues.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    
    for gh in github_issues {
        let is_dup = local_issues.iter().any(|li| {
            li.body.contains(&gh.html_url) || li.body.contains(&format!("github_issue_url: {}", gh.html_url))
        });
        
        if is_dup {
            continue;
        }
        
        let body_ref = format!("{}\n\n<!-- github_issue_url: {} -->", gh.body.unwrap_or_default(), gh.html_url);
        
        let new_issue = Issue {
            id: next_id,
            title: gh.title,
            body: body_ref,
            status: "open".to_string(),
            created_at: gh.created_at,
            resolved_at: None,
        };
        
        local_issues.push(new_issue);
        next_id += 1;
        changed = true;
    }
    
    if changed {
        save_issues(&local_issues)?;
    }
    
    Ok(())
}

pub fn close_github_issue(issue_url: &str) -> io::Result<()> {
    let token = match std::env::var("GITHUB_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            println!("GITHUB_TOKEN environment variable not set or empty. Skipping remote issue closing.");
            return Ok(());
        }
    };

    let prefix = "https://github.com/";
    if !issue_url.starts_with(prefix) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid GitHub issue URL"));
    }
    
    let path = &issue_url[prefix.len()..];
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 4 || parts[2] != "issues" {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid GitHub issue URL structure"));
    }
    
    let owner = parts[0];
    let repo = parts[1];
    let issue_number = parts[3];
    
    let api_url = format!("https://api.github.com/repos/{}/{}/issues/{}", owner, repo, issue_number);
    
    let mut cmd = std::process::Command::new("curl");
    cmd.arg("-X").arg("PATCH")
       .arg("-s")
       .arg("-H").arg("Accept: application/vnd.github+json")
       .arg("-H").arg(format!("Authorization: Bearer {}", token))
       .arg("-H").arg("User-Agent: agy-orchestrator")
       .arg("-H").arg("Content-Type: application/json")
       .arg(&api_url)
       .arg("-d").arg(r#"{"state":"closed"}"#);
       
    let output = cmd.output()?;
    if !output.status.success() {
        return Err(io::Error::other("Failed to run curl PATCH to close issue"));
    }
    
    println!("Successfully closed remote GitHub issue: {}", issue_url);
    Ok(())
}
