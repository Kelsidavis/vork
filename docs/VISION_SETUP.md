# Vision Support Setup for Vork

This guide shows how to enable image analysis in Vork, specifically for analyzing GUIs, screenshots, and user interfaces.

## Quick Setup (3 Steps)

### 1. Download a Vision Model

Choose one option:

**Option A: LLaVA 1.6 (Recommended - Best for UI/Screenshot analysis)**
```bash
cd ~/.vork/models/  # or your model directory

# Download model (7B quantized)
wget https://huggingface.co/cjpais/llava-v1.6-mistral-7b-gguf/resolve/main/llava-v1.6-mistral-7b.Q4_K_M.gguf

# Download vision projector (REQUIRED for images)
wget https://huggingface.co/cjpais/llava-v1.6-mistral-7b-gguf/resolve/main/mmproj-model-f16.gguf
```

**Option B: Qwen2-VL (Better at general vision)**
```bash
cd ~/.vork/models/

# Download model
wget https://huggingface.co/Qwen/Qwen2-VL-7B-Instruct-GGUF/resolve/main/qwen2-vl-7b-instruct-q4_k_m.gguf

# Download vision projector
wget https://huggingface.co/Qwen/Qwen2-VL-7B-Instruct-GGUF/resolve/main/qwen2-vl-7b-mmproj-q4_k_m.gguf
```

### 2. Update Vork Config

Edit `~/.vork/config.toml`:

```toml
[llm]
model_name = "llava-v1.6-mistral-7b.Q4_K_M.gguf"  # Your vision model
server_url = "http://127.0.0.1:8080/v1"

[server]
auto_start = true
llama_server_path = "/path/to/llama-server"
context_size = 4096
n_gpu_layers = 35  # Adjust for your GPU (0 = CPU only)
threads = 8

# CRITICAL: Add this for vision models
extra_args = "--mmproj /path/to/.vork/models/mmproj-model-f16.gguf"
```

**Important:** The `--mmproj` flag is what enables vision! Without it, the model can't see images.

### 3. Test Vision

```bash
# Take a screenshot or use an existing image
vork "analyze screenshot.png and describe the UI elements"

# Or be more specific
vork "what buttons and text are visible in app-screenshot.jpg?"

# Analyze UI layout
vork "describe the layout and structure of this GUI in wireframe.png"
```

## Full Example Configuration

Here's a complete `~/.vork/config.toml` for vision:

```toml
[llm]
model_name = "llava-v1.6-mistral-7b.Q4_K_M.gguf"
server_url = "http://127.0.0.1:8080/v1"

[server]
auto_start = true
llama_server_path = "/usr/local/bin/llama-server"
host = "127.0.0.1"
port = 8080
context_size = 4096
n_gpu_layers = 35
threads = 8
extra_args = "--mmproj /home/k/.vork/models/mmproj-model-f16.gguf"

[approval]
policy = "auto"  # or "always-ask" for safety

[sandbox]
mode = "workspace-write"
```

## Usage Examples

### Analyze GUI Screenshots
```bash
# General description
vork "what do you see in ui-mockup.png?"

# Specific elements
vork "list all buttons and their labels in screenshot.png"

# Layout analysis
vork "describe the navigation structure in app-ui.jpg"
```

### Read Text from Images
```bash
vork "what text is visible in this screenshot?"
vork "extract the error message from error-dialog.png"
```

### UI/UX Feedback
```bash
vork "analyze the user interface design in mockup.png and suggest improvements"
vork "describe the color scheme and layout in dashboard.jpg"
```

### Code from Screenshots
```bash
vork "what code is shown in this screenshot?"
vork "transcribe the code from code-snippet.png"
```

## Troubleshooting

### "Model doesn't respond to image questions"
- ✅ Check `--mmproj` flag is in `extra_args`
- ✅ Verify mmproj file path is correct
- ✅ Make sure mmproj matches your model (LLaVA mmproj for LLaVA model)

### "Out of memory errors"
- Lower `context_size` to 2048 or 3072
- Reduce `n_gpu_layers` (try 20 or 0 for CPU)
- Use a smaller model (2B instead of 7B)

### "Images not loading"
- Check file path is correct (use `./` for workspace-relative)
- Verify image format is supported (PNG, JPG, GIF, BMP, WebP)
- Check file permissions

## Model Recommendations for GUI Analysis

**Best for UI/Screenshot Analysis:**
1. **LLaVA 1.6** - Excellent at reading UI text and layouts
2. **Qwen2-VL** - Great general vision, good UI understanding
3. **BakLLaVA** - Lightweight, decent for simple UIs

**Model Sizes:**
- **2B** - Fast, low memory, basic UI analysis
- **7B** - Good balance, recommended for most users
- **13B/72B** - Best quality, requires more GPU memory

## Quick Reference: llama-server Command

Manual start (if not using auto-start):

```bash
# LLaVA 1.6
llama-server \
  -m ~/.vork/models/llava-v1.6-mistral-7b.Q4_K_M.gguf \
  --mmproj ~/.vork/models/mmproj-model-f16.gguf \
  --host 127.0.0.1 \
  --port 8080 \
  -c 4096 \
  -ngl 35

# Qwen2-VL
llama-server \
  -m ~/.vork/models/qwen2-vl-7b-instruct-q4_k_m.gguf \
  --mmproj ~/.vork/models/qwen2-vl-7b-mmproj-q4_k_m.gguf \
  --host 127.0.0.1 \
  --port 8080 \
  -c 4096 \
  -ngl 35
```

## Tips for Best Results

1. **Use high-quality screenshots** - Clear, high-resolution images work best
2. **Ask specific questions** - "What buttons are visible?" vs. "Describe this"
3. **One image at a time** - Analyze complex UIs step by step
4. **Use descriptive filenames** - `login-screen.png` vs. `img001.png`

## Next Steps

- Try analyzing some GUI screenshots
- Experiment with different questions/prompts
- Adjust `n_gpu_layers` for your hardware
- Check [llama.cpp vision docs](https://github.com/ggerganov/llama.cpp) for advanced options

---

**Note:** Vision support requires a vision-capable model. Regular text-only models (like Qwen 3 Coder) cannot analyze images even with the mmproj file.
