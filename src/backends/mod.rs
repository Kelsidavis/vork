pub mod ollama;
pub mod llamacpp;

use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Backend {
    async fn is_available(&self) -> bool;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    async fn install_model(&self, model: &str) -> Result<()>;
    async fn remove_model(&self, model: &str) -> Result<()>;
    async fn run_model(&self, model: &str, port: u16) -> Result<()>;
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ModelInfo {
    pub name: String,
    pub size: Option<String>,
    pub modified: Option<String>,
    pub backend: String,
}

pub fn get_backend(name: &str) -> Result<Box<dyn Backend>> {
    match name.to_lowercase().as_str() {
        "ollama" => Ok(Box::new(ollama::OllamaBackend::new())),
        "llamacpp" | "llama.cpp" => Ok(Box::new(llamacpp::LlamaCppBackend::new())),
        _ => anyhow::bail!("Unknown backend: {}", name),
    }
}

pub async fn detect_backends() -> Vec<(String, bool)> {
    let mut backends = vec![];

    let ollama = ollama::OllamaBackend::new();
    backends.push(("ollama".to_string(), ollama.is_available().await));

    let llamacpp = llamacpp::LlamaCppBackend::new();
    backends.push(("llama.cpp".to_string(), llamacpp.is_available().await));

    backends
}
