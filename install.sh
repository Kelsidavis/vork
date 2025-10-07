#!/usr/bin/env bash
# Install vork to ~/.local/bin (no sudo needed)

set -e

echo "🔨 Building vork..."
cargo build --release

# Create ~/.local/bin if it doesn't exist
mkdir -p ~/.local/bin

echo "📦 Installing to ~/.local/bin/vork..."
cp target/release/vork ~/.local/bin/vork
chmod +x ~/.local/bin/vork

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo ""
    echo "⚠️  ~/.local/bin is not in your PATH"
    echo ""
    echo "Add this to your ~/.bashrc or ~/.zshrc:"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "Then run: source ~/.bashrc  (or source ~/.zshrc)"
else
    echo ""
    echo "✅ Installation complete!"
    echo ""
    echo "Run: vork"
fi
