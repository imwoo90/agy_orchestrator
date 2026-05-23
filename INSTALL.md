# 🛠️ Installation & Operation Guide

This guide outlines how to build, install, and operate the **AGY Orchestrator** on your local machine.

---

## 📋 Prerequisites

Before installing the orchestrator, ensure you have the following requirements installed on your system:

- **Rust toolchain** (v1.75+ recommended): Install via [rustup.rs](https://rustup.rs/)
- **Dioxus CLI** (for running and building the web dashboard):
  ```bash
  cargo install dioxus-cli --version 0.6.0-alpha.5 # Match project dioxus version
  ```
- **AGY CLI**: The underlying AI execution tool (`agy`) must be available in your `PATH`.
  - Ensure `/home/user/.local/bin/agy` or equivalent execution path is set up.

---

## ⚙️ Installation

To compile and install the orchestrator globally on your system, follow these steps:

### 1. Build and Install Binary
Run the following cargo command inside the repository root to compile in release mode and install it to your cargo bin directory (`~/.cargo/bin`):
```bash
cargo install --path .
```

### 2. Verify Installation
Ensure the executable is available in your shell:
```bash
agy-orchestrator --help
```
> [!NOTE]
> Make sure `~/.cargo/bin` is added to your shell's `PATH` environment variable (e.g., in `~/.bashrc` or `~/.zshrc`).

---

## 🚀 Running the Orchestrator

The orchestrator consists of two primary runtime parts: the background daemon and the web dashboard.

### 1. Starting the Background Daemon
The daemon monitors active agent tasks, handles log compression, auto-reaps processes, and triggers self-evolution tasks.

To start the daemon in the background:
```bash
agy-orchestrator daemon --start
```

To check daemon status:
```bash
agy-orchestrator daemon --status
```

To stop the daemon:
```bash
agy-orchestrator daemon --stop
```

---

### 2. Launching the Web Dashboard
The web dashboard provides a premium UI to visually track active projects, inspect kanban issues, view vault notes, and review hierarchy tree views of sub-agents.

To start the dashboard server (binds to port 8080 by default):
```bash
agy-orchestrator dashboard --port 8080
```
Open your browser and navigate to `http://localhost:8080` to access the console interface.

---

## 🧠 Data & Configuration Structure

All global data and configuration memory are stored inside your user home directory:
- **Global Path**: `~/.agy_orchestrator/`
- **Active Projects List**: `~/.agy_orchestrator/projects.json`
- **Knowledge Vault**: `~/.agy_orchestrator/memory/vault/` (Obsidian-style Markdown notes)
- **Skills Catalog**: `~/.agy_orchestrator/memory/skills/` (YAML-frontmatter guidelines)
- **Execution Log Files**: `~/.agy_orchestrator/logs/`
- **Notifications History**: `~/.agy_orchestrator/notifications.log`
