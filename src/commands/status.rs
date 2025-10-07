use anyhow::Result;
use colored::Colorize;
use crate::backends;

pub async fn execute() -> Result<()> {
    println!("{}", "LLM Backend Status:".green().bold());
    println!();

    let backends = backends::detect_backends().await;

    for (name, available) in backends {
        let status = if available {
            format!("{} running", "●".green())
        } else {
            format!("{} not available", "○".yellow())
        };

        println!("  {} {}", name.bold(), status);
    }

    println!();

    Ok(())
}
