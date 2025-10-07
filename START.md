# Quick Start Guide

## What happens when you type `vork`?

1. **🔍 Checks for existing llama-server** - Kills any running instances
2. **🚀 Starts your local model** - Automatically launches llama-server with your configured model
3. **⏳ Waits for server to be ready** - Polls health endpoint until ready
4. **🎨 Opens beautiful TUI** - Rich terminal interface with:
   - Colored message display (you, assistant, tools, errors)
   - Real-time status bar with session info and token usage
   - Context-aware conversation tracking
   - Automatic tool execution with visual feedback

## Usage

```bash
# Just type vork - that's it!
vork

# Or with initial prompt
vork "help me fix this code"

# Advanced: specify custom server/model
vork --server http://remote:8080 --model custom-model
```

## In the TUI

- **Type your message** and press Enter
- **Ctrl+C** to exit gracefully
- **Type "exit" or "quit"** to close
- **Up/Down arrows** to scroll through conversation
- **Watch live** as tools execute and AI responds

## What it does automatically

- Reads files when needed
- Writes/edits files with approval
- Executes bash commands (with approval)
- Searches files with grep
- Tracks conversation context
- Saves sessions for resume

## Configuration

Edit `~/.vork/config.toml` to customize:
- Model location
- Server settings
- Approval policies
- Sandbox modes

## First Run

On first run, vork will:
1. Create `~/.vork/` directory
2. Generate default config
3. Look for GGUF models in your configured path
4. Start the first model it finds

Make sure you have:
- llama.cpp built with `llama-server` binary
- At least one GGUF model file
- Paths configured in config.toml
