use std::io;

use crate::backend::upgrade::run_self_upgrade;
use crate::backend::cli::CliResult;

pub fn execute(resolve_issue: Option<u32>, remote: bool) -> io::Result<CliResult> {
    if remote {
        println!("Checking for latest release on GitHub...");
        match crate::backend::upgrade::check_latest_release() {
            Ok(Some((tag, url))) => {
                println!("Found new version '{}'!", tag);
                crate::backend::upgrade::run_remote_upgrade(&url)?;
            }
            Ok(None) => {
                println!("You are already running the latest version.");
            }
            Err(e) => {
                eprintln!("Error checking latest release: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        run_self_upgrade(resolve_issue)?;
    }
    Ok(CliResult::Exit)
}
