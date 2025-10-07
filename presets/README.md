# Vork Configuration Presets

Saved configurations for quick switching between different models and setups.

## Available Presets

### qwen3-14b-large-context.toml
**Qwen3 14B - Large Context Configuration**

- Model: Qwen3-14B-UD-Q5_K_XL.gguf
- Context: 73728 tokens (72k)
- GPU Layers: All layers on GPU
- VRAM Usage: ~25.7GB total (11.2GB + 14.5GB) - 91% utilization
- Speed: Faster than 30B model
- Best for: Long file analysis, large codebases, extended conversations

**Usage:**
```bash
cp ~/.vork/presets/qwen3-14b-large-context.toml ~/.vork/config.toml
```

Then start llama-server:
```bash
/home/k/llama.cpp/build/bin/llama-server \
  -m /media/k/vbox/models/Qwen3/Qwen3-14B-UD-Q5_K_XL.gguf \
  --host 0.0.0.0 --port 8080 \
  -c 73728 -ngl 99 -t 20 -b 512 --parallel 4 \
  --cache-type-k bf16 --cache-type-v bf16 --jinja
```

---

### qwen3-30b-max-gpu.toml
**Qwen3 Coder 30B - Maximum GPU Utilization**

- Model: Qwen3-Coder-30B-A3B-Instruct-Q5_K_S.gguf
- Context: 6144 tokens
- GPU Layers: 49/49 (all layers on GPU)
- VRAM Usage: ~21.7GB total
  - RTX 5080: 11.9GB
  - RTX 3060: 8.0GB
- Speed: ~38-41 tokens/second
- Best for: Maximum performance with full GPU utilization

**Usage:**
```bash
cp ~/.vork/presets/qwen3-30b-max-gpu.toml ~/.vork/config.toml
```

Then start llama-server:
```bash
/home/k/llama.cpp/build/bin/llama-server \
  -m /media/k/vbox/models/Qwen3/Qwen3-Coder-30B-A3B-Instruct-Q5_K_S.gguf \
  --host 0.0.0.0 --port 8080 \
  -c 6144 -ngl 49 -t 20 -b 170 --parallel 4 \
  --cache-type-k bf16 --cache-type-v bf16 --jinja
```

## Creating New Presets

1. Configure vork as desired
2. Save current config:
   ```bash
   cp ~/.vork/config.toml ~/.vork/presets/my-preset.toml
   ```
3. Add description in this README
4. Switch anytime with:
   ```bash
   cp ~/.vork/presets/my-preset.toml ~/.vork/config.toml
   ```
