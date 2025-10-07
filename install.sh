#!/usr/bin/env bash
# Install vork to ~/.local/bin (no sudo needed)

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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
    echo -e "${YELLOW}⚠️  ~/.local/bin is not in your PATH${NC}"
    echo ""
    echo "Add this to your ~/.bashrc or ~/.zshrc:"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "Then run: source ~/.bashrc  (or source ~/.zshrc)"
else
    echo ""
    echo -e "${GREEN}✅ Installation complete!${NC}"
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}                    Optional: Vision Support               ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo "Vork can analyze images (screenshots, GUIs, diagrams) with vision models."
echo ""
read -p "Would you like to set up vision support now? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo -e "${BLUE}Starting vision setup...${NC}"
    echo ""
    ./scripts/setup-vision-2b.sh
else
    echo ""
    echo -e "${YELLOW}Skipping vision setup.${NC}"
    echo ""
    echo "You can set up vision later by running:"
    echo "    ./scripts/setup-vision-2b.sh"
    echo ""
    echo "Or see: docs/VISION_SETUP.md for manual setup"
fi

echo ""
echo -e "${GREEN}Ready to use Vork! 🐴${NC}"
echo ""
echo "Run: vork"
echo ""
