use anyhow::Result;
use colored::Colorize;
use crate::backends::{self, Backend};
use crate::config::Config;

pub async fn execute(installed: bool) -> Result<()> {
    let config = Config::load()?;

    if installed {
        println!("{}", "Installed Models:".green().bold());
        println!();
    } else {
        println!("{}", "Available Backends and Models:".green().bold());
        println!();
    }

    // Check Ollama
    if config.ollama.enabled {
        let ollama = backends::ollama::OllamaBackend::new();

        if ollama.is_available().await {
            match ollama.list_models().await {
                Ok(models) => {
                    if !models.is_empty() {
                        println!("  {} {}", "●".green(), "Ollama".bold());
                        for model in models {
                            let size_str = model.size.unwrap_or_else(|| "unknown".to_string());
                            println!("    {} {}", "→".cyan(), format!("{} ({})", model.name, size_str));
                        }
                        println!();
                    } else if !installed {
                        println!("  {} {} {}", "●".green(), "Ollama".bold(), "(no models installed)".dimmed());
                        println!();
                    }
                }
                Err(e) => {
                    if !installed {
                        println!("  {} {} {}", "○".red(), "Ollama".bold(), format!("(error: {})", e).red());
                        println!();
                    }
                }
            }
        } else if !installed {
            println!("  {} {} {}", "○".yellow(), "Ollama".bold(), "(not running)".dimmed());
            println!("    Run: ollama serve");
            println!();
        }
    }

    // Check llama.cpp
    if config.llamacpp.enabled {
        let llamacpp = backends::llamacpp::LlamaCppBackend::new();

        if llamacpp.is_available().await {
            match llamacpp.list_models().await {
                Ok(models) => {
                    if !models.is_empty() {
                        println!("  {} {}", "●".green(), "llama.cpp".bold());
                        for model in models {
                            let size_str = model.size.unwrap_or_else(|| "unknown".to_string());
                            println!("    {} {}", "→".cyan(), format!("{} ({})", model.name, size_str));
                        }
                        println!();
                    } else if !installed {
                        println!("  {} {} {}", "●".green(), "llama.cpp".bold(), "(no models found)".dimmed());
                        println!("    Models directory: {}", config.llamacpp.models_dir);
                        println!();
                    }
                }
                Err(e) => {
                    if !installed {
                        println!("  {} {} {}", "○".red(), "llama.cpp".bold(), format!("(error: {})", e).red());
                        println!();
                    }
                }
            }
        } else if !installed {
            println!("  {} {} {}", "○".yellow(), "llama.cpp".bold(), "(not found)".dimmed());
            println!();
        }
    }

    Ok(())
}
