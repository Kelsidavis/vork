# Changelog

All notable changes to Vork will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- 🔬 Online research agent with web search capability (DuckDuckGo)
- 🌐 Web search tool for all agents to find documentation and solutions
- 📊 Detailed file operation status messages (reading, writing, executing)
- 🗂️ Workspace-relative path interpretation (e.g., /docs/ means ./docs/)
- 15 specialized AI agents with auto-selection (added researcher)
- Beautiful TUI interface powered by ratatui
- llama.cpp integration with auto-start server
- Session management and resume capability
- Configurable approval policies (never, auto, always-ask, read-only)
- Sandbox modes (read-only, workspace-write, danger-full-access)
- Tool calling system (read, write, bash, search)
- Performance metrics with tokens/second display
- Window resize support
- Agent management commands (list, create, view)
- Interactive setup wizard
- Comprehensive documentation

### Changed
- All agent prompts now understand workspace-relative paths by default
- Tool execution shows real-time status with emojis and completion messages

### Agents Included
- 🚀 default - General-purpose coding assistant
- 🔬 researcher - Online research with workspace context linking
- 🦀 rust-expert - Rust programming specialist
- 🔍 code-auditor - Code quality and compliance auditor
- 🔬 reverse-engineer - Binary analysis with radare2/Ghidra
- ✏️ code-editor - Precision code modifications
- 🚀 release-manager - Version management and releases
- ⚡ performance-optimizer - Speed and efficiency optimization
- 🛡️ security-auditor - Security vulnerabilities and secure coding
- 🧪 test-writer - Comprehensive test engineering
- 🔧 devops - CI/CD, Docker, Kubernetes, infrastructure
- 🔍 reviewer - Code review and suggestions
- 📝 documenter - Documentation specialist
- 🐛 debugger - Debug and fix issues
- 📋 template - Template for creating custom agents

## [0.1.0] - 2025-10-07

### Initial Release
- First public release of Vork
- Complete local alternative to OpenAI Codex
- 100% private, runs entirely on local hardware
