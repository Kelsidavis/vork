use clap::{Parser, Subcommand};
use anyhow::Result;

mod config;
mod backends;
mod commands;
mod llm;
mod agents;

#[derive(Parser)]
#[command(name = "vork")]
#[command(about = "AI-powered coding assistant and LLM manager", long_about = None)]
struct Cli {
    /// Initial prompt for interactive chat (e.g., vork "fix bugs")
    #[arg(value_name = "PROMPT")]
    prompt: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Server URL (for chat mode)
    #[arg(short, long, global = true)]
    server: Option<String>,

    /// Model name (for chat mode)
    #[arg(short, long, global = true)]
    model: Option<String>,

    /// Agent to use (e.g., rust-expert, reviewer, debugger)
    #[arg(short, long, global = true)]
    agent: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available LLM backends and models
    List {
        /// Show only installed models
        #[arg(short, long)]
        installed: bool,
    },
    /// Install a model
    Install {
        /// Model name (e.g., llama3.2, mistral)
        model: String,
        /// Backend to use (ollama, llamacpp, auto)
        #[arg(short, long, default_value = "auto")]
        backend: String,
    },
    /// Run/serve a model
    Run {
        /// Model name
        model: String,
        /// Port to serve on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Remove a model
    Remove {
        /// Model name to remove
        model: String,
    },
    /// Show vork configuration
    Config {
        /// Show config file path
        #[arg(short, long)]
        path: bool,
    },
    /// Interactive configuration setup
    Setup,
    /// Manage AI agents
    Agents {
        /// List all available agents
        #[arg(short, long)]
        list: bool,
        /// Create a new agent interactively
        #[arg(short, long)]
        create: bool,
        /// Show details for a specific agent
        agent_name: Option<String>,
    },
    /// Check status of LLM backends
    Status,
    /// Interactive chat with AI coding assistant
    Chat {
        /// Server URL (default: http://localhost:8080)
        #[arg(short, long)]
        server: Option<String>,
        /// Model name
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Ask a one-off question to the AI assistant
    Ask {
        /// The question to ask
        question: String,
        /// Disable tool calling (get direct response only)
        #[arg(long)]
        no_tools: bool,
    },
    /// Resume a previous session
    Resume {
        /// Session ID to resume
        session_id: Option<String>,
        /// Resume the last session
        #[arg(short, long)]
        last: bool,
    },
    /// Non-interactive mode (read-only by default)
    Exec {
        /// The task to execute
        prompt: String,
        /// Allow file edits and full access
        #[arg(long)]
        full_auto: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // If no subcommand, default to TUI mode with auto-server-start
    let command = cli.command.unwrap_or_else(|| {
        // Default to TUI mode (will auto-start server)
        Commands::Chat {
            server: cli.server.clone(),
            model: cli.model.clone(),
        }
    });

    match command {
        Commands::List { installed } => {
            commands::list::execute(installed).await?;
        }
        Commands::Install { model, backend } => {
            commands::install::execute(&model, &backend).await?;
        }
        Commands::Run { model, port } => {
            commands::run::execute(&model, port).await?;
        }
        Commands::Remove { model } => {
            commands::remove::execute(&model).await?;
        }
        Commands::Config { path } => {
            commands::config::execute(path)?;
        }
        Commands::Setup => {
            commands::setup::execute()?;
        }
        Commands::Agents { list, create, agent_name } => {
            commands::agents::execute(list, create, agent_name)?;
        }
        Commands::Status => {
            commands::status::execute().await?;
        }
        Commands::Chat { server, model } => {
            // Use TUI mode by default, only fall back to old chat if explicitly requested
            if cli.prompt.is_some() {
                // If prompt provided, use simple chat with initial prompt
                commands::chat::execute(server, model, cli.prompt).await?;
            } else {
                // Use fancy TUI interface with auto-server-start
                commands::tui::execute(server, model, cli.agent).await?;
            }
        }
        Commands::Ask {
            question,
            no_tools,
        } => {
            commands::ask::execute(&question, cli.server, cli.model, no_tools).await?;
        }
        Commands::Resume { session_id, last } => {
            commands::resume::execute(session_id, last).await?;
        }
        Commands::Exec {
            prompt,
            full_auto,
            json,
        } => {
            commands::exec::execute(&prompt, cli.server, cli.model, full_auto, json).await?;
        }
    }

    Ok(())
}
