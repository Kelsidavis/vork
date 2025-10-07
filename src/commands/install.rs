use anyhow::Result;
use colored::Colorize;
use crate::backends;
use crate::config::Config;

pub async fn execute(model: &str, backend: &str) -> Result<()> {
    let config = Config::load()?;

    let backend_name = if backend == "auto" {
        &config.default_backend
    } else {
        backend
    };

    println!(
        "{} {} {} {}",
        "Installing".green().bold(),
        model.yellow(),
        "using".green().bold(),
        backend_name.cyan()
    );
    println!();

    let backend = backends::get_backend(backend_name)?;

    if !backend.is_available().await {
        anyhow::bail!(
            "Backend {} is not available. Please install and start it first.",
            backend_name
        );
    }

    backend.install_model(model).await?;

    Ok(())
}
