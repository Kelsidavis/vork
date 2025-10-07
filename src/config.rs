use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_backend: String,
    pub ollama: OllamaConfig,
    pub llamacpp: LlamaCppConfig,
    #[serde(default)]
    pub assistant: AssistantConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantConfig {
    pub server_url: String,
    pub model: String,
    pub approval_policy: ApprovalPolicy,
    pub sandbox_mode: SandboxMode,
    pub require_git_repo: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalPolicy {
    Auto,
    ReadOnly,
    AlwaysAsk,
    Never,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl Default for AssistantConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".to_string(),
            model: "unknown".to_string(),
            approval_policy: ApprovalPolicy::Never,
            sandbox_mode: SandboxMode::DangerFullAccess,
            require_git_repo: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaConfig {
    pub enabled: bool,
    pub api_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlamaCppConfig {
    pub enabled: bool,
    pub models_dir: String,
    pub binary_path: Option<String>,
    pub context_size: u32,
    #[serde(default = "default_context_limit")]
    pub context_limit: usize,
    pub ngl: u32,
    pub threads: u32,
    pub batch_size: u32,
    pub parallel: u32,
    pub cache_type_k: String,
    pub cache_type_v: String,
    #[serde(default)]
    pub cuda_visible_devices: Option<String>,
}

fn default_context_limit() -> usize {
    32768
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_backend: "ollama".to_string(),
            ollama: OllamaConfig {
                enabled: true,
                api_url: "http://localhost:11434".to_string(),
            },
            llamacpp: LlamaCppConfig {
                enabled: true,
                models_dir: "/media/k/vbox/models".to_string(),
                binary_path: Some("/home/k/llama.cpp/build/bin/llama-server".to_string()),
                context_size: 42768,
                context_limit: 32768,
                ngl: 48,
                threads: 20,
                batch_size: 170,
                parallel: 8,
                cache_type_k: "bf16".to_string(),
                cache_type_v: "bf16".to_string(),
                cuda_visible_devices: None,
            },
            assistant: AssistantConfig::default(),
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(".vork"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&path)
            .context("Failed to read config file")?;

        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)
            .context("Failed to create config directory")?;

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(Self::config_path()?, content)
            .context("Failed to write config file")?;

        Ok(())
    }
}
