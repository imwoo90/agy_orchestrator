#![cfg(not(target_arch = "wasm32"))]

use clap::{Parser, Subcommand};
use std::io;

use super::vault::bootstrap_if_needed;

#[derive(Parser, Debug)]
#[command(name = "orchestrate")]
#[command(about = "JIT Memory Agent Orchestrator & Knowledge Vault", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Spawn a new project task in the background
    Spawn {
        #[arg(long)]
        name: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        goal: String,
    },
    /// List all registered projects and their status
    List,
    /// Get status and show report for a specific project
    Status {
        #[arg(long)]
        name: String,
    },
    /// Fetch the target project's path and local context for JIT load
    GetContext {
        #[arg(long)]
        name: String,
    },
    /// Consolidate the report.md into context.md and update project status
    Consolidate {
        #[arg(long)]
        name: String,
    },
    /// Search the personal knowledge vault for relevant context notes
    QueryMemory {
        #[arg(long)]
        query: String,
    },
    /// Create or update a note in the personal knowledge vault
    UpdateMemory {
        #[arg(long)]
        topic: String,
        #[arg(long)]
        content: String,
    },
    /// Inject a knowledge note from the vault into a project's context.md
    InjectMemory {
        #[arg(long)]
        project: String,
        #[arg(long)]
        query: String,
    },
    /// Manage the background orchestrator daemon
    Daemon {
        /// Start the daemon in the background
        #[arg(long)]
        start: bool,
        /// Stop the running background daemon
        #[arg(long)]
        stop: bool,
        /// Check if the daemon is currently running
        #[arg(long)]
        status: bool,
        /// Run the daemon in the foreground (blocking loop)
        #[arg(long)]
        run: bool,
    },
    /// Test, compile, and hot-reload/upgrade the orchestrator binary and daemon
    SelfUpgrade {
        /// Resolve a specific issue ID upon successful upgrade
        #[arg(long)]
        resolve_issue: Option<u32>,
        /// Upgrade from the latest remote GitHub release instead of local compilation
        #[arg(long)]
        remote: bool,
    },
    /// Manage and register self-evolution issues
    Issue {
        /// Create a new issue with a title
        #[arg(long)]
        create: Option<String>,
        /// Body/details of the new issue
        #[arg(long)]
        body: Option<String>,
        /// List registered issues
        #[arg(long)]
        list: bool,
        /// Mark a specific issue as resolved by ID
        #[arg(long)]
        resolve: Option<u32>,
    },
    /// Start the embedded web dashboard server
    Dashboard {
        /// Port to bind the dashboard web server to
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// Run a proactive health check on all registered targets
    HealthCheck,
    /// Load a specific procedural skill's full details
    LoadSkill {
        #[arg(long)]
        name: String,
    },
    /// Learn and register a new procedural skill
    LearnSkill {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        content: String,
    },
    /// Compress the active execution log file for token optimization
    Compress {
        #[arg(long)]
        name: String,
    },
    /// Search the project's historical logs (context_history.md) by keyword
    SearchHistory {
        #[arg(long)]
        name: String,
        #[arg(long)]
        query: String,
    },
    /// Delegate a subtask to a sub-agent
    Delegate {
        #[arg(long)]
        parent: String,
        #[arg(long)]
        subtask: String,
        #[arg(long)]
        goal: String,
    },
    /// Run safety checks (Clippy, test) for self-evolution and auto-rollback on failure
    EvolutionHarness {
        /// The specific issue ID to validate
        #[arg(long)]
        issue_id: u32,
    },
    /// Display system information, mode, and background daemon status
    Info,
}

pub enum CliResult {
    Exit,
    StartDashboard { port: u16 },
}

pub fn run_cli(cli: Cli) -> io::Result<CliResult> {
    bootstrap_if_needed()?;
    super::commands::execute(cli.command)
}
