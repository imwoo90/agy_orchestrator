use std::io;
use crate::backend::cli::{Commands, CliResult};

pub mod spawn;
pub mod project;
pub mod memory;
pub mod daemon;
pub mod upgrade;
pub mod issue;
pub mod skill;
pub mod harness;
pub mod utils;

pub fn execute(command: Commands) -> io::Result<CliResult> {
    match command {
        Commands::Spawn { name, path, goal } => {
            spawn::execute(name, path, goal)
        }
        Commands::List => {
            project::execute_list()
        }
        Commands::Status { name } => {
            project::execute_status(name)
        }
        Commands::GetContext { name } => {
            project::execute_get_context(name)
        }
        Commands::Consolidate { name } => {
            project::execute_consolidate(name)
        }
        Commands::QueryMemory { query } => {
            memory::execute_query_memory(query)
        }
        Commands::UpdateMemory { topic, content } => {
            memory::execute_update_memory(topic, content)
        }
        Commands::InjectMemory { project: proj, query } => {
            memory::execute_inject_memory(proj, query)
        }
        Commands::Daemon { start, stop, status, run } => {
            daemon::execute(start, stop, status, run)
        }
        Commands::SelfUpgrade { resolve_issue, remote } => {
            upgrade::execute(resolve_issue, remote)
        }
        Commands::Issue { create, body, list, resolve } => {
            issue::execute(create, body, list, resolve)
        }
        Commands::Dashboard { port } => {
            Ok(CliResult::StartDashboard { port })
        }
        Commands::HealthCheck => {
            project::execute_health_check()
        }
        Commands::LoadSkill { name } => {
            skill::execute_load_skill(name)
        }
        Commands::LearnSkill { name, description, content } => {
            skill::execute_learn_skill(name, description, content)
        }
        Commands::Compress { name } => {
            utils::execute_compress(name)
        }
        Commands::SearchHistory { name, query } => {
            utils::execute_search_history(name, query)
        }
        Commands::Delegate { parent, subtask, goal } => {
            utils::execute_delegate(parent, subtask, goal)
        }
        Commands::EvolutionHarness { issue_id } => {
            harness::execute(issue_id)
        }
        Commands::Info => {
            utils::execute_info()
        }
    }
}
