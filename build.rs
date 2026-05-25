fn main() {
    let version = env!("CARGO_PKG_VERSION");
    let github_actions = std::env::var("GITHUB_ACTIONS").is_ok();
    
    let display_version = if github_actions {
        version.to_string()
    } else {
        format!("{}-dev", version)
    };
    
    println!("cargo:rustc-env=AGY_ORCHESTRATOR_VERSION={}", display_version);
}
