#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║        Vork Vision 2B Model Setup (CPU Optimized)         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     OS_TYPE=Linux;;
    Darwin*)    OS_TYPE=Mac;;
    *)          OS_TYPE="UNKNOWN"
esac

echo -e "${GREEN}✓${NC} Detected OS: ${OS_TYPE}"
echo ""

# Set model directory
VORK_DIR="${HOME}/.vork"
MODEL_DIR="${VORK_DIR}/models"
CONFIG_FILE="${VORK_DIR}/config.toml"

# Create directories
mkdir -p "${MODEL_DIR}"
echo -e "${GREEN}✓${NC} Model directory: ${MODEL_DIR}"

# Check for wget or curl
if command -v wget &> /dev/null; then
    DOWNLOAD_CMD="wget -c"
    echo -e "${GREEN}✓${NC} Using wget for downloads"
elif command -v curl &> /dev/null; then
    DOWNLOAD_CMD="curl -L -C - -O"
    echo -e "${GREEN}✓${NC} Using curl for downloads"
else
    echo -e "${RED}✗${NC} Neither wget nor curl found. Please install one."
    exit 1
fi

echo ""
echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}  Downloading LLaVA 1.6 Mistral 7B (CPU-Optimized Q4_K_S)  ${NC}"
echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo "This will download approximately:"
echo "  • Model: ~4.1 GB (Q4_K_S quantization)"
echo "  • Vision Projector: ~600 MB"
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Setup cancelled.${NC}"
    exit 0
fi

cd "${MODEL_DIR}"

# Download model (Q4_K_S - smallest, fastest for CPU)
MODEL_FILE="llava-v1.6-mistral-7b.Q4_K_S.gguf"
if [ -f "${MODEL_FILE}" ]; then
    echo -e "${GREEN}✓${NC} Model already exists: ${MODEL_FILE}"
else
    echo ""
    echo -e "${BLUE}Downloading model (Q4_K_S - optimized for CPU)...${NC}"
    MODEL_URL="https://huggingface.co/cjpais/llava-v1.6-mistral-7b-gguf/resolve/main/llava-v1.6-mistral-7b.Q4_K_S.gguf"

    if [ "$DOWNLOAD_CMD" = "wget -c" ]; then
        wget -c "${MODEL_URL}" -O "${MODEL_FILE}"
    else
        curl -L -C - "${MODEL_URL}" -o "${MODEL_FILE}"
    fi

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Model downloaded successfully"
    else
        echo -e "${RED}✗${NC} Model download failed"
        exit 1
    fi
fi

# Download mmproj (vision projector)
MMPROJ_FILE="mmproj-model-f16.gguf"
if [ -f "${MMPROJ_FILE}" ]; then
    echo -e "${GREEN}✓${NC} Vision projector already exists: ${MMPROJ_FILE}"
else
    echo ""
    echo -e "${BLUE}Downloading vision projector (required for image analysis)...${NC}"
    MMPROJ_URL="https://huggingface.co/cjpais/llava-v1.6-mistral-7b-gguf/resolve/main/mmproj-model-f16.gguf"

    if [ "$DOWNLOAD_CMD" = "wget -c" ]; then
        wget -c "${MMPROJ_URL}" -O "${MMPROJ_FILE}"
    else
        curl -L -C - "${MMPROJ_URL}" -o "${MMPROJ_FILE}"
    fi

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Vision projector downloaded successfully"
    else
        echo -e "${RED}✗${NC} Vision projector download failed"
        exit 1
    fi
fi

echo ""
echo -e "${GREEN}✓${NC} All files downloaded successfully!"
echo ""

# Detect CPU cores
CPU_CORES=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 8)
echo -e "${GREEN}✓${NC} Detected ${CPU_CORES} CPU cores"

# Update config.toml
echo ""
echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}              Configuring Vork for Vision (CPU)            ${NC}"
echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"
echo ""

if [ -f "${CONFIG_FILE}" ]; then
    echo -e "${YELLOW}⚠${NC}  Config file already exists: ${CONFIG_FILE}"
    read -p "Update config for vision support? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Skipping config update.${NC}"
        echo ""
        echo -e "${BLUE}To manually enable vision, add this to your config.toml:${NC}"
        echo ""
        echo "[llm]"
        echo "model_name = \"${MODEL_FILE}\""
        echo ""
        echo "[server]"
        echo "context_size = 2048"
        echo "n_gpu_layers = 0"
        echo "threads = ${CPU_CORES}"
        echo "extra_args = \"--mmproj ${MODEL_DIR}/${MMPROJ_FILE} --mlock\""
        echo ""
        exit 0
    fi

    # Backup existing config
    cp "${CONFIG_FILE}" "${CONFIG_FILE}.backup"
    echo -e "${GREEN}✓${NC} Backed up existing config to ${CONFIG_FILE}.backup"
fi

# Create/update config
cat > "${CONFIG_FILE}" << EOF
[llm]
model_name = "${MODEL_FILE}"
server_url = "http://127.0.0.1:8080/v1"

[server]
auto_start = true
llama_server_path = "/usr/local/bin/llama-server"
host = "127.0.0.1"
port = 8080
context_size = 2048
n_gpu_layers = 0
threads = ${CPU_CORES}
extra_args = "--mmproj ${MODEL_DIR}/${MMPROJ_FILE} --mlock"

[approval]
policy = "auto"

[sandbox]
mode = "workspace-write"
EOF

echo -e "${GREEN}✓${NC} Configuration file updated: ${CONFIG_FILE}"
echo ""

# Summary
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    Setup Complete! ✓                       ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}Model Configuration:${NC}"
echo "  • Model: LLaVA 1.6 Mistral 7B (Q4_K_S)"
echo "  • Vision: Enabled with mmproj"
echo "  • CPU Cores: ${CPU_CORES}"
echo "  • Context: 2048 tokens (optimized for speed)"
echo "  • Location: ${MODEL_DIR}"
echo ""
echo -e "${BLUE}Expected Performance (CPU only):${NC}"
echo "  • Speed: ~0.5-2 tokens/second"
echo "  • Screenshot analysis: ~1-2 minutes"
echo "  • Best for: Occasional use, simple GUIs"
echo ""
echo -e "${BLUE}Usage Examples:${NC}"
echo "  vork \"analyze screenshot.png and describe the UI\""
echo "  vork \"what buttons are visible in app-mockup.jpg?\""
echo "  vork \"read the text from this dialog image\""
echo ""
echo -e "${YELLOW}Tips for better CPU performance:${NC}"
echo "  • Use smaller images (<2MB)"
echo "  • Close other heavy applications"
echo "  • Ask specific questions instead of general ones"
echo "  • Batch multiple questions in one prompt"
echo ""
echo -e "${BLUE}To test vision:${NC}"
echo "  vork \"analyze screenshot.png\""
echo ""
echo -e "${GREEN}Ready to analyze images! 🖼️${NC}"
