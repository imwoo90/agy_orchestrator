# Project Context: agy-orchestrator

JIT Memory Agent Orchestrator & Knowledge Vault for AI coding assistants.

## Architecture Overview
- **Backend (Rust)**: Manages background monitoring daemon, CLI subcommands, health checks, self-upgrades, and evolution issues.
- **Frontend (Dioxus)**: Web-based fullstack dashboard serving projects state, logs, issues, and configuration.
- **Persistent Service**: Automatically configured via standard `systemd` user service (`agy-orchestrator.service`) during installation.
- **Self-Evolution**: Active git developer workspaces can spawn evolution tasks, self-verify using Clippy/cargo-test harness, and auto-release.

## System CLI State (v0.1.13)
- **Daemon Loop**: Handles process status, auto-consolidation of reports, log auto-compression, issues execution, and GitHub version checks.
- **Upgrade Process**: Automatically downloads, extracts, and deploys `tar.gz` package containing binary and frontend `public` assets, restarts the systemd service smoothly, and auto-closes resolved remote GitHub issues using the `GITHUB_TOKEN`.
- **Upgrade Diagnostics**: Displays real-time visual progress monitoring (downloading, installing, restarting) and error diagnostics in the dashboard modal with automatic browser reload on success.
- **Dashboard Support**: Detects active dashboard status (PID/Port) and automatically spawns a new upgraded instance, terminating the old one seamlessly during remote updates.
- **Info Command**: Reports Version, Execution Mode, Daemon Status (with PID), and Dashboard Status (with PID and Port).
- **Modular Subcommands**: Single-responsibility Rust commands under `src/backend/commands/` managed via entrypoint routing in `mod.rs`.
- **Interactive Kanban Board**: Allows triggering self-evolution harness (`run_evolution_harness_fn`) and manual resolution (`resolve_issue_fn`) directly from the web client, piping harness output to `Live Logs`.
- **Modular Frontend**: Exposes individual component tabs under `src/frontend/components/` for projects, issues, vault, and logs, improving AI readability and clean code structure.
- **GitHub Issues Integration**: Syncs remote open issues labeled `evolution` into local `issues.json` automatically via daemon polling or manually via `issue --sync` CLI flag.

