# 🦅 AI Agent Operating Protocol (AGENTS.md)

Welcome! If you are an AI Coding Agent entering this workspace, **you must read and adhere to this protocol at all times.** This document serves as your operating manual, context anchor, and safety harness to ensure you do not break the system, lose track of goals, or violate codebase consistency.

---

## 🧭 Entrypoint Protocol (Read This First)

Before performing any edit or analyzing files, you must run this initial diagnostic check:
1. **Check Workspace State**: Run `git status` and `git diff` to understand what state the previous agent left behind.
2. **Understand Project Architecture**: Read the high-resolution architecture guide at [docs/architecture.md](file:///home/wimvm/works/agy_orchestrator/docs/architecture.md).
3. **Initialize Instructions**: Read the following paths to load instruction rules:
   * **Static instructions**: [~/.agy_orchestrator/memory/system_instructions.md](file:///home/wimvm/.agy_orchestrator/memory/system_instructions.md)
   * **Dynamic habits/preferences**: Search Obsidian-style vault files in `~/.agy_orchestrator/memory/vault/` based on user-prompt keywords.
4. **On-Demand Context Loading**: For this workspace, the registered project name is `agy_orchestrator`. Load its active hot memory context:
   * Run: `agy-orchestrator get-context --name agy_orchestrator`
   * Read the returned `/home/wimvm/works/agy_orchestrator/context.md` file.

---

## 🛡️ Agent Harness System (Safety Gate)

We enforce a strict **Agent Harness System** to validate code changes during self-evolution. You must not commit or request deployment manually.

### The Validation Gate Loop
Whenever you complete an issue resolution or change:
1. **Run the Harness**: Run the safety validator (it has workspace detection fallback to make it safe to run from any shell):
   ```bash
   agy-orchestrator evolution-harness --issue-id <id>
   ```
2. **Static Integrity Gate**: The harness checks your changes against `HEAD`. If you deleted too many comments or documentation structures, it will reject the changes to preserve knowledge.
3. **Clippy Gate**: The harness runs `cargo clippy --all-targets -- -D warnings`. Code warnings are treated as hard compilation errors.
4. **Test Gate**: The harness runs `cargo test` to verify no regressions were introduced.
5. **Rollback Mechanism & Diagnostics**: If any check fails, the harness will:
   * Generate a detailed diagnostics log with your diff at `~/.agy_orchestrator/logs/evolution_failed_issue_<id>.log`.
   * Automatically rollback all uncommitted changes using `git reset --hard` and mark the issue as `failed`.
6. **Success Promotion**: On success, the harness will stage changes, auto-commit with issue metadata, push to remote, and mark the issue as `resolved`.

---

## ✍️ Coding Rules & Guardrails

* **Preserve Documentation**: Do not remove, alter, or simplify existing comments, docstrings, or structural markers. The Static Integrity Gate enforces this.
* **Modular Command Layout**: If you are adding or refactoring subcommands, do not inline them in `src/backend/cli.rs`. Add them as a module under [src/backend/commands/](file:///home/wimvm/works/agy_orchestrator/src/backend/commands/) and link them in [commands/mod.rs](file:///home/wimvm/works/agy_orchestrator/src/backend/commands/mod.rs).
* **Keep Dependencies Minimal**: Never add cargo dependencies (like heavy HTTP libraries) that increase compile times unless you receive explicit user approval. Utilize subprocess system commands (e.g. `curl`) where appropriate.
* **Incremental Fixes**: When encountering compile errors, do not rewrite large portions of code. Make focused, minimal edits to resolve the exact error reported by the compiler.
* **Update Habits**: If the user corrects your approach or expresses a preference, immediately record it in the vault using:
  ```bash
  agy-orchestrator update-memory --topic "<topic>" --content "<markdown>"
  ```
* **Local Dashboard Development**: To run the Dioxus dashboard locally, use Dioxus CLI (`dx serve` / `dx build`). Do not run raw `cargo run -- dashboard` because `default = ["web"]` will cause a native startup panic. If you must run via cargo directly, use:
  ```bash
  cargo run --no-default-features --features server -- dashboard
  ```
* **Tool Invocation Argument Formatting**: When executing platform tools (such as `view_file`, `list_dir`, `write_to_file`, `replace_file_content`, etc.), you MUST pass raw string paths without literal double quotes or escaped backslashes in the arguments JSON. For example, pass `"AbsolutePath": "/home/wimvm/works/agy_orchestrator/src/main.rs"`, NOT `"AbsolutePath": "\"/home/wimvm/works/agy_orchestrator/src/main.rs\""`. Double-quoted path values will cause sandbox permission validation to fail with a timeout!

