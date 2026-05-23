use crate::frontend::app::Issue;
use std::fs::File;
use std::io;
use super::vault::get_base_dir;

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
