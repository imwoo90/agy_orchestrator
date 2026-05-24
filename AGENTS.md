# 🦅 AI Agent Operating Protocol (AGENTS.md)

Welcome! If you are an AI Coding Agent entering this workspace, **you must read and adhere to this protocol at all times.** This document serves as your operating manual, context anchor, and safety harness to ensure you do not break the system, lose track of goals, or violate codebase consistency.

---

## 🧭 Entrypoint Protocol (Read This First)

Before performing any edit or analyzing files, you must run this initial diagnostic check:
1. **Check Workspace State**: Run `git status` and `git diff` to understand what state the previous agent left behind.
2. **Initialize Instructions**: Read the following paths to load instruction rules:
   * **Static instructions**: [~/.agy_orchestrator/memory/system_instructions.md](file:///home/wimvm/.agy_orchestrator/memory/system_instructions.md)
   * **Dynamic habits/preferences**: Search Obsidian-style vault files in `~/.agy_orchestrator/memory/vault/` based on user-prompt keywords.
3. **On-Demand Context Loading**: Do not read the entire workspace directory structure. Query specific projects when asked:
   * Run: `agy-orchestrator get-context --name <project_name>`
   * Read the returned `<project_path>/context.md` file.

---

## 🛡️ Agent Harness System (Safety Gate)

We enforce a strict **Agent Harness System** to validate code changes during self-evolution. You must not commit or request deployment manually.

### The Validation Gate Loop
Whenever you complete an issue resolution or change:
1. **Run the Harness**: Run the safety validator:
   ```bash
   agy-orchestrator evolution-harness --issue-id <id>
   ```
2. **Clippy Gate**: The harness runs `cargo clippy --all-targets -- -D warnings`. Code warnings are treated as hard compilation errors.
3. **Test Gate**: The harness runs `cargo test` to verify no regressions were introduced.
4. **Rollback Mechanism**: If any check fails, the harness will **automatically rollback all uncommitted changes** using `git reset --hard` and mark the issue as `failed`. This prevents broken code from corrupting the master workspace.
5. **Success Promotion**: On success, the harness will stage changes, auto-commit with issue metadata, push to remote, and mark the issue as `resolved`.

---

## ✍️ Coding Rules & Guardrails

* **Preserve Documentation**: Do not remove, alter, or simplify existing comments, docstrings, or structural markers unless explicitly requested.
* **Keep Dependencies Minimal**: Never add cargo dependencies (like heavy HTTP libraries) that increase compile times unless you receive explicit user approval. Utilize subprocess system commands (e.g. `curl`) where appropriate.
* **Incremental Fixes**: When encountering compile errors, do not rewrite large portions of code. Make focused, minimal edits to resolve the exact error reported by the compiler.
* **Update Habits**: If the user corrects your approach or expresses a preference, immediately record it in the vault using:
  ```bash
  agy-orchestrator update-memory --topic "<topic>" --content "<markdown>"
  ```
