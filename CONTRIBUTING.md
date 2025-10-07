# Contributing to Vork

Thank you for your interest in contributing to Vork! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/vork.git`
3. Create a feature branch: `git checkout -b feature/amazing-feature`
4. Make your changes
5. Test your changes: `cargo test && cargo build`
6. Commit your changes: `git commit -m 'Add amazing feature'`
7. Push to your fork: `git push origin feature/amazing-feature`
8. Open a Pull Request

## Development Setup

### Prerequisites
- Rust 1.70+ (`rustup install stable`)
- llama.cpp with server support
- A GGUF model for testing

### Building
```bash
cargo build --release
```

### Running Tests
```bash
cargo test
```

### Running Locally
```bash
cargo run -- agents
cargo run -- --agent rust-expert
```

## Code Style

- Follow Rust conventions and idioms
- Use `rustfmt` for formatting: `cargo fmt`
- Use `clippy` for linting: `cargo clippy`
- Write tests for new features
- Document public APIs

## Creating New Agents

To add a new specialized agent:

1. Edit `src/agents/mod.rs`
2. Add your agent to the `create_default_agents()` function
3. Define the agent's:
   - Name (kebab-case)
   - Description
   - System prompt
   - Temperature
   - Color
   - Title with emoji
4. Add auto-selection keywords to `auto_select()` function

Example:
```rust
let my_agent = Agent {
    name: "my-agent".to_string(),
    description: "Short description".to_string(),
    system_prompt: r#"You are..."#.to_string(),
    temperature: 0.7,
    tools_enabled: true,
    color: "cyan".to_string(),
    title: Some("🤖 My Agent".to_string()),
};
my_agent.save()?;
```

## Pull Request Guidelines

- Keep PRs focused on a single feature or fix
- Include tests for new functionality
- Update documentation if needed
- Ensure all tests pass before submitting
- Write clear commit messages
- Reference related issues in PR description

## Reporting Issues

- Use GitHub Issues
- Provide:
  - Clear description of the issue
  - Steps to reproduce
  - Expected vs actual behavior
  - System information (OS, Rust version, model)
  - Relevant logs or error messages

## Feature Requests

- Open a GitHub Issue with `[Feature Request]` prefix
- Describe the feature and its use case
- Explain why it would be valuable

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow

## Questions?

Open a GitHub Discussion or Issue if you have questions about contributing.

Thank you for contributing to Vork! 🐴
