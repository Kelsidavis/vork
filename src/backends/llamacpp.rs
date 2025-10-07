use super::{Backend, ModelInfo};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crate::config::Config;

pub struct LlamaCppBackend {
    config: Config,
}

impl LlamaCppBackend {
    pub fn new() -> Self {
        let config = Config::load().unwrap_or_default();
        Self { config }
    }

    fn find_binary(&self) -> Option<String> {
        // First check configured path
        if let Some(ref path) = self.config.llamacpp.binary_path {
            if Path::new(path).exists() {
                return Some(path.clone());
            }
        }

        // Check common locations for llama.cpp binaries
        let possible_names = vec!["llama-server", "llama-cli", "main"];

        for name in possible_names {
            if let Ok(output) = Command::new("which").arg(name).output() {
                if output.status.success() {
                    if let Ok(path) = String::from_utf8(output.stdout) {
                        return Some(path.trim().to_string());
                    }
                }
            }
        }

        None
    }

    fn scan_models_dir(&self) -> Result<Vec<PathBuf>> {
        let models_dir = shellexpand::tilde(&self.config.llamacpp.models_dir).to_string();
        let path = Path::new(&models_dir);

        if !path.exists() {
            return Ok(vec![]);
        }

        let mut models = vec![];

        // Recursively search for .gguf files
        fn scan_dir(dir: &Path, models: &mut Vec<PathBuf>) -> Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    scan_dir(&path, models)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("gguf") {
                    models.push(path);
                }
            }
            Ok(())
        }

        scan_dir(path, &mut models)?;
        Ok(models)
    }

    fn get_model_alias(&self, model_path: &Path) -> String {
        // Extract a clean alias from the model path
        model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_lowercase()
            .replace(['_', '-', '.'], "-")
    }

    pub fn start_server(port: u16) -> Result<()> {
        // Load fresh config
        let config = Config::load()?;
        let backend = Self { config };

        let binary = backend
            .find_binary()
            .ok_or_else(|| anyhow::anyhow!("llama.cpp binary not found"))?;

        // Get the model from config
        let model = &backend.config.assistant.model;

        // Find the model file
        let models = backend.scan_models_dir()?;
        let model_path = models
            .iter()
            .find(|p| backend.get_model_alias(p).contains(model) || p.file_name().and_then(|n| n.to_str()).map(|n| n.contains(model)).unwrap_or(false))
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found", model))?;

        let cfg = &backend.config.llamacpp;

        let mut cmd = Command::new(&binary);

        // Use split-mode "none" if forcing to single GPU, otherwise "layer"
        let split_mode = if cfg.cuda_visible_devices.is_some() {
            "none"
        } else {
            "layer"
        };

        cmd.arg("-m").arg(model_path)
            .arg("--host").arg("0.0.0.0")
            .arg("--port").arg(port.to_string())
            .arg("-c").arg(cfg.context_size.to_string())
            .arg("--batch-size").arg(cfg.batch_size.to_string())
            .arg("-ngl").arg(cfg.ngl.to_string())
            .arg("--alias").arg(model)
            .arg("--split-mode").arg(split_mode)
            .arg("--jinja")
            .arg("--temp").arg("0.6")
            .arg("--top-p").arg("0.9")
            .arg("--min-p").arg("0.05")
            .arg("--repeat-penalty").arg("1.1")
            .arg("--repeat-last-n").arg("256")
            .arg("--no-warmup")
            .arg("-t").arg(cfg.threads.to_string())
            .arg("--verbose");

        // Set main GPU if cuda_visible_devices is specified
        // Note: cuda_visible_devices is used as the main GPU index
        if let Some(ref gpu_index) = cfg.cuda_visible_devices {
            cmd.arg("--main-gpu").arg(gpu_index);
        }

        // Redirect stdout/stderr to prevent UI corruption during TUI mode
        cmd.stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());

        // Spawn in background
        cmd.spawn()
            .context("Failed to spawn llama-server")?;

        Ok(())
    }
}

#[async_trait]
impl Backend for LlamaCppBackend {
    async fn is_available(&self) -> bool {
        self.find_binary().is_some()
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let models = self.scan_models_dir()?;

        Ok(models
            .into_iter()
            .map(|path| {
                let size = fs::metadata(&path)
                    .ok()
                    .map(|m| {
                        let bytes = m.len();
                        if bytes >= 1_073_741_824 {
                            format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
                        } else if bytes >= 1_048_576 {
                            format!("{:.1} MB", bytes as f64 / 1_048_576.0)
                        } else {
                            format!("{} bytes", bytes)
                        }
                    });

                ModelInfo {
                    name: self.get_model_alias(&path),
                    size,
                    modified: None,
                    backend: "llama.cpp".to_string(),
                }
            })
            .collect())
    }

    async fn install_model(&self, _model: &str) -> Result<()> {
        anyhow::bail!("llama.cpp backend does not support automatic model installation. Please download GGUF models manually to: {}", self.config.llamacpp.models_dir);
    }

    async fn remove_model(&self, _model: &str) -> Result<()> {
        anyhow::bail!("llama.cpp backend does not support model removal through vork");
    }

    async fn run_model(&self, model: &str, port: u16) -> Result<()> {
        use colored::Colorize;

        let binary = self
            .find_binary()
            .ok_or_else(|| anyhow::anyhow!("llama.cpp binary not found"))?;

        // Find the model file
        let models = self.scan_models_dir()?;
        let model_path = models
            .iter()
            .find(|p| self.get_model_alias(p) == model)
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found", model))?;

        let cfg = &self.config.llamacpp;

        println!("==========================================");
        println!("{} {}", "🚀 Launching".green().bold(), model.yellow());
        println!("{} {}", "Model:".cyan(), model_path.display());
        println!("{} {} | {} {}", "Context:".cyan(), cfg.context_size, "NGL:".cyan(), cfg.ngl);
        println!("{} {} | {} {}", "Threads:".cyan(), cfg.threads, "Batch:".cyan(), cfg.batch_size);
        println!("==========================================");
        println!();

        let mut cmd = Command::new(&binary);

        // Use split-mode "none" if forcing to single GPU, otherwise "layer"
        let split_mode = if cfg.cuda_visible_devices.is_some() {
            "none"
        } else {
            "layer"
        };

        cmd.arg("-m").arg(model_path)
            .arg("--host").arg("0.0.0.0")
            .arg("--port").arg(port.to_string())
            .arg("-c").arg(cfg.context_size.to_string())
            .arg("--batch-size").arg(cfg.batch_size.to_string())
            .arg("-ngl").arg(cfg.ngl.to_string())
            .arg("--alias").arg(model)
            .arg("--split-mode").arg(split_mode)
            .arg("--jinja")
            .arg("--temp").arg("0.6")
            .arg("--top-p").arg("0.9")
            .arg("--min-p").arg("0.05")
            .arg("--repeat-penalty").arg("1.1")
            .arg("--repeat-last-n").arg("256")
            .arg("--no-warmup")
            .arg("-t").arg(cfg.threads.to_string())
            .arg("--verbose");

        // Set main GPU if cuda_visible_devices is specified
        // Note: cuda_visible_devices is used as the main GPU index
        if let Some(ref gpu_index) = cfg.cuda_visible_devices {
            cmd.arg("--main-gpu").arg(gpu_index);
        }

        println!("{} {:?}", "Executing:".green().bold(), cmd);
        println!();

        let status = cmd
            .status()
            .context("Failed to execute llama-server")?;

        if !status.success() {
            anyhow::bail!("llama-server exited with status: {}", status);
        }

        Ok(())
    }
}
