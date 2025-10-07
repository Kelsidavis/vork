use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::agents::Agent;

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkResults {
    timestamp: String,
    fastest_preset: String,
    largest_context_preset: String,
    best_reasoning_preset: String,
    presets: Vec<PresetStats>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PresetStats {
    name: String,
    avg_tokens_per_second: f64,
    context_size: u32,
}

pub fn execute() -> Result<()> {
    println!("{}", "=== Model Presets & Agent Assignments ===".green().bold());
    println!();

    // Load benchmark results if available
    let config_dir = Config::config_dir()?;
    let benchmark_path = config_dir.join("benchmark_results.json");

    if !benchmark_path.exists() {
        println!("{}", "⚠️  No benchmark results found.".yellow());
        println!("   Run {} to benchmark your models first.", "vork benchmark".cyan());
        println!();
        return Ok(());
    }

    let benchmark_json = std::fs::read_to_string(&benchmark_path)?;
    let benchmark: BenchmarkResults = serde_json::from_str(&benchmark_json)?;

    println!("{}", "📊 Benchmark Results:".cyan().bold());
    println!("   Timestamp: {}", benchmark.timestamp.yellow());
    println!();
    println!("   ⚡ Fastest: {} ({:.1} tok/s avg)",
        benchmark.fastest_preset.green().bold(),
        benchmark.presets.iter()
            .find(|p| p.name == benchmark.fastest_preset)
            .map(|p| p.avg_tokens_per_second)
            .unwrap_or(0.0)
    );
    println!("   📄 Largest Context: {} ({}k tokens)",
        benchmark.largest_context_preset.green().bold(),
        benchmark.presets.iter()
            .find(|p| p.name == benchmark.largest_context_preset)
            .map(|p| p.context_size / 1024)
            .unwrap_or(0)
    );
    println!("   🧠 Best Reasoning: {}",
        benchmark.best_reasoning_preset.green().bold()
    );
    println!();

    // Show all presets with stats
    println!("{}", "Available Presets:".cyan().bold());
    for preset in &benchmark.presets {
        println!("   • {} - {:.1} tok/s, {}k context",
            preset.name.green(),
            preset.avg_tokens_per_second,
            preset.context_size / 1024
        );
    }
    println!();

    // Show agent assignments
    println!("{}", "🤖 Agent → Preset Assignments:".cyan().bold());
    let agents = Agent::list_agents()?;

    if agents.is_empty() {
        println!("   {}", "No agents found.".yellow());
        return Ok(());
    }

    // Group by preset
    use std::collections::HashMap;
    let mut preset_to_agents: HashMap<String, Vec<String>> = HashMap::new();

    for agent_name in &agents {
        if let Ok(agent) = Agent::load(agent_name) {
            if let Some(preset) = agent.preferred_preset {
                preset_to_agents.entry(preset)
                    .or_insert_with(Vec::new)
                    .push(agent_name.clone());
            }
        }
    }

    // Display grouped by preset
    for preset in &benchmark.presets {
        if let Some(agents) = preset_to_agents.get(&preset.name) {
            println!();
            println!("   {} ({:.1} tok/s):",
                preset.name.green().bold(),
                preset.avg_tokens_per_second
            );
            for agent in agents {
                println!("      • {}", agent.cyan());
            }
        }
    }

    println!();
    println!("{}", "Commands:".yellow().bold());
    println!("   {} - Re-run benchmark and update assignments", "vork benchmark".cyan());
    println!("   {} - List all agents", "vork agents".cyan());
    println!("   {} - Use specific agent", "vork --agent <name>".cyan());

    Ok(())
}
