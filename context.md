# Project Context: agy-orchestrator

JIT Memory Agent Orchestrator & Knowledge Vault for AI coding assistants.

## Architecture Overview
- **Backend (Rust)**: CLI subcommands (`src/backend/commands/`), background daemon, health checks, and self-evolution safety harness.
- **Frontend (Dioxus)**: Fullstack dashboard displaying projects state, logs, tasks, vault, and interactive chat secretary.
- **Persistent Service**: Configured via systemd user service (`agy-orchestrator.service`).

## System Features
- **Daemon Loop**: Handles status monitoring, report consolidation, log compression, task running, and updates.
- **OTA Self-Upgrade**: Downloads/recompiles binary, updates public assets, restarts daemon, and automatically restarts dashboard web process on its active port using detached `setsid()`.
- **Auto-Incrementing Dev Version**: Tracks local dev compiles at `~/.agy_orchestrator/dev_build_number` and appends dev version suffix.
- **Evolution Harness**: Validates edits against static integrity gates, clippy warnings (`-D warnings`), and test suites before committing/resolving issues.
  - Premium Chat Assistant: Glassmorphic, highly polished UI tab integrated with agy CLI using session tracking. Supports:
    - Custom pure-Rust Markdown & code block parser/renderer with interactive copy.
    - Multi-Room Chat Session Management: Switch, create, and delete individual rooms with first-message auto-naming.
    - Log Tab Auto-Scroll & Chat Tab Auto-Scroll.
    - Desktop-optimized responsive layout (h-full w-full overflow-hidden) eliminating double scrollbars.
    - Command Masking: Mask technical CLI command prompts (like info/list) in the chat bubble UI with beautiful human-readable labels.
    - Direct Slash Command Execution: Bypasses system instructions for prompts starting with `/` (e.g. `/agents`, `/goal`) to route them directly to the `agy` CLI, preventing LLM interactive tool blocks.
- Port Conflict Fix: Prevents environment pollution by explicitly clearing PORT, ADDR, IP, and DIOXUS_ACTIVE from command builders when spawning subcommands and background daemons from the dashboard context.
- **Self-Kill Prevention during Upgrade**: Avoids self-killing the dashboard process when a remote upgrade is initiated from the UI, ensuring the upgrade server function returns a successful response to the browser before triggering a clean background restart.

## Project Playbook (AGENTS.md)
Rules for AI developers:
1. Run Clippy & tests via `agy-orchestrator evolution-harness` to verify logic.
2. Separate CLI subcommands into [src/backend/commands/](file:///home/wimvm/works/agy_orchestrator/src/backend/commands/).
3. Do not drop comments or simplify documentation.

## TODO / Future Work
- `[ ]` Add real-time log streaming for active projects in dashboard.
- `[ ]` Support multiple registered developers/workspaces simultaneously.
