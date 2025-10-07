use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};

use crate::agents::Agent;

pub fn execute(list: bool, create: bool, agent_name: Option<String>) -> Result<()> {
    // Initialize default agents if agents dir doesn't exist
    let agents_dir = Agent::agents_dir()?;
    if !agents_dir.exists() {
        println!("{}", "Creating default agents...".cyan());
        Agent::create_default_agents()?;
        println!("{} Default agents created", "✓".green());
        println!();
    }

    if list || agent_name.is_none() {
        // List all agents
        println!("{}", "=== Available Agents ===".green().bold());
        println!();

        let agents = Agent::list_agents()?;
        if agents.is_empty() {
            println!("{}", "No agents found. Creating defaults...".yellow());
            Agent::create_default_agents()?;
            let agents = Agent::list_agents()?;
            for name in agents {
                display_agent(&name)?;
            }
        } else {
            for name in agents {
                display_agent(&name)?;
            }
        }

        println!();
        println!("{}", "Usage:".cyan().bold());
        println!("  {} --agent <name>    Start vork with a specific agent", "vork".green());
        println!("  {} agents <name>         Show details for an agent", "vork".green());
        println!("  {} agents --create       Create a new custom agent", "vork".green());
        println!();
        println!("{}", "Agents directory:".cyan());
        println!("  {}", agents_dir.display().to_string().yellow());
        println!();
        println!("{}", "To customize:".cyan());
        println!("  1. Copy template.json to new-agent.json");
        println!("  2. Edit the system_prompt and settings");
        println!("  3. Use with: vork --agent new-agent");

        return Ok(());
    }

    if create {
        // Create new agent interactively
        println!("{}", "=== Create New Agent ===".green().bold());
        println!();

        print!("Agent name: ");
        io::stdout().flush()?;
        let mut name = String::new();
        io::stdin().read_line(&mut name)?;
        let name = name.trim().to_lowercase().replace(' ', "-");

        print!("Description: ");
        io::stdout().flush()?;
        let mut description = String::new();
        io::stdin().read_line(&mut description)?;
        let description = description.trim().to_string();

        println!();
        println!("{}", "Enter system prompt (end with Ctrl+D or empty line):".cyan());
        let mut system_prompt = String::new();
        loop {
            let mut line = String::new();
            if io::stdin().read_line(&mut line)? == 0 {
                break; // EOF
            }
            if line.trim().is_empty() && !system_prompt.is_empty() {
                break;
            }
            system_prompt.push_str(&line);
        }

        print!("Temperature (0.0-1.0, default 0.7): ");
        io::stdout().flush()?;
        let mut temp_str = String::new();
        io::stdin().read_line(&mut temp_str)?;
        let temperature = temp_str.trim().parse::<f32>().unwrap_or(0.7);

        print!("Enable tools? (Y/n): ");
        io::stdout().flush()?;
        let mut tools_str = String::new();
        io::stdin().read_line(&mut tools_str)?;
        let tools_enabled = !matches!(tools_str.trim().to_lowercase().as_str(), "n" | "no");

        print!("Color (red/green/blue/yellow/cyan/magenta, default cyan): ");
        io::stdout().flush()?;
        let mut color_str = String::new();
        io::stdin().read_line(&mut color_str)?;
        let color = if color_str.trim().is_empty() {
            "cyan".to_string()
        } else {
            color_str.trim().to_lowercase()
        };

        print!("Title (optional, e.g. '🤖 My Agent'): ");
        io::stdout().flush()?;
        let mut title_str = String::new();
        io::stdin().read_line(&mut title_str)?;
        let title = if title_str.trim().is_empty() {
            None
        } else {
            Some(title_str.trim().to_string())
        };

        let agent = Agent {
            name: name.clone(),
            description,
            system_prompt: system_prompt.trim().to_string(),
            temperature,
            tools_enabled,
            color,
            title,
        };

        agent.save()?;
        println!();
        println!("{} Agent '{}' created!", "✓".green(), name.green().bold());
        println!("Use with: {} --agent {}", "vork".green(), name);

        return Ok(());
    }

    if let Some(name) = agent_name {
        // Show specific agent details
        let agent = Agent::load(&name)?;
        println!("{}", format!("=== Agent: {} ===", agent.name).green().bold());
        println!();
        println!("{} {}", "Description:".cyan().bold(), agent.description);
        println!("{} {}", "Temperature:".cyan().bold(), agent.temperature);
        println!("{} {}", "Tools Enabled:".cyan().bold(), agent.tools_enabled);
        println!();
        println!("{}", "System Prompt:".cyan().bold());
        println!("{}", "─".repeat(60).cyan());
        println!("{}", agent.system_prompt);
        println!("{}", "─".repeat(60).cyan());
        println!();
        println!("Use with: {} --agent {}", "vork".green(), name);
    }

    Ok(())
}

fn display_agent(name: &str) -> Result<()> {
    match Agent::load(name) {
        Ok(agent) => {
            let icon = if agent.tools_enabled { "🛠️ " } else { "💬 " };
            println!("{}{}", icon, agent.name.green().bold());
            println!("  {}", agent.description.cyan());
            println!();
        }
        Err(_) => {
            println!("{} {} (failed to load)", "⚠️".yellow(), name);
        }
    }
    Ok(())
}
