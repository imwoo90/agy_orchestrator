#!/usr/bin/env bash

# AGY Agentic Orchestrator - Clean Uninstaller
# Supported OS: Linux (x86_64)

echo "🗼 AGY Orchestrator Uninstaller starting..."

# 1. Stop and disable systemd user service if exists
SYSTEMD_SERVICE_FILE="$HOME/.config/systemd/user/agy-orchestrator.service"
if [ -f "$SYSTEMD_SERVICE_FILE" ]; then
    echo "🛑 Disabling and stopping systemd user service..."
    systemctl --user stop agy-orchestrator.service || true
    systemctl --user disable agy-orchestrator.service || true
    rm -f "$SYSTEMD_SERVICE_FILE"
    systemctl --user daemon-reload
fi

# 2. Stop legacy background daemon if running via daemon.pid
PID_PATH="$HOME/.agy_orchestrator/daemon.pid"
if [ -f "$PID_PATH" ]; then
    DAEMON_PID=$(cat "$PID_PATH")
    if ps -p "$DAEMON_PID" > /dev/null 2>&1; then
        echo "🛑 Stopping legacy background orchestrator daemon (PID: $DAEMON_PID)..."
        kill "$DAEMON_PID" || true
        sleep 1
    fi
fi

# 3. Force terminate any remaining orphan agy-orchestrator processes
if pgrep -f "agy-orchestrator daemon" > /dev/null 2>&1; then
    echo "🛑 Terminating remaining daemon processes..."
    pkill -f "agy-orchestrator daemon" || true
fi

# 3. Remove compiled/installed binary
INSTALL_BIN="$HOME/.local/bin/agy-orchestrator"
if [ -f "$INSTALL_BIN" ]; then
    echo "🗑️ Removing binary: $INSTALL_BIN"
    rm -f "$INSTALL_BIN"
fi

# 4. Remove all configurations, memories, and logs
CONFIG_DIR="$HOME/.agy_orchestrator"
if [ -d "$CONFIG_DIR" ]; then
    echo "🗑️ Removing JIT memories, configs, and logs directory: $CONFIG_DIR"
    rm -rf "$CONFIG_DIR"
fi

echo ""
echo "✨ AGY Orchestrator has been uninstalled successfully! ✨"
echo "--------------------------------------------------"
echo "Note: If you added '$HOME/.local/bin' to your PATH in your shell config"
echo "(e.g., ~/.bashrc or ~/.zshrc), you may want to remove it manually."
echo "--------------------------------------------------"
