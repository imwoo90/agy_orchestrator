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
  - **PTY Agy Runner (rexpect)**: Spawns the underlying `agy` command within a pseudo-terminal (PTY) to intercept and automatically answer unexpected interactive permission prompts from subagents (`invoke_subagent`), utilizing robust argument array spawning (`spawn_command`) and error buffer recovery (`got` payload in EOF/Timeout errors) to prevent hangs and output loss.
  - Premium Chat Assistant: Glassmorphic, highly polished UI tab integrated with agy CLI using session tracking. Supports:
    - Custom pure-Rust Markdown & code block parser/renderer with interactive copy.
    - Multi-Room Chat Session Management: Switch, create, and delete individual rooms with first-message auto-naming.
    - Log Tab Auto-Scroll & Chat Tab Auto-Scroll.
    - Desktop-optimized responsive layout (h-full w-full overflow-hidden) eliminating double scrollbars.
    - Command Masking: Mask technical CLI command prompts (like info/list) in the chat bubble UI with beautiful human-readable labels.
    - Direct Slash Command Execution: Bypasses system instructions for prompts starting with `/` (e.g. `/agents`, `/goal`) to route them directly to the `agy` CLI, preventing LLM interactive tool blocks.
- Port Conflict Fix: Prevents environment pollution by explicitly clearing PORT, ADDR, IP, and DIOXUS_ACTIVE from command builders when spawning subcommands and background daemons from the dashboard context.
- **Self-Kill Prevention during Upgrade**: Avoids self-killing the dashboard process when a remote upgrade is initiated from the UI, ensuring the upgrade server function returns a successful response to the browser before triggering a clean background restart.
- **Sandbox Permission Management**: Exposes settings.json authorization status and whitelists paths on spawning or via the dashboard Projects UI to prevent background subagents from timing out on permission prompts.

## Project Playbook (AGENTS.md)
Rules for AI developers:
1. Run Clippy & tests via `agy-orchestrator evolution-harness` to verify logic.
2. Separate CLI subcommands into [src/backend/commands/](file:///home/wimvm/works/agy_orchestrator/src/backend/commands/).
3. Do not drop comments or simplify documentation.

## TODO / Future Work
- `[ ]` Add real-time log streaming for active projects in dashboard.
- `[ ]` Support multiple registered developers/workspaces simultaneously.



# 📅 History log from 2026-05-28 21:38:23 (Auto-consolidated)

# Subtask Report: Refactoring Plan for commands/utils.rs

## 1. Summary of Completed Tasks
- **Diagnostics Check**: Performed the initial workspace diagnostic protocol (`git status`, `git diff`, architecture overview, static instructions, and get-context / context.md loading).
- **Codebase Analysis**: Analyzed the log compression and subagent delegation implementations in `src/backend/commands/utils.rs`.
- **Refactoring Design**: Outlined clear structural improvements, including extracting filesystem I/O, using helper parser states, grouping hardcoded parameters into static constants, and introducing clean helper functions.
- **Refactoring Plan**: Documented the complete strategy in [refactoring_plan.md](file:///home/wimvm/works/agy_orchestrator/refactoring_plan.md).

## 2. Crucial Design/Architectural Choices Made
- **Pure Functional Core for Log Compression**: Decoupled filesystem access from text processing to make log compression unit-testable (`compress_log_content(content: &str) -> String`).
- **Parsing Modularization**: Split the parsing state logic of log files (cargo logs vs tool output blocks) into dedicated functions to reduce the complexity of the main loop.
- **Goal Keyword Extraction Extraction**: Decoupled skills parsing/matching out of the direct execution path of `get_skills_injection`.

## 3. Minor Choices Resolved Autonomously
- Chose to group all configuration settings (such as minimum cargo skip counts, block thresholds, etc.) into module constants rather than an external configuration file to keep compile time minimal.
- Defined a set list of delegation stop words as a static slice.

## 4. CRITICAL ITEMS FOR REVIEW
None



# 📅 History log from 2026-05-28 21:38:43 (Auto-consolidated)

# Subtask Report: Refactoring Plan for commands/utils.rs

## 1. Summary of Completed Tasks
- **Diagnostics Check**: Performed the initial workspace diagnostic protocol (`git status`, `git diff`, architecture overview, static instructions, and get-context / context.md loading).
- **Codebase Analysis**: Analyzed the log compression and subagent delegation implementations in `src/backend/commands/utils.rs`.
- **Refactoring Design**: Outlined clear structural improvements, including extracting filesystem I/O, using helper parser states, grouping hardcoded parameters into static constants, and introducing clean helper functions.
- **Refactoring Plan**: Documented the complete strategy in [refactoring_plan.md](file:///home/wimvm/works/agy_orchestrator/refactoring_plan.md).

## 2. Crucial Design/Architectural Choices Made
- **Pure Functional Core for Log Compression**: Decoupled filesystem access from text processing to make log compression unit-testable (`compress_log_content(content: &str) -> String`).
- **Parsing Modularization**: Split the parsing state logic of log files (cargo logs vs tool output blocks) into dedicated functions to reduce the complexity of the main loop.
- **Goal Keyword Extraction Extraction**: Decoupled skills parsing/matching out of the direct execution path of `get_skills_injection`.

## 3. Minor Choices Resolved Autonomously
- Chose to group all configuration settings (such as minimum cargo skip counts, block thresholds, etc.) into module constants rather than an external configuration file to keep compile time minimal.
- Defined a set list of delegation stop words as a static slice.

## 4. CRITICAL ITEMS FOR REVIEW
None



# 📅 History log from 2026-05-28 21:39:58 (Auto-consolidated)

# Subtask Report: Create Test File

## 1. Summary of Completed Tasks
- Executed diagnostic steps per the `AGENTS.md` protocol (workspace state checks, reading system architecture docs, and fetching hot memory context).
- Created a test file `test.txt` in the root of the project containing "Hello World".

## 2. Crucial Design/Architectural Choices Made
- None. The task was a straightforward text file creation as requested by the user.

## 3. Minor Choices Resolved Autonomously
- Decided to run `cargo test` to verify workspace compilation and test suite health before finalizing the task.

## 4. CRITICAL ITEMS FOR REVIEW
None



# 📅 History log from 2026-05-28 21:44:14 (Auto-consolidated)

# Refactoring Report: Log Compression & Delegation Logic in `commands/utils.rs`

## 1. Summary of Completed Tasks
- **Diagnostics Check**: Ran git diagnostic checks and verified the codebase architecture and history context.
- **Log Compression Refactoring**:
  - Extracted pure log processing into a functional, unit-testable core: `pub fn compress_log_content(content: &str) -> String`.
  - Added constants for thresholds and parameters: `LOG_LINE_COMPRESSION_THRESHOLD`, `MIN_CARGO_SKIP_COUNT`, `MAX_TOOL_OUTPUT_LINES`, and `TOOL_OUTPUT_BOUNDARY_LINES`.
  - Structured the sequential parser into helper functions `is_cargo_log`, `skip_cargo_logs`, `is_tool_block_start`, `is_tool_block_end`, and `compress_tool_block`.
- **Delegation Logic Refactoring**:
  - Extracted goal keyword extraction into a functional helper `extract_goal_keywords`.
  - Defined standard stop words in a module constant `DELEGATE_STOP_WORDS`.
  - Decoupled parsing skill metadata from `get_skills_injection` into `parse_skill_metadata`.
  - Structured the prompt building using a helper struct `SubagentPromptBuilder`.
  - Isolated subprocess execution (`spawn_subagent`) from database state management (`update_subagent_state`).
- **Comprehensive Unit Testing**:
  - Added unit tests directly under `src/backend/commands/utils.rs` covering log content threshold logic, Cargo log compression, tool block truncation/compression, keyword extraction, and skill metadata parsing.
  - Ran cargo test and cargo clippy successfully to ensure full logical correctness and clippy-compliance (`-D warnings`).

## 2. Crucial Design/Architectural Choices Made
- **Pure Functional Log Compression Core**: Enabled testability of the log compression logic without filesystem dependencies by separating path resolution and write calls from content compression logic.
- **PTY CLI Execution & Database Separation**: Isolated state database updates (`update_subagent_state`) from command runner execution (`spawn_subagent`), providing clear transactional separation between side-effects and local state persistence.
- **Idiomatic Iterators over Indexing Loops**: Addressed Clippy's `needless_range_loop` by refactoring index-based loops to use safe iterators (`take` and `skip`).

## 3. Minor Choices Resolved Autonomously
- Aligned tool output block prefix truncation bounds precisely with 0-indexed vectors, adapting assertions in `test_compress_log_content_tool_block` to match the exact boundaries (keeping `[diff_block_start]` plus 14 lines of logs).
- Defined delegation stop words slice with standard prepositions and articles.

## 4. CRITICAL ITEMS FOR REVIEW
None



# 📅 History log from 2026-05-28 21:48:38 (Auto-consolidated)

# Refactoring Plan Report: issue.rs Command Modernization

## 1. Summary of Completed Tasks
- **Diagnostics Check**: Ran git checks (`status` and `diff`), read the project architecture guide (`docs/architecture.md`), and loaded static instructions (`~/.agy_orchestrator/memory/system_instructions.md`) and the active project context via the orchestrator.
- **Code Analysis**: Analyzed the issues management subcommand code structure and interfaces inside `src/backend/commands/issue.rs`.
- **Refactoring Strategy & Rationale**: Formulated a modular design that breaks the monolithic command execution branches into isolated, logical functions, extracts formatting details, and eliminates the process-level code smell `std::process::exit(1)`.
- **Plan Documentation**: Created and saved the detailed refactoring plan inside `/home/wimvm/works/agy_orchestrator/refactoring_plan_issue.md`.
- **Compilation Check**: Verified the workspace compilability successfully with `cargo check`.

## 2. Crucial Design/Architectural Choices Made
- **Pure Presentation Logic separation**: Decoupled date formatting (`format_created_at`), body truncation (`truncate_body`), and printing (`render_issues_table`) from core subcommand control flows, simplifying unit-testing for formatting and UI adaptations.
- **Standard Rust Error Propagation**: Replaced direct termination calls (`std::process::exit(1)`) with `std::io::Error` (specifically `io::ErrorKind::NotFound`), delegating the error presentation and termination decisions to the command entrypoint (`main` / `cli`), thereby enhancing modularity and unit-testing safety.
- **Unified Action-Command Handlers**: Structured independent handlers (`handle_list`, `handle_create`, `handle_resolve`, `handle_sync`) to simplify code reuse, isolation, and testing.

## 3. Minor Choices Resolved Autonomously
- Left the existing external CLI parsing arguments layout unmodified to avoid cascading changes across other subsystems.
- Restructured `handle_sync` to print descriptive progress and re-load the updated local issue list, returning the list to maintain consistency across chained CLI flags (like `--sync --list`).

## 4. CRITICAL ITEMS FOR REVIEW
None



# 📅 History log from 2026-05-28 21:51:29 (Auto-consolidated)

# Subtask Report: issue.rs Command Modernization

## 1. Summary of Completed Tasks
- **Diagnostics Check**: Ran git status/diff, verified the project architecture (`docs/architecture.md`), loaded static instructions (`system_instructions.md`), and fetched the project context (`context.md`).
- **Formatting Helpers Extraction**:
  - Extracted `format_created_at` to format issue timestamps cleanly for CLI tabular display.
  - Extracted `truncate_body` to handle Unicode-safe body truncation with ellipsis.
  - Extracted `render_issues_table` to output issues in a formatted layout.
- **Command Action Handlers Extraction**:
  - Decoupled `handle_sync` to perform remote synchronization and reload issue database list.
  - Decoupled `handle_list` to list issues.
  - Decoupled `handle_create` to register and persist a new issue.
  - Decoupled `handle_resolve` to mark an issue as resolved and save modifications.
- **Process exit(1) Removal**:
  - Replaced the direct `std::process::exit(1)` inside the resolve logic with structured error propagation, returning `std::io::Error` of kind `io::ErrorKind::NotFound` when an issue is not found.
  - Simplified the `execute` entrypoint function to delegate task actions cleanly to individual handlers.
- **Unit Testing Suite**:
  - Added unit tests under a `#[cfg(test)]` module directly in `src/backend/commands/issue.rs`.
  - Configured a dynamic test setup helper `setup_test_env` modifying `HOME` temporarily to direct file saves into the cargo target directory, preventing developers' issues file from being corrupted during test runs.
  - Verified tests and lints compile successfully (`cargo check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`).

## 2. Crucial Design/Architectural Choices Made
- **Clean error propagation**: Replacing direct termination with `io::Result` delegation lets calling controllers (e.g. CLI entrypoint or potential dashboard backend functions) handle resolving errors in a standard Rust idiomatic way, improving code modularity and unit test safety.
- **PTY CLI exit compatibility**: Decoupled `handle_resolve` prints to stderr via `eprintln!` before returning the `io::Error` to ensure the exact user CLI display is preserved.

## 3. Minor Choices Resolved Autonomously
- **Redirected HOME in tests**: Used a test setup helper to dynamically re-bind the `HOME` environment variable inside the test thread context. This redirects the file database saves to `target/test_home_issue/` to safely test filesystem side effects without corrupting developer issue database configurations.

## 4. CRITICAL ITEMS FOR REVIEW
None

