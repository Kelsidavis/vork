use anyhow::{Context, Result};
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;

use crate::config::{Config, ApprovalPolicy, SandboxMode};

pub fn execute() -> Result<()> {
    println!("{}", "=== Vork Configuration Setup ===".green().bold());
    println!();

    let mut config = Config::load().unwrap_or_default();

    // Model directory
    println!("{}", "📁 Model Configuration".cyan().bold());
    println!("Current model directory: {}", config.llamacpp.models_dir.yellow());
    print!("Enter new model directory (or press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if !input.is_empty() {
        let expanded = shellexpand::tilde(input).to_string();
        if Path::new(&expanded).exists() {
            config.llamacpp.models_dir = input.to_string();
            println!("{} Model directory updated", "✓".green());
        } else {
            println!("{} Directory doesn't exist: {}", "⚠️".yellow(), expanded);
            print!("Create it? (y/N): ");
            io::stdout().flush()?;
            let mut confirm = String::new();
            io::stdin().read_line(&mut confirm)?;
            if confirm.trim().to_lowercase() == "y" {
                std::fs::create_dir_all(&expanded)?;
                config.llamacpp.models_dir = input.to_string();
                println!("{} Directory created and set", "✓".green());
            }
        }
    }
    println!();

    // Binary path
    println!("{}", "🔧 llama-server Binary".cyan().bold());
    if let Some(ref binary) = config.llamacpp.binary_path {
        println!("Current: {}", binary.yellow());
    } else {
        println!("Current: {}", "Not set".red());
    }
    print!("Enter llama-server binary path (or press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input2 = String::new();
    io::stdin().read_line(&mut input2)?;
    let input = input2.trim();

    if !input.is_empty() {
        let expanded = shellexpand::tilde(input).to_string();
        if Path::new(&expanded).exists() {
            config.llamacpp.binary_path = Some(input.to_string());
            println!("{} Binary path updated", "✓".green());
        } else {
            println!("{} File not found: {}", "⚠️".yellow(), expanded);
        }
    }
    println!();

    // Context size
    println!("{}", "⚙️  Model Parameters".cyan().bold());
    println!("Current context size: {}", config.llamacpp.context_size.to_string().yellow());
    print!("Enter context size (2048-131072, or press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input3 = String::new();
    io::stdin().read_line(&mut input3)?;
    let input = input3.trim();

    if !input.is_empty() {
        if let Ok(size) = input.parse::<u32>() {
            if (2048..=131072).contains(&size) {
                config.llamacpp.context_size = size;
                println!("{} Context size updated", "✓".green());
            } else {
                println!("{} Context size must be between 2048 and 131072", "⚠️".yellow());
            }
        }
    }
    println!();

    // GPU layers
    println!("Current GPU layers (NGL): {}", config.llamacpp.ngl.to_string().yellow());
    print!("Enter GPU layers (0-999, or press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input4 = String::new();
    io::stdin().read_line(&mut input4)?;
    let input = input4.trim();

    if !input.is_empty() {
        if let Ok(ngl) = input.parse::<u32>() {
            config.llamacpp.ngl = ngl;
            println!("{} GPU layers updated", "✓".green());
        }
    }
    println!();

    // Threads
    println!("Current threads: {}", config.llamacpp.threads.to_string().yellow());
    print!("Enter thread count (1-128, or press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input5 = String::new();
    io::stdin().read_line(&mut input5)?;
    let input = input5.trim();

    if !input.is_empty() {
        if let Ok(threads) = input.parse::<u32>() {
            if (1..=128).contains(&threads) {
                config.llamacpp.threads = threads;
                println!("{} Thread count updated", "✓".green());
            }
        }
    }
    println!();

    // Batch size
    println!("Current batch size: {}", config.llamacpp.batch_size.to_string().yellow());
    print!("Enter batch size (32-2048, or press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input6 = String::new();
    io::stdin().read_line(&mut input6)?;
    let input = input6.trim();

    if !input.is_empty() {
        if let Ok(batch) = input.parse::<u32>() {
            if (32..=2048).contains(&batch) {
                config.llamacpp.batch_size = batch;
                println!("{} Batch size updated", "✓".green());
            }
        }
    }
    println!();

    // Assistant settings
    println!("{}", "🤖 Assistant Configuration".cyan().bold());
    println!("Current approval policy: {:?}", config.assistant.approval_policy);
    println!("Options:");
    println!("  1. auto        - Auto-approve workspace edits, ask for external");
    println!("  2. read-only   - Require approval for all modifications");
    println!("  3. always-ask  - Prompt for every operation");
    println!("  4. never       - Full automation (dangerous!)");
    print!("Select (1-4, or press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input7 = String::new();
    io::stdin().read_line(&mut input7)?;
    let input = input7.trim();

    match input {
        "1" => {
            config.assistant.approval_policy = ApprovalPolicy::Auto;
            println!("{} Approval policy: Auto", "✓".green());
        }
        "2" => {
            config.assistant.approval_policy = ApprovalPolicy::ReadOnly;
            println!("{} Approval policy: ReadOnly", "✓".green());
        }
        "3" => {
            config.assistant.approval_policy = ApprovalPolicy::AlwaysAsk;
            println!("{} Approval policy: AlwaysAsk", "✓".green());
        }
        "4" => {
            config.assistant.approval_policy = ApprovalPolicy::Never;
            println!("{} Approval policy: Never", "✓".green());
        }
        _ => {}
    }
    println!();

    // Sandbox mode
    println!("Current sandbox mode: {:?}", config.assistant.sandbox_mode);
    println!("Options:");
    println!("  1. read-only        - No modifications allowed");
    println!("  2. workspace-write  - Can edit current directory only");
    println!("  3. danger-full-access - Full system access");
    print!("Select (1-3, or press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input8 = String::new();
    io::stdin().read_line(&mut input8)?;
    let input = input8.trim();

    match input {
        "1" => {
            config.assistant.sandbox_mode = SandboxMode::ReadOnly;
            println!("{} Sandbox mode: ReadOnly", "✓".green());
        }
        "2" => {
            config.assistant.sandbox_mode = SandboxMode::WorkspaceWrite;
            println!("{} Sandbox mode: WorkspaceWrite", "✓".green());
        }
        "3" => {
            config.assistant.sandbox_mode = SandboxMode::DangerFullAccess;
            println!("{} Sandbox mode: DangerFullAccess", "✓".green());
        }
        _ => {}
    }
    println!();

    // Save config
    config.save()?;

    println!("{}", "=== Configuration Summary ===".green().bold());
    println!();
    println!("{}", "Model Settings:".cyan().bold());
    println!("  Directory: {}", config.llamacpp.models_dir.yellow());
    if let Some(ref binary) = config.llamacpp.binary_path {
        println!("  Binary: {}", binary.yellow());
    }
    println!("  Context: {}", config.llamacpp.context_size.to_string().yellow());
    println!("  GPU Layers: {}", config.llamacpp.ngl.to_string().yellow());
    println!("  Threads: {}", config.llamacpp.threads.to_string().yellow());
    println!("  Batch Size: {}", config.llamacpp.batch_size.to_string().yellow());
    println!();
    println!("{}", "Assistant Settings:".cyan().bold());
    println!("  Approval: {:?}", config.assistant.approval_policy);
    println!("  Sandbox: {:?}", config.assistant.sandbox_mode);
    println!();

    let config_path = Config::config_path()?;
    println!("{} Configuration saved to: {}", "✓".green(), config_path.display().to_string().cyan());
    println!();
    println!("Run {} to start chatting!", "vork".green().bold());

    Ok(())
}
