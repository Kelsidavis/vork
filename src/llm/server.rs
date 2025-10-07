use anyhow::{Context, Result};
use colored::Colorize;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

use crate::config::Config;

pub struct ServerManager {
    config: Config,
}

impl ServerManager {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self {
            config,
        })
    }

    /// Kill any existing llama-server instances
    pub fn kill_existing_servers(&self) -> Result<()> {
        println!("{}", "🔍 Checking for existing llama-server instances...".cyan());

        // Use pkill to kill all llama-server processes
        let output = Command::new("pkill")
            .arg("-9")
            .arg("llama-server")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                println!("{}", "✓ Killed existing llama-server instances".yellow());
            }
            _ => {
                // No processes found or pkill failed, that's okay
            }
        }

        // Also check for any process on the configured port
        let port = 8080; // Default port
        let output = Command::new("lsof")
            .arg("-ti")
            .arg(format!(":{}", port))
            .output();

        if let Ok(out) = output {
            if let Ok(pids) = String::from_utf8(out.stdout) {
                for pid in pids.lines() {
                    let _ = Command::new("kill")
                        .arg("-9")
                        .arg(pid.trim())
                        .output();
                }
            }
        }

        // Give processes time to die
        std::thread::sleep(Duration::from_millis(500));

        Ok(())
    }

    /// Start the model server in the background
    pub async fn start_server(&mut self) -> Result<String> {
        self.kill_existing_servers()?;

        println!("{}", "🚀 Starting llama-server...".green().bold());

        let binary = self
            .config
            .llamacpp
            .binary_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("llama-server binary path not configured"))?;

        // Find the model
        let models_dir = shellexpand::tilde(&self.config.llamacpp.models_dir).to_string();
        let model_files = std::fs::read_dir(&models_dir)
            .context("Failed to read models directory")?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    == Some("gguf")
            })
            .collect::<Vec<_>>();

        let model_path = model_files
            .first()
            .ok_or_else(|| anyhow::anyhow!("No GGUF models found in {}", models_dir))?
            .path();

        let model_name = model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("model");

        println!("{} {}", "📦 Model:".cyan(), model_name.yellow());
        println!("{} {}", "🔧 Binary:".cyan(), binary.cyan());

        let cfg = &self.config.llamacpp;
        let port = 8080;

        println!();
        println!("{}", "Configuration:".cyan().bold());
        println!("  {} {}", "Context Size:".cyan(), cfg.context_size);
        println!("  {} {}", "GPU Layers (NGL):".cyan(), cfg.ngl);
        println!("  {} {}", "Threads:".cyan(), cfg.threads);
        println!("  {} {}", "Batch Size:".cyan(), cfg.batch_size);
        println!("  {} {}", "Port:".cyan(), port);
        println!();

        // Use split-mode "none" if forcing to single GPU, otherwise "layer"
        let split_mode = if cfg.cuda_visible_devices.is_some() {
            "none"
        } else {
            "layer"
        };

        // Start the server process with output redirected to /dev/null
        let mut cmd = Command::new(binary);
        cmd.arg("-m")
            .arg(&model_path)
            .arg("--host")
            .arg("0.0.0.0")
            .arg("--port")
            .arg(port.to_string())
            .arg("-c")
            .arg(cfg.context_size.to_string())
            .arg("--batch-size")
            .arg(cfg.batch_size.to_string())
            .arg("-ngl")
            .arg(cfg.ngl.to_string())
            .arg("--alias")
            .arg(model_name)
            .arg("--split-mode")
            .arg(split_mode);

        // Set main GPU if cuda_visible_devices is specified
        if let Some(ref gpu_index) = cfg.cuda_visible_devices {
            cmd.arg("--main-gpu").arg(gpu_index);
        }

        let child = cmd
            .arg("--jinja")
            .arg("--temp")
            .arg("0.6")
            .arg("--top-p")
            .arg("0.9")
            .arg("--min-p")
            .arg("0.05")
            .arg("--repeat-penalty")
            .arg("1.1")
            .arg("--repeat-last-n")
            .arg("256")
            .arg("--no-warmup")
            .arg("-t")
            .arg(cfg.threads.to_string())
            .arg("--log-disable")  // Disable logging
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .context("Failed to start llama-server")?;

        // Don't store the process - let it run independently
        // This prevents it from being killed when ServerManager is dropped
        std::mem::forget(child);

        println!("{}", "⏳ Waiting for server to be ready...".yellow());

        // Wait for server to be ready
        let client = reqwest::Client::new();
        let server_url = format!("http://localhost:{}", port);

        for i in 0..30 {
            sleep(Duration::from_secs(1)).await;

            if let Ok(response) = client.get(&format!("{}/health", server_url)).send().await {
                if response.status().is_success() {
                    println!("{}", "✓ Server is ready!".green().bold());
                    println!("{} {}", "🌐 URL:".cyan(), server_url.green());
                    println!();
                    return Ok(server_url);
                }
            }

            if i % 5 == 0 && i > 0 {
                println!("  Still waiting... ({}s)", i);
            }
        }

        anyhow::bail!("Server failed to start within 30 seconds")
    }

    /// Check if server is running
    #[allow(dead_code)]
    pub async fn is_server_running(&self, url: &str) -> bool {
        let client = reqwest::Client::new();
        client
            .get(&format!("{}/health", url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

// Server runs independently - we don't kill it on drop
// Users can manually kill with pkill llama-server if needed
