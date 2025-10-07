#!/bin/bash

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║              Vork Model Diagnostics                        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check config
echo -e "${YELLOW}Checking Vork configuration...${NC}"
if [ -f ~/.vork/config.toml ]; then
    echo -e "${GREEN}✓${NC} Config file exists"
    echo ""
    echo "Backend: $(grep default_backend ~/.vork/config.toml)"
    echo "Model: $(grep 'model =' ~/.vork/config.toml | head -1)"
    echo "Server: $(grep server_url ~/.vork/config.toml | head -1)"
else
    echo -e "${RED}✗${NC} No config file found at ~/.vork/config.toml"
fi

echo ""
echo -e "${YELLOW}Checking Ollama...${NC}"

# Check if Ollama is running
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Ollama is running"

    # List available models
    echo ""
    echo "Available models:"
    curl -s http://localhost:11434/api/tags | jq -r '.models[] | "  • \(.name) - \(.details.parameter_size) (\(.details.quantization_level))"' 2>/dev/null || echo "  (jq not installed, run: curl -s http://localhost:11434/api/tags)"

    # Check loaded models
    echo ""
    echo "Currently loaded models:"
    LOADED=$(curl -s http://localhost:11434/api/ps | jq -r '.models[] | .name' 2>/dev/null)
    if [ -z "$LOADED" ]; then
        echo -e "  ${RED}✗ No models loaded${NC}"
        echo ""
        echo -e "${YELLOW}To load a model, you need to:${NC}"
        echo "  1. Send a request to it, or"
        echo "  2. Run: ollama run qwen3coder"
    else
        echo -e "  ${GREEN}✓${NC} $LOADED"
    fi
else
    echo -e "${RED}✗${NC} Ollama is not running"
    echo "  Start it with: ollama serve"
fi

echo ""
echo -e "${YELLOW}Checking llama-server...${NC}"

if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} llama-server is running on port 8080"

    # Try to get model info
    MODEL_INFO=$(curl -s http://localhost:8080/v1/models 2>/dev/null)
    if [ ! -z "$MODEL_INFO" ]; then
        echo "  Model: $MODEL_INFO"
    fi
else
    echo -e "${RED}✗${NC} llama-server is not running on port 8080"
fi

echo ""
echo -e "${YELLOW}Checking GPU...${NC}"

if command -v nvidia-smi &> /dev/null; then
    echo ""
    nvidia-smi --query-gpu=index,name,memory.used,memory.total --format=csv
    echo ""

    VRAM_USED=$(nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits | head -1)

    if [ "$VRAM_USED" -lt 1000 ]; then
        echo -e "${RED}⚠️  Warning: Only ${VRAM_USED}MB VRAM in use${NC}"
        echo "   A 30B Q6_K model should use ~20GB+"
        echo "   Model may not be loaded!"
    else
        echo -e "${GREEN}✓${NC} VRAM usage looks normal (${VRAM_USED}MB)"
    fi
else
    echo -e "${YELLOW}nvidia-smi not found (CPU only?)${NC}"
fi

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                    Recommendations                         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if model is loaded
LOADED=$(curl -s http://localhost:11434/api/ps | jq -r '.models[] | .name' 2>/dev/null)
if [ -z "$LOADED" ]; then
    echo -e "${YELLOW}1. Load your model first:${NC}"
    echo "   ollama run qwen3coder"
    echo ""
    echo "   Or send a request to it via Vork (it will auto-load)"
    echo ""
fi

CONFIG_MODEL=$(grep 'model =' ~/.vork/config.toml | grep -oP '"\K[^"]+' | head -1)
OLLAMA_MODEL=$(curl -s http://localhost:11434/api/tags | jq -r '.models[0].name' 2>/dev/null)

if [ "$CONFIG_MODEL" != "$OLLAMA_MODEL" ]; then
    echo -e "${YELLOW}2. Config model mismatch:${NC}"
    echo "   Config has: $CONFIG_MODEL"
    echo "   Ollama has: $OLLAMA_MODEL"
    echo ""
    echo "   Update ~/.vork/config.toml to:"
    echo "   model = \"$OLLAMA_MODEL\""
    echo ""
fi

echo -e "${GREEN}To test if model is working:${NC}"
echo "  vork \"what is 2+2?\""
echo ""
