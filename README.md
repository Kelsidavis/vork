# Vork 🐴

<p align="center">
  <img src="assets/vork-logo.png" alt="Vork Logo" width="400"/>
</p>

<p align="center">
  <strong>AI-Powered Coding Assistant Running Entirely on Your Hardware</strong>
</p>

<p align="center">
  A feature-complete alternative to OpenAI Codex with 15 specialized AI agents, beautiful TUI, and complete local control.
</p>

---

## ✨ Features

- 🎨 **Beautiful TUI Interface** - Ratatui-based terminal UI with custom colors per agent
- 🤖 **15 Specialized AI Agents** - Expert agents for different coding tasks
- 🎯 **Smart Agent Selection** - Automatically picks the right agent based on your task
- 🔧 **Tool Integration** - File operations, bash commands, code search, web research, image analysis
- 🚀 **Auto-Start Server** - Automatically manages llama-server lifecycle
- 💾 **Session Management** - Save and resume conversations
- ⚡ **Performance Metrics** - Real-time tokens/second display
- 🛡️ **Configurable Safety** - Approval policies and sandbox modes
- 🎨 **Window Resize Support** - Smooth handling of terminal resizing
- 🔒 **100% Local & Private** - No cloud, no API keys, no data leaves your machine

## 🤖 Available Agents

| Agent | Description | Color | Auto-Select Keywords |
|-------|-------------|-------|---------------------|
| 🚀 **default** | General-purpose coding assistant | cyan | _(default)_ |
| 🦀 **rust-expert** | Rust programming specialist | red | rust, borrow, lifetime, cargo, async |
| 🔍 **code-auditor** | Finds stubs, poor implementations, compliance issues | lightred | audit, compliance, stub, todo, fixme |
| 🔬 **reverse-engineer** | Binary analysis with radare2/Ghidra | lightmagenta | radare, ghidra, disassemble, malware, r2 |
| ✏️ **code-editor** | Precision code modifications | lightblue | edit, change, modify, rename, fix typo |
| 🚀 **release-manager** | Version management and releases | lightgreen | release, version, changelog, deploy, tag |
| ⚡ **performance-optimizer** | Speed and efficiency optimization | lightyellow | optimize, performance, benchmark, profile, slow |
| 🛡️ **security-auditor** | Security vulnerabilities and secure coding | red | security, vulnerability, exploit, cve, injection |
| 🧪 **test-writer** | Comprehensive test engineering | lightcyan | test, unit test, coverage, tdd, pytest |
| 🔧 **devops** | CI/CD, Docker, Kubernetes, infrastructure | blue | docker, kubernetes, terraform, pipeline, helm |
| 🔍 **reviewer** | Code review and suggestions | magenta | review, feedback, improve, refactor |
| 📝 **documenter** | Documentation specialist | blue | document, readme, explain, comment, doc |
| 🐛 **debugger** | Debug and fix issues | yellow | debug, fix bug, error, crash, broken |
| 🔬 **researcher** | Online research with workspace context linking | lightgreen | research, look up, web search, documentation, what is |

## 📦 Installation

### Prerequisites

- **Rust toolchain** (1.70+) - Install from [rustup.rs](https://rustup.rs/)
- **llama.cpp** with server support - [Build instructions](https://github.com/ggerganov/llama.cpp)
- **Compatible LLM model** - GGUF format (Qwen, Llama, Mistral, etc.)

### Build from Source

```bash
git clone https://github.com/yourusername/vork.git
cd vork
cargo build --release
sudo cp target/release/vork /usr/local/bin/
```

Or install directly:

```bash
cargo install --path .
```

## 🚀 Quick Start

### First-Time Setup

Run the interactive setup wizard:

```bash
vork setup
```

This guides you through:
- ✅ Model directory location
- ✅ llama-server binary path
- ✅ Server parameters (context, threads, GPU layers)
- ✅ Approval policies and sandbox modes

### Basic Usage

```bash
# Launch TUI with automatic agent selection
vork

# Use a specific agent
vork --agent rust-expert

# Start with an initial prompt
vork "fix the bugs in this code"

# One-off question
vork ask "how do I use async/await in Rust?"

# Non-interactive execution (read-only)
vork exec "analyze this code for performance issues"

# Full automation mode (allows edits)
vork exec --full-auto "refactor this function"
```

### TUI Interface

The terminal UI includes:
- 🎨 **Color-coded messages** - User (blue), Assistant (agent-specific), Tools (yellow)
- 📊 **Live status bar** - Session ID, message count, token usage, tokens/second
- 🎮 **Live GPU stats** - Real-time VRAM usage, GPU utilization, temperature (via nvidia-smi)
- 🔧 **Real-time tool execution** - Watch as the agent reads files and runs commands
- 💾 **Auto-save** - Every conversation is automatically saved
- ⌨️ **Keyboard controls**:
  - `Enter` - Send message
  - `Up/Down` - Navigate message history
  - `Ctrl+C` - Exit
  - Type `exit` or `quit` - Graceful exit

## 🎯 Automatic Agent Selection

Vork intelligently selects the appropriate agent based on your first message. No need to specify `--agent` unless you want to override!

### Examples

```bash
vork  # Then type:

# "audit this codebase for stubs and TODOs"
→ 🔍 Auto-selected: code-auditor

# "reverse engineer this binary with radare2"
→ 🔬 Auto-selected: reverse-engineer

# "optimize this slow database query"
→ ⚡ Auto-selected: performance-optimizer

# "write unit tests for the auth module"
→ 🧪 Auto-selected: test-writer

# "create a Dockerfile for this application"
→ 🔧 Auto-selected: devops

# "fix the rust borrow checker error"
→ 🦀 Auto-selected: rust-expert

# "check for security vulnerabilities"
→ 🛡️ Auto-selected: security-auditor
```

## 🔧 Agent Management

### List Agents

```bash
# View all available agents
vork agents

# View specific agent details
vork agents rust-expert
```

### Create Custom Agents

```bash
# Interactive agent creation
vork agents --create
```

Or manually create `~/.vork/agents/my-agent.json`:

```json
{
  "name": "my-agent",
  "description": "My specialized agent",
  "system_prompt": "You are an expert in...",
  "temperature": 0.7,
  "tools_enabled": true,
  "color": "cyan",
  "title": "🤖 My Agent"
}
```

Then use it:

```bash
vork --agent my-agent
```

## ⚙️ Configuration

Vork stores its configuration in `~/.vork/`:

```
~/.vork/
├── config.toml          # Main configuration
├── agents/              # Agent definitions
│   ├── default.json
│   ├── rust-expert.json
│   └── ...
├── presets/             # Pre-configured model setups
│   ├── qwen3-14b-large-context.toml
│   ├── qwen3-30b-max-gpu.toml
│   └── README.md
└── sessions/            # Saved conversations
    └── *.json
```

### Model Presets

Vork includes optimized presets for different use cases. See [presets/README.md](presets/README.md) for details.

**Quick switch:**
```bash
# Use 14B with 72k context (default)
cp presets/qwen3-14b-large-context.toml ~/.vork/config.toml

# Use 30B for maximum quality
cp presets/qwen3-30b-max-gpu.toml ~/.vork/config.toml
```

### Configuration File

Edit `~/.vork/config.toml`:

```toml
[assistant]
server_url = "http://localhost:8080"
model = "qwen3-coder-30b-tools"
approval_policy = "never"                 # auto | never | always-ask | read-only
sandbox_mode = "danger-full-access"       # read-only | workspace-write | danger-full-access
require_git_repo = false

[llamacpp]
models_dir = "/media/k/vbox/models/Qwen3"
binary_path = "/home/k/llama.cpp/build/bin/llama-server"
context_size = 42768
ngl = 48          # GPU layers
threads = 20
batch_size = 170
parallel = 8
cache_type_k = "bf16"
cache_type_v = "bf16"

[ollama]
enabled = true
api_url = "http://localhost:11434"
```

## 🛡️ Safety and Permissions

Vork has flexible approval policies to control what operations the AI can perform.

### Approval Policies

| Policy | Behavior |
|--------|----------|
| **never** | Auto-approve everything (except critical system commands like `sudo`) |
| **auto** | Auto-approve safe operations, prompt for dangerous ones |
| **always-ask** | Prompt for every operation |
| **read-only** | Block all write operations |

### Sandbox Modes

| Mode | File Writes | Bash Commands | Network |
|------|-------------|---------------|---------|
| **read-only** | ❌ Blocked | ❌ Blocked | ❌ Blocked |
| **workspace-write** | ✅ Current dir only | ✅ Safe commands | ❌ Blocked |
| **danger-full-access** | ✅ Anywhere | ✅ Almost all | ✅ Allowed |

### Protected Operations

Even in **never** + **danger-full-access** mode, these operations require approval:
- 🛑 `sudo` commands
- 🛑 `shutdown` / `reboot`
- 🛑 `mkfs`, `dd if=`, `/dev/` writes
- 🛑 Other destructive disk operations

Safe to auto-approve:
- ✅ `rm -rf` (but be careful!)
- ✅ `curl` / `wget`
- ✅ `nc` / `netcat`
- ✅ Regular bash commands

## 🔨 Available Tools

AI agents have access to these tools:

| Tool | Description |
|------|-------------|
| **read_file** | Read file contents |
| **write_file** | Create or modify files |
| **list_files** | List directory contents |
| **bash_exec** | Execute shell commands |
| **search_files** | Grep-based code search |

Tool usage is automatically tracked and displayed in the TUI.

## 📝 Session Management

```bash
# Resume last session
vork resume --last

# Resume specific session by ID
vork resume <session-id>

# List all sessions
ls ~/.vork/sessions/

# View session file
cat ~/.vork/sessions/<session-id>.json
```

Sessions include:
- Full conversation history
- Working directory context
- Tool execution results
- Timestamp metadata

## 💡 Usage Examples

### Code Auditing

```bash
vork --agent code-auditor
> "Audit this codebase for stubs, TODOs, and unwrap() calls"
```

### Reverse Engineering

```bash
vork --agent reverse-engineer
> "Analyze this binary with radare2 and document the main function"
```

### Performance Optimization

```bash
vork --agent performance-optimizer
> "Profile this function and suggest optimizations"
```

### Security Audit

```bash
vork --agent security-auditor
> "Check this code for SQL injection vulnerabilities"
```

### Test Generation

```bash
vork --agent test-writer
> "Write comprehensive unit tests for the user authentication module"
```

### DevOps Automation

```bash
vork --agent devops
> "Create a GitHub Actions workflow for CI/CD"
```

## 🐛 Troubleshooting

### Server Won't Start

```bash
# Check if server is already running
ps aux | grep llama-server

# Kill existing servers
pkill llama-server

# Check status
vork status
```

### Model Not Found

```bash
# Verify configuration
vork config --path
cat ~/.vork/config.toml

# Run setup wizard
vork setup
```

### Permission Issues

Check your approval policy and sandbox mode:

```bash
vork config
```

Edit `~/.vork/config.toml` to adjust permissions.

### Agent Not Auto-Selecting

Make sure you're not explicitly setting an agent with `--agent`. Auto-selection only works when no agent is specified.

## 🚀 Advanced Usage

### Custom Model Parameters

Edit `~/.vork/config.toml` to tune llama.cpp parameters:

```toml
[llamacpp]
context_size = 42768    # Larger context window
ngl = 48               # More GPU layers
threads = 20           # CPU threads
batch_size = 170       # Batch size
parallel = 8           # Parallel sequences
```

### Integration with Scripts

```bash
# Use exec mode for scripting
vork exec "count lines of code" --json > report.json

# Full automation for CI/CD
vork exec --full-auto "run tests and commit fixes"
```

### Multiple Agents in Sequence

```bash
# Audit first
vork --agent code-auditor
> "Find all issues"

# Then fix with editor
vork --agent code-editor
> "Fix the issues from the audit"
```

## 🤝 Contributing

Contributions are welcome! Here's how:

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/amazing-feature`
3. **Commit** your changes: `git commit -m 'Add amazing feature'`
4. **Push** to the branch: `git push origin feature/amazing-feature`
5. **Submit** a pull request

### Development Setup

```bash
# Clone repository
git clone https://github.com/yourusername/vork.git
cd vork

# Build
cargo build

# Run tests
cargo test

# Run locally
cargo run -- agents
```

### Creating New Agents

See the template in `~/.vork/agents/template.json` for guidance on creating new specialized agents.

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **[llama.cpp](https://github.com/ggerganov/llama.cpp)** - Efficient LLM inference engine
- **[ratatui](https://github.com/ratatui-org/ratatui)** - Terminal UI framework
- **[Ollama](https://ollama.ai/)** - Inspired model management design
- **Claude Code** - Inspired interactive assistant UX
- **Cursor** - Inspired code editing capabilities

## 🔗 Links

- 📂 [GitHub Repository](https://github.com/yourusername/vork)
- 🐛 [Issue Tracker](https://github.com/yourusername/vork/issues)
- 📚 [llama.cpp Documentation](https://github.com/ggerganov/llama.cpp)
- 💬 [Discussions](https://github.com/yourusername/vork/discussions)

## 🖼️ Vision Support (Image Analysis)

Vork supports analyzing images with vision-capable models! Perfect for:
- 📸 Analyzing GUI screenshots and mockups
- 🎨 Understanding UI/UX layouts
- 📝 Reading text from images
- 🔍 Describing visual content

**Automated Setup (CPU-Optimized):**
```bash
# One-command setup for vision on CPU
./scripts/setup-vision-2b.sh
```

This downloads LLaVA 1.6 (7B Q4_K_S) and configures Vork for CPU-only vision support.

**Manual Setup:**
```bash
# 1. Download a vision model (LLaVA 1.6 recommended)
wget https://huggingface.co/cjpais/llava-v1.6-mistral-7b-gguf/resolve/main/llava-v1.6-mistral-7b.Q4_K_M.gguf
wget https://huggingface.co/cjpais/llava-v1.6-mistral-7b-gguf/resolve/main/mmproj-model-f16.gguf

# 2. Update config.toml
extra_args = "--mmproj /path/to/mmproj-model-f16.gguf"

# 3. Use it!
vork "analyze screenshot.png and describe the UI"
```

**📖 Guides:**
- [scripts/README.md](scripts/README.md) - Automated setup scripts
- [docs/VISION_SETUP.md](docs/VISION_SETUP.md) - Complete manual setup, GPU optimization, model comparisons

## 🆚 Comparison: Vork vs OpenAI Codex

| Feature | OpenAI Codex | Vork |
|---------|--------------|------|
| Interactive chat | ✅ | ✅ |
| Initial prompts | ✅ | ✅ |
| Non-interactive mode | ✅ | ✅ |
| Session management | ✅ | ✅ |
| Approval system | ✅ | ✅ |
| Sandbox modes | ✅ | ✅ |
| Tool calling | ✅ | ✅ |
| JSON output | ✅ | ✅ |
| **Specialized agents** | ❌ | ✅ **14 agents** |
| **Auto agent selection** | ❌ | ✅ **Smart selection** |
| **Custom agents** | ❌ | ✅ **User-created** |
| **Local execution** | ❌ Cloud | ✅ **100% local** |
| **Privacy** | ❌ Data to cloud | ✅ **Never leaves machine** |
| **Cost** | ❌ API fees | ✅ **Free** |
| **Model choice** | ❌ Fixed | ✅ **Any GGUF model** |
| **Customizable** | ❌ | ✅ **Full source access** |

## 🌟 Why Vork?

### 🔒 **Privacy First**
All processing happens on your hardware. Your code never leaves your machine.

### 💰 **Zero Cost**
No API fees, no subscriptions. Run unlimited queries on your own hardware.

### 🎛️ **Complete Control**
Choose any model, tune parameters, customize agents, modify source code.

### ⚡ **No Latency**
Local GPU inference means instant responses with no network round-trips.

### 🤖 **Specialized Agents**
14 expert agents optimized for different tasks, with automatic selection.

### 🛠️ **Extensible**
Create custom agents, integrate with your tools, modify behavior.

---

<p align="center">
  Made with 🐴 by the Vork team
</p>

<p align="center">
  <sub>Vork - The noble steed of your local coding adventures</sub>
</p>
