use std::io;

use crate::backend::upgrade::run_evolution_harness;
use crate::backend::cli::CliResult;

pub fn execute(issue_id: u32) -> io::Result<CliResult> {
    run_evolution_harness(issue_id)?;
    Ok(CliResult::Exit)
}
