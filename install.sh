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

# 2. Retrieve latest release URL from GitHub redirects (independent of GitHub API)
echo "🔍 Fetching latest release tag from GitHub..."
REDIRECT_URL=$(curl -sI https://github.com/imwoo90/agy_orchestrator/releases/latest | grep -i "location:" || true)
TAG_NAME=$(echo "$REDIRECT_URL" | grep -o "tag/[^[:space:]]*" | cut -d'/' -f2 | tr -d '\r\n' || true)

if [ -z "$TAG_NAME" ]; then
    echo "❌ Error: Could not resolve latest release tag from GitHub redirects."
    echo "Please check your internet connection or verify the repository Releases page."
    exit 1
fi

echo "🏷️ Latest release tag is: $TAG_NAME"
DOWNLOAD_URL="https://github.com/imwoo90/agy_orchestrator/releases/download/$TAG_NAME/agy-orchestrator-linux.tar.gz"

# 3. Create install directories
INSTALL_DIR="$HOME/.local/bin"
echo "📁 Creating binary installation directory: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"

# 4. Download and extract pre-compiled binary package
echo "📥 Downloading binary package..."
curl -L -o "$INSTALL_DIR/agy-orchestrator-linux.tar.gz" "$DOWNLOAD_URL"
echo "📦 Extracting package..."
tar -xzf "$INSTALL_DIR/agy-orchestrator-linux.tar.gz" -C "$INSTALL_DIR"
rm -f "$INSTALL_DIR/agy-orchestrator-linux.tar.gz"
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
