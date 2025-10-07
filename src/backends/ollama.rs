use super::{Backend, ModelInfo};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct OllamaBackend {
    api_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    size: i64,
    modified_at: String,
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Serialize)]
struct PullRequest {
    name: String,
}

impl OllamaBackend {
    pub fn new() -> Self {
        Self {
            api_url: "http://localhost:11434".to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn format_size(bytes: i64) -> String {
        const GB: i64 = 1_073_741_824;
        const MB: i64 = 1_048_576;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

#[async_trait]
impl Backend for OllamaBackend {
    async fn is_available(&self) -> bool {
        self.client
            .get(&format!("{}/api/tags", self.api_url))
            .send()
            .await
            .is_ok()
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let response = self
            .client
            .get(&format!("{}/api/tags", self.api_url))
            .send()
            .await
            .context("Failed to connect to Ollama API")?;

        let list: ListResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        Ok(list
            .models
            .into_iter()
            .map(|m| ModelInfo {
                name: m.name,
                size: Some(Self::format_size(m.size)),
                modified: Some(m.modified_at),
                backend: "ollama".to_string(),
            })
            .collect())
    }

    async fn install_model(&self, model: &str) -> Result<()> {
        use colored::Colorize;
        use indicatif::{ProgressBar, ProgressStyle};

        println!("{} {}", "Pulling model:".green().bold(), model);

        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        spinner.set_message("Downloading...");

        let response = self
            .client
            .post(&format!("{}/api/pull", self.api_url))
            .json(&PullRequest {
                name: model.to_string(),
            })
            .send()
            .await
            .context("Failed to pull model")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to pull model: {}", response.status());
        }

        spinner.finish_with_message("Done!");
        println!("{} {}", "Successfully installed:".green().bold(), model);

        Ok(())
    }

    async fn remove_model(&self, model: &str) -> Result<()> {
        use colored::Colorize;

        #[derive(Serialize)]
        struct DeleteRequest {
            name: String,
        }

        let response = self
            .client
            .delete(&format!("{}/api/delete", self.api_url))
            .json(&DeleteRequest {
                name: model.to_string(),
            })
            .send()
            .await
            .context("Failed to delete model")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to delete model: {}", response.status());
        }

        println!("{} {}", "Removed model:".green().bold(), model);

        Ok(())
    }

    async fn run_model(&self, model: &str, _port: u16) -> Result<()> {
        use colored::Colorize;

        println!(
            "{} {} {}",
            "Model".cyan(),
            model.yellow().bold(),
            "is managed by Ollama".cyan()
        );
        println!(
            "{}",
            "Ollama serves all models on http://localhost:11434".cyan()
        );
        println!(
            "\n{} ollama run {}",
            "To chat with this model, use:".green(),
            model
        );

        Ok(())
    }
}
