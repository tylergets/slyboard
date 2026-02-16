use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "slyboard", version, about = "Slyboard daemon")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Optional explicit config path (overrides discovery; useful for Nix store paths).
    #[arg(short = 'c', long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Run the clipboard manager daemon.
    Run,
    /// Print clipboard history from the cache database.
    History(HistoryArgs),
    /// Load and validate config, then exit.
    ValidateConfig,
}

#[derive(Debug, Clone, Args)]
pub struct HistoryArgs {
    /// Emit clipboard history as JSON.
    #[arg(long)]
    pub json: bool,
}
