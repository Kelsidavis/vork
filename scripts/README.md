# Vork Setup Scripts

Automated setup scripts for common Vork configurations.

## Available Scripts

### 🖼️ setup-vision-2b.sh

Automated setup for vision support (CPU-optimized).

**What it does:**
- Downloads LLaVA 1.6 Mistral 7B (Q4_K_S quantization)
- Downloads vision projector (mmproj)
- Configures Vork for CPU-only vision support
- Optimizes settings for best CPU performance

**Downloads:**
- Model: ~4.1 GB (Q4_K_S - fastest for CPU)
- Vision Projector: ~600 MB
- Total: ~4.7 GB

**Usage:**
```bash
cd vork
./scripts/setup-vision-2b.sh
```

**Performance:**
- Speed: ~0.5-2 tokens/second (CPU only)
- Screenshot analysis: ~1-2 minutes
- Best for: Occasional use, simple GUIs

**Requirements:**
- wget or curl
- ~5 GB free disk space
- 8+ CPU cores recommended
- 8+ GB RAM

**After Setup:**
```bash
# Test vision
vork "analyze screenshot.png and describe the UI"

# Analyze specific elements
vork "what buttons are visible in mockup.jpg?"

# Read text from images
vork "extract text from this dialog"
```

## Manual Setup

If you prefer manual setup or need a different model size, see:
- [docs/VISION_SETUP.md](../docs/VISION_SETUP.md) - Complete vision setup guide
- [README.md](../README.md) - General Vork setup

## Troubleshooting

### Script fails to download
- Check internet connection
- Try running again (downloads resume automatically)
- Manually download from HuggingFace if needed

### Config backup exists
- Script backs up existing config to `config.toml.backup`
- Review differences before proceeding
- Restore backup if needed: `cp ~/.vork/config.toml.backup ~/.vork/config.toml`

### llama-server not found
- Install llama.cpp with server support
- Update `llama_server_path` in `~/.vork/config.toml`
- See main README for installation instructions

### Vision not working
- Verify mmproj path in config.toml
- Check that `extra_args` includes `--mmproj`
- Ensure model file exists in `~/.vork/models/`

## Notes

- Script uses Q4_K_S quantization for fastest CPU performance
- Context size set to 2048 for speed (vs 4096 default)
- `--mlock` flag locks model in RAM to prevent swapping
- GPU layers set to 0 (CPU only)

For GPU-optimized setup, see [docs/VISION_SETUP.md](../docs/VISION_SETUP.md).
