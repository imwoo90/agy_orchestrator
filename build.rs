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
                if let Ok(val) = content.trim().parse::<u32>() {
                    count = val + 1;
                }
            }
        }
        let _ = std::fs::create_dir_all(dev_num_file.parent().unwrap());
        let _ = std::fs::write(&dev_num_file, count.to_string());

        format!("{}-dev{}", version, count)
    };
    
    println!("cargo:rustc-env=AGY_ORCHESTRATOR_VERSION={}", display_version);
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/");
}
