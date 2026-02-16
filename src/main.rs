mod cli;
mod clipboard;
mod config;
mod core;
mod platform;

use std::thread;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use serde::Serialize;

use crate::cli::{Cli, Commands, HistoryArgs};
use crate::clipboard::{ClipboardEntry, SharedClipboardState, DEFAULT_HISTORY_LIMIT};
use crate::config::AppConfig;
use crate::core::active_window::ActiveWindowContext;
use crate::core::capture_control::{is_capture_paused, set_capture_paused};
use crate::core::instance_lock::InstanceLock;
#[cfg(target_os = "linux")]
use crate::platform::tray_indicator;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => run(cli.config),
        Commands::History(HistoryArgs { json, images }) => print_history(json, images),
        Commands::ClearHistory => clear_history(),
        Commands::PauseCapture => pause_capture(),
        Commands::ResumeCapture => resume_capture(),
        Commands::CaptureStatus => print_capture_status(),
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
    if is_capture_paused()? {
        println!("Clipboard capture is currently paused.");
    }

    let shared_state = SharedClipboardState::load_default(DEFAULT_HISTORY_LIMIT)?;

    #[cfg(target_os = "linux")]
    let _app_indicator = tray_indicator::start(shared_state, config.clipboard.clone());

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn print_history(json: bool, include_images: bool) -> Result<()> {
    let shared_state = SharedClipboardState::load_default(DEFAULT_HISTORY_LIMIT)?;
    let history = shared_state.history_snapshot();

    if json {
        let serializable: Vec<SerializableHistoryEntry> = history
            .iter()
            .enumerate()
            .map(|(id, entry)| SerializableHistoryEntry::new(id, entry, include_images))
            .collect();
        println!("{}", serde_json::to_string(&serializable)?);
        return Ok(());
    }

    for (id, entry) in history.iter().enumerate() {
        println!("{}", format_history_entry(id, entry));
    }
    Ok(())
}

fn clear_history() -> Result<()> {
    let shared_state = SharedClipboardState::load_default(DEFAULT_HISTORY_LIMIT)?;
    shared_state.clear_history()?;
    println!("Clipboard history cleared.");
    Ok(())
}

fn pause_capture() -> Result<()> {
    if is_capture_paused()? {
        println!("Clipboard capture is already paused.");
        return Ok(());
    }

    set_capture_paused(true)?;
    println!("Clipboard capture paused.");
    Ok(())
}

fn resume_capture() -> Result<()> {
    if !is_capture_paused()? {
        println!("Clipboard capture is already running.");
        return Ok(());
    }

    set_capture_paused(false)?;
    println!("Clipboard capture resumed.");
    Ok(())
}

fn print_capture_status() -> Result<()> {
    if is_capture_paused()? {
        println!("paused");
    } else {
        println!("running");
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct SerializableHistoryEntry {
    id: usize,
    #[serde(flatten)]
    entry: SerializableClipboardEntry,
}

impl SerializableHistoryEntry {
    fn new(id: usize, entry: &ClipboardEntry, include_images: bool) -> Self {
        Self {
            id,
            entry: SerializableClipboardEntry::from_entry(entry, include_images),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SerializableClipboardEntry {
    Text {
        value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        source_window: Option<ActiveWindowContext>,
    },
    Image {
        width: i32,
        height: i32,
        rowstride: i32,
        has_alpha: bool,
        bits_per_sample: i32,
        channels: i32,
        pixel_bytes: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        pixels: Option<Vec<u8>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source_window: Option<ActiveWindowContext>,
    },
}

impl SerializableClipboardEntry {
    fn from_entry(entry: &ClipboardEntry, include_images: bool) -> Self {
        match entry {
            ClipboardEntry::Text {
                value,
                source_window,
            } => Self::Text {
                value: value.clone(),
                source_window: source_window.clone(),
            },
            ClipboardEntry::Image {
                width,
                height,
                rowstride,
                has_alpha,
                bits_per_sample,
                channels,
                pixels,
                source_window,
            } => Self::Image {
                width: *width,
                height: *height,
                rowstride: *rowstride,
                has_alpha: *has_alpha,
                bits_per_sample: *bits_per_sample,
                channels: *channels,
                pixel_bytes: pixels.len(),
                pixels: include_images.then_some(pixels.clone()),
                source_window: source_window.clone(),
            },
        }
    }
}

fn format_history_entry(id: usize, entry: &ClipboardEntry) -> String {
    match entry {
        ClipboardEntry::Text {
            value,
            source_window,
        } => format_entry_with_source(id, value.clone(), source_window.as_ref()),
        ClipboardEntry::Image {
            width,
            height,
            source_window,
            ..
        } => format_entry_with_source(
            id,
            format!("[image] {}x{}", width, height),
            source_window.as_ref(),
        ),
    }
}

fn format_entry_with_source(
    id: usize,
    value: String,
    source_window: Option<&ActiveWindowContext>,
) -> String {
    match source_window {
        Some(context) => {
            if let Some(app_id) = &context.app_id {
                format!(
                    "{}: {} [source: {} ({}) via {}]",
                    id, value, context.title, app_id, context.backend
                )
            } else {
                format!(
                    "{}: {} [source: {} via {}]",
                    id, value, context.title, context.backend
                )
            }
        }
        None => format!("{}: {}", id, value),
    }
}

fn validate_config(config_path_override: Option<std::path::PathBuf>) -> Result<()> {
    let loaded = AppConfig::load(config_path_override)?;
    loaded.config.validate()?;
    println!("Config is valid: {}", loaded.path.display());
    Ok(())
}
