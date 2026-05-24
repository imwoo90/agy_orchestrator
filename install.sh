#!/usr/bin/env bash

# AGY Agentic Orchestrator - One-click Automated Installer
# Supported OS: Linux (x86_64)

set -e

echo "🗼 AGY Orchestrator Installer starting..."

# 1. OS check
OS_TYPE="$(uname -s)"
ARCH_TYPE="$(uname -m)"

if [ "$OS_TYPE" != "Linux" ] || [ "$ARCH_TYPE" != "x86_64" ]; then
    echo "❌ Error: Currently, pre-compiled binaries only support Linux x86_64."
    echo "For other platforms, please compile from source: cargo install --path ."
    exit 1
fi

# 2. Retrieve latest release URL from GitHub API (independent of jq)
echo "🔍 Fetching latest release metadata from GitHub..."
API_URL="https://api.github.com/repos/imwoo90/agy_orchestrator/releases/latest"
DOWNLOAD_URL=$(curl -s "$API_URL" | grep -o '"browser_download_url": *"[^"]*"' | grep "agy-orchestrator" | cut -d '"' -f 4 || true)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "❌ Error: Could not resolve binary download URL from GitHub Releases."
    echo "Please check your internet connection or verify the repository Releases page."
    exit 1
fi

# 3. Create install directories
INSTALL_DIR="$HOME/.local/bin"
echo "📁 Creating binary installation directory: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"

# 4. Download pre-compiled binary
echo "📥 Downloading binary asset..."
curl -L -o "$INSTALL_DIR/agy-orchestrator" "$DOWNLOAD_URL"
chmod +x "$INSTALL_DIR/agy-orchestrator"

# 5. Bootstrap config directories
echo "🧠 Creating JIT memory & logs structures at ~/.agy_orchestrator/..."
mkdir -p "$HOME/.agy_orchestrator/memory/vault"
mkdir -p "$HOME/.agy_orchestrator/memory/skills"
mkdir -p "$HOME/.agy_orchestrator/logs"

# 6. Set up systemd user service for auto-start on boot & persistence
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
echo "⚙️ Configuring systemd user service..."
mkdir -p "$SYSTEMD_USER_DIR"

cat <<EOF > "$SYSTEMD_USER_DIR/agy-orchestrator.service"
[Unit]
Description=AGY Agentic Orchestrator Background Daemon
After=network.target

[Service]
ExecStart=%h/.local/bin/agy-orchestrator daemon --run
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=default.target
EOF

echo "🔄 Activating and starting agy-orchestrator service..."
systemctl --user daemon-reload
systemctl --user enable agy-orchestrator.service
systemctl --user restart agy-orchestrator.service

# 7. Verify installation
echo "✅ Verification: Running basic sanity check..."
"$INSTALL_DIR/agy-orchestrator" --help > /dev/null

# 7. Check PATH and alert user if needed
echo ""
echo "✨ Installation Completed Successfully! ✨"
echo "--------------------------------------------------"
echo "Binary location: $INSTALL_DIR/agy-orchestrator"

if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️ Warning: '$HOME/.local/bin' is not in your PATH."
    echo "To run the orchestrator globally, add this line to your shell config file (e.g. ~/.bashrc or ~/.zshrc):"
    echo '  export PATH="$HOME/.local/bin:$PATH"'
    echo "Then reload your shell: source ~/.bashrc"
else
    echo "🎉 '$HOME/.local/bin' is already in your PATH."
    echo "You can now run the orchestrator using:"
    echo "  agy-orchestrator --help"
fi
echo "--------------------------------------------------"
