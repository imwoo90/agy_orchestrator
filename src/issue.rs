use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Issue {
    pub id: u32,
    pub title: String,
    pub body: String,
    pub status: String, // "open", "in-progress", "resolved", "failed"
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::io;
#[cfg(not(target_arch = "wasm32"))]
use crate::vault::get_base_dir;

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
pub fn save_issues(issues: &[Issue]) -> io::Result<()> {
    let path = get_base_dir().join("issues.json");
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, issues)?;
    Ok(())
}
