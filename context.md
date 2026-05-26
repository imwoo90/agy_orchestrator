# Project Context: agy-orchestrator

JIT Memory Agent Orchestrator & Knowledge Vault for AI coding assistants.

## Architecture Overview
- **Backend (Rust)**: CLI subcommands (`src/backend/commands/`), background daemon, health checks, and self-evolution safety harness.
- **Frontend (Dioxus)**: Fullstack dashboard displaying projects state, logs, tasks, vault, and interactive chat secretary.
- **Persistent Service**: Configured via systemd user service (`agy-orchestrator.service`).

## System Features
- **Daemon Loop**: Handles status monitoring, report consolidation, log compression, task running, and updates.
- **OTA Self-Upgrade**: Downloads/recompiles binary, updates public assets, restarts daemon, and automatically restarts dashboard web process on its active port using detached `setsid()`.
- **Auto-Incrementing Dev Version**: Tracks local dev compiles at `~/.agy_orchestrator/dev_build_number` and appends `-dev<Count>` suffix for clear version tracking. Enforces recompilation on source folder edits.
- **Evolution Harness**: Validates edits against static integrity gates, clippy warnings (`-D warnings`), and test suites before committing/resolving issues.
- **Premium Chat Assistant**: Glassmorphic, highly polished UI tab integrated with `agy` CLI using session tracking. Supports:
  - Custom pure-Rust Markdown & code block parser/renderer with interactive copy buttons.
  - Quick action chips for JIT system queries (info, list, issues, create task).
  - Header controls to reset conversation sessions.
  - Multi-Room Chat Session Management: Switch, create, and delete individual rooms with first-message auto-naming. Auto-promotes missing/empty UUID session folders safely.
  - Isolated UX states: HashMap-based loading flags per room and active session filtering to prevent cross-room message bleed-through.

## Project Playbook (AGENTS.md)
Rules for AI developers:
1. Run Clippy & tests via `agy-orchestrator evolution-harness` to verify logic.
2. Separate CLI subcommands into [src/backend/commands/](file:///home/wimvm/works/agy_orchestrator/src/backend/commands/).
3. Do not drop comments or simplify documentation.

## TODO / Future Work
- `[ ]` Add real-time log streaming for active projects in dashboard.
- `[ ]` Support multiple registered developers/workspaces simultaneously.
