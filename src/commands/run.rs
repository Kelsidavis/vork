use anyhow::Result;
use colored::Colorize;
use crate::backends::{self, Backend};
use crate::config::Config;

pub async fn execute(model: &str, port: u16) -> Result<()> {
    let config = Config::load()?;

    // Try to find which backend has this model
    let ollama = backends::ollama::OllamaBackend::new();

    if config.ollama.enabled && ollama.is_available().await {
        if let Ok(models) = ollama.list_models().await {
            if models.iter().any(|m| m.name == model) {
                return ollama.run_model(model, port).await;
            }
        }
    }

    // If not found in Ollama, try llama.cpp
    let llamacpp = backends::llamacpp::LlamaCppBackend::new();
    if config.llamacpp.enabled && llamacpp.is_available().await {
        return llamacpp.run_model(model, port).await;
    }

    anyhow::bail!(
        "{} {} {}",
        "Model".red(),
        model.yellow(),
        "not found in any backend".red()
    );
}
