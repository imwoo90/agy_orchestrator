fn main() {
    let version = env!("CARGO_PKG_VERSION");
    let github_actions = std::env::var("GITHUB_ACTIONS").is_ok();
    
    let display_version = if github_actions {
        version.to_string()
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
        let dev_num_file = std::path::PathBuf::from(home).join(".agy_orchestrator/dev_build_number");
        let mut count = 0;
        
        if dev_num_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&dev_num_file) {
                let parts: Vec<&str> = content.trim().split(':').collect();
                if parts.len() == 2 {
                    let file_version = parts[0];
                    if file_version == version {
                        if let Ok(val) = parts[1].parse::<u32>() {
                            count = val + 1;
                        }
                    }
                } else {
                    // Legacy migration: if file just contains a raw number
                    if let Ok(val) = content.trim().parse::<u32>() {
                        count = val + 1;
                    }
                }
            }
        }
        let _ = std::fs::create_dir_all(dev_num_file.parent().unwrap());
        let _ = std::fs::write(&dev_num_file, format!("{}:{}", version, count));

        format!("{}-dev{}", version, count)
    };
    
    let commit_hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=AGY_ORCHESTRATOR_VERSION={}", display_version);
    println!("cargo:rustc-env=AGY_ORCHESTRATOR_COMMIT_HASH={}", commit_hash);
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/");
}
