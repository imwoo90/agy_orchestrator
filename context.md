# Project Context: agy-orchestrator

JIT Memory Agent Orchestrator & Knowledge Vault for AI coding assistants.

## Architecture Overview
- **Backend (Rust)**: Manages background monitoring daemon, CLI subcommands, health checks, self-upgrades, and evolution issues.
- **Frontend (Dioxus)**: Web-based fullstack dashboard serving projects state, logs, issues, and configuration.
- **Persistent Service**: Automatically configured via standard `systemd` user service (`agy-orchestrator.service`) during installation.
- **Self-Evolution**: Active git developer workspaces can spawn evolution tasks, self-verify using Clippy/cargo-test harness, and auto-release.

## System CLI State (v0.1.5)
- **Daemon Loop**: Handles process status, auto-consolidation of reports, log auto-compression, issues execution, and GitHub version checks.
- **Upgrade Process**: Automatically downloads, extracts, and deploys `tar.gz` package containing binary and frontend `public` assets, and restarts the systemd service smoothly.
- **Info Command**: Reports Version, Execution Mode, Daemon Status (with PID), and Dashboard Status (with PID and Port).
