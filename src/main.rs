mod cli;
mod clipboard;
mod config;
mod core;
mod platform;

use std::thread;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands, HistoryArgs};
use crate::clipboard::{ClipboardEntry, SharedClipboardState, DEFAULT_HISTORY_LIMIT};
use crate::config::AppConfig;
use crate::core::instance_lock::InstanceLock;
#[cfg(target_os = "linux")]
use crate::platform::tray_indicator;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => run(cli.config),
        Commands::History(HistoryArgs { json }) => print_history(json),
        Commands::ValidateConfig => validate_config(cli.config),
    }
}

fn run(config_path_override: Option<std::path::PathBuf>) -> Result<()> {
    println!("slyboard v{}", env!("CARGO_PKG_VERSION"));
    let _instance_lock = InstanceLock::acquire()?;

    let loaded = AppConfig::load(config_path_override)?;
    let config_path = loaded.path.clone();
    let config = loaded.config;
    config.validate()?;

    println!("Loaded config from {}", config_path.display());
    println!("Running clipboard manager...");

    let shared_state = SharedClipboardState::load_default(DEFAULT_HISTORY_LIMIT)?;

    #[cfg(target_os = "linux")]
    let _app_indicator = tray_indicator::start(shared_state);

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn print_history(json: bool) -> Result<()> {
    let shared_state = SharedClipboardState::load_default(DEFAULT_HISTORY_LIMIT)?;
    let history = shared_state.history_snapshot();

    if json {
        println!("{}", serde_json::to_string(&history)?);
        return Ok(());
    }

    for entry in history {
        println!("{}", format_history_entry(&entry));
    }
    Ok(())
}

fn format_history_entry(entry: &ClipboardEntry) -> String {
    match entry {
        ClipboardEntry::Text { value } => value.clone(),
        ClipboardEntry::Image { width, height, .. } => format!("[image] {}x{}", width, height),
    }
}

fn validate_config(config_path_override: Option<std::path::PathBuf>) -> Result<()> {
    let loaded = AppConfig::load(config_path_override)?;
    loaded.config.validate()?;
    println!("Config is valid: {}", loaded.path.display());
    Ok(())
}
