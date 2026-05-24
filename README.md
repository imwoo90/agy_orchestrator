# 🗼 AGY Agentic Orchestrator (Rust Core)

> [!NOTE]
> **AI Agents**: If you are an AI coding assistant entering this workspace, you **must** immediately read and follow the instructions in **[AGENTS.md](file:///home/wimvm/works/agy_orchestrator/AGENTS.md)** before proceeding.

**AGY Orchestrator** is a lightweight, zero-dependency control tower designed to orchestrate, monitor, and evolve multiple software projects utilizing Just-in-Time (JIT) memory management, Obsidian-style personal knowledge vaults, and sub-agent task delegation.

---

## ⚙️ Installation & Quick Start

To install or upgrade **AGY Orchestrator** instantly on Linux x86_64, run the following one-liner in your terminal:

```bash
curl -fsSL https://raw.githubusercontent.com/imwoo90/agy_orchestrator/main/install.sh | bash
```

### For Developers (Compile from Source)
If you want to compile and modify the orchestrator locally:
```bash
git clone https://github.com/imwoo90/agy_orchestrator.git
cd agy_orchestrator
cargo install --path .
```
For AI agents performing code modifications, please strictly refer to **[AGENTS.md](file:///home/wimvm/works/agy_orchestrator/AGENTS.md)**.

---

## 🚀 Key Features

* **Sub-Agent Delegation**: Spawn sandboxed sub-agents with custom objectives and hot-swap context.
* **Just-in-Time (JIT) Memory**: Dynamic caching of system instructions and obsidian notes instead of massive context loading.
* **Automated Context Compression**: Log compressor that condenses output logs and token consumption.
* **Over-The-Air (OTA) Updates**: Seamless updates directly from GitHub Releases without requiring local compilation.
* **Self-Evolution Framework**: Internal issue tracking system and safe evolution verification gate (Harness).

---

## 🛠️ Orchestrator CLI Utilities

Run `agy-orchestrator <command> [options]` to manage your agent workspaces.

### Core Commands
* **`get-context --name <name>`**: Resolves path and loads dynamic JIT memory context.
* **`spawn --name <name> --path <path> --goal "<goal>"`**: Spawns an agent task in the target directory.
* **`status --name <name>`**: Checks the status of sub-agents and retrieves report artifacts.
* **`consolidate --name <name>`**: Merges task outputs and consolidates lessons into vault memories.
* **`query-memory` / `update-memory`**: Manages personal knowledge vaults (Habits, Preferences).
* **`daemon`**: Controls the background manager daemon (`--start`, `--stop`, `--status`, `--run`).
* **`dashboard`**: Starts the embedded fullstack dashboard on port 8080.
* **`self-upgrade`**: Upgrades the active orchestrator. Use `--remote` for OTA release download.
* **`evolution-harness` [NEW]**: Executes strict verification gates (Clippy & Test) for self-evolution issue resolution.

---

## 📁 Repository Map

* **`src/backend/`**: Rust core daemon, CLI logic, and OTA/evolution upgrade engine.
* **`src/frontend/`**: Dioxus 0.7.9 fullstack web dashboard.
* **`AGENTS.md`**: Protocol guidelines and harness checkpoints for AI agents.
* **`install.sh`**: Automated one-click installation script.


