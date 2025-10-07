use anyhow::Result;
use colored::Colorize;
use crate::backends::{self, Backend};
use crate::config::Config;

pub async fn execute(model: &str) -> Result<()> {
    let config = Config::load()?;

    // Try to find which backend has this model
    let ollama = backends::ollama::OllamaBackend::new();

    if config.ollama.enabled && ollama.is_available().await {
        if let Ok(models) = ollama.list_models().await {
            if models.iter().any(|m| m.name == model) {
                println!(
                    "{} {} {} {}",
                    "Removing".red().bold(),
                    model.yellow(),
                    "from".red().bold(),
                    "Ollama".cyan()
                );
                return ollama.remove_model(model).await;
            }
        }
    }

    anyhow::bail!(
        "{} {} {}",
        "Model".red(),
        model.yellow(),
        "not found".red()
    );
}
