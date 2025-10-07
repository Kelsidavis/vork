use anyhow::Result;
use colored::Colorize;
use crate::config::Config;

pub fn execute(show_path: bool) -> Result<()> {
    if show_path {
        let path = Config::config_path()?;
        println!("{}", path.display());
    } else {
        let config = Config::load()?;
        let config_str = toml::to_string_pretty(&config)?;

        println!("{}", "Vork Configuration:".green().bold());
        println!();
        println!("{}", config_str);
        println!();
        println!("{} {}", "Config file:".cyan(), Config::config_path()?.display());
    }

    Ok(())
}
