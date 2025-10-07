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
- **2B** - Fast, low memory, basic UI analysis (~2-5 tok/s CPU)
- **7B** - Good balance, recommended for most users (~0.5-2 tok/s CPU)
- **13B/72B** - Best quality, requires GPU (too slow on CPU)

## CPU vs GPU Performance

### GPU (Recommended for Vision)
```toml
n_gpu_layers = 35  # Most layers on GPU
```
- **2B models**: ~20-40 tokens/sec (very usable)
- **7B models**: ~10-20 tokens/sec (good)
- **13B models**: ~5-10 tokens/sec (acceptable)

### CPU Only (Challenging but possible)
```toml
n_gpu_layers = 0  # All on CPU
threads = 16      # Use all CPU cores
```
- **2B models**: ~2-5 tokens/sec (usable, 20-30 seconds per response)
- **7B models**: ~0.5-2 tokens/sec (SLOW, 1-2 minutes per response)
- **13B+ models**: ~0.1-0.5 tokens/sec (too slow, 5+ minutes)

### Hybrid (Best for CPU-focused systems)
```toml
n_gpu_layers = 10  # Some layers on GPU, rest on CPU
```
- Better than pure CPU, not as fast as full GPU
- Good if you have limited VRAM (4-6GB)

## CPU-Optimized Setup

If you're running on CPU only, here's the best configuration:

### 1. Use the Smallest Vision Model

**MobileVLM (2B) - Best for CPU**
```bash
# Download (if available as GGUF)
wget https://huggingface.co/.../mobilevlm-2b-q4_k_m.gguf
wget https://huggingface.co/.../mobilevlm-2b-mmproj.gguf
```

**Or LLaVA 1.6 (2B if available, otherwise 7B Q4_K_S)**
```bash
# Use the smallest quantization
wget https://huggingface.co/cjpais/llava-v1.6-mistral-7b-gguf/resolve/main/llava-v1.6-mistral-7b.Q4_K_S.gguf
```

### 2. CPU-Optimized Config

```toml
[llm]
model_name = "llava-v1.6-mistral-7b.Q4_K_S.gguf"  # Smallest quantization
server_url = "http://127.0.0.1:8080/v1"

[server]
auto_start = true
llama_server_path = "/usr/local/bin/llama-server"
context_size = 2048  # Smaller = faster
n_gpu_layers = 0     # CPU only
threads = 16         # Use all CPU cores (adjust to your CPU)
extra_args = "--mmproj /path/to/mmproj-model-f16.gguf --mlock"

[approval]
policy = "auto"
```

### 3. Performance Tips for CPU

**Maximize CPU Usage:**
- Set `threads` to your CPU core count (check with `nproc`)
- Close other heavy applications
- Use `--mlock` to lock model in RAM (prevents swapping)

**Reduce Context Size:**
```toml
context_size = 2048  # Instead of 4096
```
- Smaller context = faster processing
- Still enough for most image analysis

**Use Q4 or Q5 Quantization:**
- Q4_K_S - Smallest, fastest
- Q4_K_M - Good balance
- Q5_K_S - Better quality, slower
- Q8 - Too large for CPU

**Batch Your Requests:**
- Analyze multiple aspects in one prompt instead of multiple prompts
- Example: "Describe the UI layout, list all buttons, and read any visible text"

## Realistic CPU Expectations

### What Works on CPU:
✅ **2B Vision Models** (with patience)
- ~30 seconds to analyze a screenshot
- Good for occasional use
- Basic UI analysis works well

✅ **Simple Images**
- Screenshots with clear UI elements
- Text-heavy images (forms, dialogs)
- Simple layouts

### What's Challenging on CPU:
⚠️ **7B Vision Models**
- 1-2 minutes per response
- Workable if you're patient
- Quality is much better than 2B

❌ **13B+ Vision Models**
- 5+ minutes per response
- Not practical for interactive use
- Consider cloud GPU or local GPU instead

❌ **Complex Images**
- High-resolution images (>2048px)
- Multiple images at once
- Very detailed scenes

## Alternative: Use Non-Vision Models

If CPU performance is too slow, consider this workflow:

1. **Extract text from image first** (OCR)
```bash
tesseract screenshot.png output.txt
vork "analyze this UI description: $(cat output.txt)"
```

2. **Manually describe the image**
```bash
vork "I have a login screen with username field, password field, and a blue 'Sign In' button. Help me implement this."
```

3. **Use cloud vision API occasionally**
```bash
# Use OpenAI Vision API for complex analysis
# Then use local Vork for the coding work
```

## Speed Comparison Table

| Setup | 2B Model | 7B Model | 13B Model |
|-------|----------|----------|-----------|
| **High-end GPU (RTX 4090)** | 40 tok/s ⚡ | 25 tok/s ⚡ | 15 tok/s ✅ |
| **Mid GPU (RTX 3060)** | 25 tok/s ⚡ | 12 tok/s ✅ | 6 tok/s ⚠️ |
| **Low GPU (GTX 1660)** | 15 tok/s ✅ | 5 tok/s ⚠️ | 2 tok/s ❌ |
| **CPU (16 cores)** | 3 tok/s ⚠️ | 1 tok/s ❌ | 0.3 tok/s ❌ |
| **CPU (8 cores)** | 2 tok/s ❌ | 0.5 tok/s ❌ | 0.1 tok/s ❌ |

⚡ Very usable (instant responses)
✅ Usable (acceptable wait times)
⚠️ Slow but workable (patience required)
❌ Too slow (impractical)

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
