# ✅ Issue RESOLVED: Qwen3 Coder 30B Now Running

## Problem Was

Config had model name mismatch and insufficient VRAM for Q6_K with large context.

**Fixed Status:**
- ✅ Model loaded: Qwen3-Coder-30B-A3B-Instruct-Q5_K_S.gguf
- ✅ VRAM usage: 19.3GB (7.9GB on RTX 3060 + 11.4GB on RTX 5080)
- ✅ Speed: ~38 tokens/second for generation
- ✅ Using llama-server directly (more control than Ollama)

## Root Cause

The model isn't loaded because:
1. Ollama doesn't auto-load models
2. Models only load when you first request them
3. Config model name doesn't match Ollama model name

## Solution

### Option 1: Fix Config (Recommended)

Edit `~/.vork/config.toml`:

```toml
[assistant]
server_url = "http://localhost:11434"  # Use Ollama's port
model = "qwen3coder:latest"             # Match Ollama's model name (not qwen3-coder-30b-tools)
```

Then test:
```bash
vork "what is 2+2?"
```

This will load the model (takes ~30 seconds first time, then stays loaded).

### Option 2: Use llama-server Instead

If you want more control and better performance:

```bash
# Start llama-server with your 30B model
/home/k/llama.cpp/build/bin/llama-server \
  -m /media/k/vbox/models/Qwen3/qwen3-coder-30b-tools.gguf \
  --host 0.0.0.0 \
  --port 8080 \
  -c 42768 \
  -ngl 48 \
  -t 20 \
  -b 170 \
  --parallel 8
```

Then in config:
```toml
[assistant]
server_url = "http://localhost:8080"
```

## Why Low VRAM?

A 30B Q6_K model should use:
- **Expected**: ~20-24 GB VRAM (with ngl=48)
- **Current**: 537 MB (model not loaded!)

The model isn't in memory because Ollama hasn't loaded it yet.

## Audit Task Failures

The audit tasks are failing because:
1. No model is actually processing requests
2. Requests timeout or get no response
3. Vork thinks it's connected but model isn't loaded

## Quick Fix Commands

```bash
# Check current status
./scripts/check-model.sh

# Load model via Ollama
ollama run qwen3coder

# Or just send a request (auto-loads)
vork "hello"

# Check VRAM after loading
nvidia-smi
```

## Expected After Fix

After loading the model properly:
- VRAM usage: **~20-24 GB** on your RTX 5080 (16GB) + RTX 3060 (12GB)
- Response time: Fast (2-3 tokens/sec)
- Audit tasks: Complete successfully
- Model stays loaded (until you stop Ollama)

## Verify It's Working

```bash
# 1. Check VRAM
nvidia-smi

# Should show ~20GB+ in use

# 2. Test simple request
vork "what is rust?"

# Should respond quickly

# 3. Test audit
vork --agent code-auditor "audit this project"

# Should complete the audit
```
