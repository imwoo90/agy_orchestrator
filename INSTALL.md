# 🛠️ Installation & Operation Guide

This guide outlines how to build, install, and operate the **AGY Orchestrator** on your local machine. 

Depending on your use case, you can install the pre-compiled production binary directly (Recommended for general users) or compile it from source (Recommended for developers/maintainers).

---

## 📋 Prerequisites

Before installing the orchestrator, ensure you have the target execution tool setup:
- **AGY CLI**: The underlying AI execution tool (`agy`) must be available in your `PATH`.
  - Ensure `/home/user/.local/bin/agy` or equivalent execution path is set up.

---

## ⚙️ Installation Channels

Choose one of the two options below to install:

### Option A: Install Pre-compiled Binary (Standard Mode)
Best for general users who do not need code compilation or self-evolution features.

1. Go to the GitHub Releases page: [imwoo90/agy_orchestrator Releases](https://github.com/imwoo90/agy_orchestrator/releases).
2. Download the latest compiled binary (`agy-orchestrator`) for your platform.
3. Move the binary into your shell's binary path:
   ```bash
   mv agy-orchestrator ~/.local/bin/
   chmod +x ~/.local/bin/agy-orchestrator
   ```
> [!NOTE]
> Running in this mode automatically disables the self-evolution compilation scanner and switches the orchestrator daemon to **Standard Mode** (runs purely as a lightweight agent supervisor without local git workspace dependencies).

---

### Option B: Compile from Source (Developer/Self-Evolution Mode)
Best for maintainers who want to test local modifications and run self-evolution upgrades.

> [!NOTE]
> For AI agents operating in Developer Mode, please strictly follow the development protocols and checkout gates detailed in **[AGENTS.md](file:///home/wimvm/works/agy_orchestrator/AGENTS.md)**.

#### Additional Requirements
- **Rust toolchain** (v1.75+ recommended): Install via [rustup.rs](https://rustup.rs/)
- **Dioxus CLI** (for fullstack web server builds):
  ```bash
  cargo install dioxus-cli --version 0.6.0-alpha.5
  ```

#### Build Instructions
1. Clone the repository and compile:
   ```bash
   git clone https://github.com/imwoo90/agy_orchestrator.git
   cd agy_orchestrator
   cargo install --path .
   ```
2. Verify:
   ```bash
   agy-orchestrator --help
   ```

---

## 🚀 Running the Orchestrator

The orchestrator consists of two primary parts: the background daemon and the web dashboard.

### 1. Starting the Background Daemon
The daemon monitors active agent tasks, handles log compression, auto-reaps processes, and scans for upgrades.
```bash
agy-orchestrator daemon --start
```
- Check status: `agy-orchestrator daemon --status`
- Stop: `agy-orchestrator daemon --stop`

---

### 2. Launching the Web Dashboard
The web dashboard provides a premium UI to visually track active projects, inspect issues, view vault notes, and review hierarchy tree views of sub-agents.
```bash
agy-orchestrator dashboard --port 8080
```
Open your browser and navigate to `http://localhost:8080`.

---

## 🔄 Updates & Upgrades (OTA)

The orchestrator supports **Over-The-Air (OTA) Updates** via GitHub Releases.

### Automatic Version Notifications
The background daemon automatically checks for new tags on GitHub every hour. If a new version is detected, a warning notice is sent to `~/.agy_orchestrator/notifications.log`.

### Upgrading via Command Line
To upgrade your installation directly from the latest GitHub release without compiling from source:
```bash
agy-orchestrator self-upgrade --remote
```

### Upgrading via Web Dashboard
If a new update is detected, an **`Update to vX.Y.Z 🚀`** button will automatically appear in the top header bar of the Web Dashboard. Click it to perform a non-intrusive upgrade.

---

## 🧠 Data & Configuration Structure

All global configurations, memories, and logs are stored inside your user home directory:
- **Global Path**: `~/.agy_orchestrator/`
- **Active Projects List**: `~/.agy_orchestrator/projects.json`
- **Knowledge Vault**: `~/.agy_orchestrator/memory/vault/` (Obsidian-style Markdown notes)
- **Skills Catalog**: `~/.agy_orchestrator/memory/skills/` (YAML-frontmatter guidelines)
- **Execution Log Files**: `~/.agy_orchestrator/logs/`
- **Notifications History**: `~/.agy_orchestrator/notifications.log`
