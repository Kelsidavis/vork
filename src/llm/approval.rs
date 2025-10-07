use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};

use crate::config::{ApprovalPolicy, SandboxMode};

pub struct ApprovalSystem {
    policy: ApprovalPolicy,
    sandbox_mode: SandboxMode,
}

impl ApprovalSystem {
    pub fn new(policy: ApprovalPolicy, sandbox_mode: SandboxMode) -> Self {
        Self {
            policy,
            sandbox_mode,
        }
    }

    pub fn should_approve_write(&self, path: &str) -> Result<bool> {
        match self.sandbox_mode {
            SandboxMode::ReadOnly => {
                println!(
                    "{} Write operation blocked in read-only mode: {}",
                    "⚠️".yellow(),
                    path
                );
                Ok(false)
            }
            SandboxMode::WorkspaceWrite => match self.policy {
                ApprovalPolicy::Auto => {
                    // Auto-approve writes within workspace
                    if self.is_within_workspace(path) {
                        Ok(true)
                    } else {
                        self.prompt_user(&format!("Write file outside workspace: {}", path))
                    }
                }
                ApprovalPolicy::ReadOnly => {
                    println!("{} Write operation requires approval: {}", "⚠️".yellow(), path);
                    self.prompt_user(&format!("Write file: {}", path))
                }
                ApprovalPolicy::AlwaysAsk => {
                    self.prompt_user(&format!("Write file: {}", path))
                }
                ApprovalPolicy::Never => Ok(true),
            },
            SandboxMode::DangerFullAccess => match self.policy {
                ApprovalPolicy::AlwaysAsk => {
                    self.prompt_user(&format!("Write file: {}", path))
                }
                ApprovalPolicy::ReadOnly => {
                    self.prompt_user(&format!("Write file: {}", path))
                }
                _ => Ok(true),
            },
        }
    }

    pub fn should_approve_bash(&self, command: &str) -> Result<bool> {
        match self.sandbox_mode {
            SandboxMode::ReadOnly => {
                println!(
                    "{} Bash execution blocked in read-only mode: {}",
                    "⚠️".yellow(),
                    command
                );
                Ok(false)
            }
            SandboxMode::WorkspaceWrite => match self.policy {
                ApprovalPolicy::Auto => {
                    // Check if command is dangerous
                    if self.is_dangerous_command(command) {
                        self.prompt_user(&format!("Execute potentially dangerous command: {}", command))
                    } else {
                        // Auto-approve non-dangerous commands
                        Ok(true)
                    }
                }
                ApprovalPolicy::ReadOnly => {
                    self.prompt_user(&format!("Execute command: {}", command))
                }
                ApprovalPolicy::AlwaysAsk => {
                    self.prompt_user(&format!("Execute command: {}", command))
                }
                ApprovalPolicy::Never => Ok(true),
            },
            SandboxMode::DangerFullAccess => match self.policy {
                ApprovalPolicy::AlwaysAsk => {
                    self.prompt_user(&format!("Execute command: {}", command))
                }
                ApprovalPolicy::ReadOnly => {
                    self.prompt_user(&format!("Execute command: {}", command))
                }
                ApprovalPolicy::Never => {
                    // Still check for truly dangerous commands even in Never mode
                    if self.is_critical_dangerous_command(command) {
                        self.prompt_user(&format!("Execute critical system command: {}", command))
                    } else {
                        Ok(true)
                    }
                }
                _ => Ok(true),
            },
        }
    }

    fn is_within_workspace(&self, path: &str) -> bool {
        // Check if path starts with ./ or doesn't start with /
        let path = std::path::Path::new(path);
        !path.is_absolute() || path.starts_with(std::env::current_dir().unwrap_or_default())
    }

    fn is_dangerous_command(&self, command: &str) -> bool {
        let dangerous_patterns = [
            "rm -rf",
            "rm -fr",
            "sudo",
            "shutdown",
            "reboot",
            "mkfs",
            "dd if=",
            "format",
            "> /dev/",
            "curl",
            "wget",
            "nc ",
            "netcat",
        ];

        dangerous_patterns
            .iter()
            .any(|pattern| command.contains(pattern))
    }

    fn is_critical_dangerous_command(&self, command: &str) -> bool {
        // Only truly critical system-level commands that need approval
        let critical_patterns = [
            "sudo",
            "shutdown",
            "reboot",
            "mkfs",
            "dd if=",
            "format",
            "> /dev/",
        ];

        critical_patterns
            .iter()
            .any(|pattern| command.contains(pattern))
    }

    fn prompt_user(&self, message: &str) -> Result<bool> {
        println!("\n{} {}", "🔒".yellow().bold(), message.yellow());
        print!("{} [y/N]: ", "Approve?".cyan().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let approved = matches!(input.trim().to_lowercase().as_str(), "y" | "yes");

        if approved {
            println!("{}", "✓ Approved".green());
        } else {
            println!("{}", "✗ Denied".red());
        }

        Ok(approved)
    }
}
