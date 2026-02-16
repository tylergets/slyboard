use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const PAUSE_FILE_SUFFIX: &str = "paused";

pub fn is_capture_paused() -> Result<bool> {
    Ok(default_pause_path().exists())
}

pub fn set_capture_paused(paused: bool) -> Result<()> {
    set_capture_paused_at_path(&default_pause_path(), paused)
}

fn set_capture_paused_at_path(path: &Path, paused: bool) -> Result<()> {
    if paused {
        fs::write(path, b"paused\n").with_context(|| {
            format!(
                "failed to write slyboard capture pause marker: {}",
                path.display()
            )
        })?;
        return Ok(());
    }

    if path.exists() {
        fs::remove_file(path).with_context(|| {
            format!(
                "failed to remove slyboard capture pause marker: {}",
                path.display()
            )
        })?;
    }
    Ok(())
}

fn default_pause_path() -> PathBuf {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    runtime_dir.join(format!("slyboard-{}-{PAUSE_FILE_SUFFIX}", user_hint()))
}

fn user_hint() -> String {
    std::env::var("USER")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "user".to_string())
}

#[cfg(test)]
mod tests {
    use super::{is_capture_paused, set_capture_paused_at_path};
    use std::path::PathBuf;

    fn test_pause_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "slyboard-test-paused-{}-{}",
            std::process::id(),
            name
        ))
    }

    #[test]
    fn writes_pause_marker_when_paused() {
        let path = test_pause_path("pause");
        set_capture_paused_at_path(&path, true).expect("pause marker write should succeed");
        assert!(path.exists(), "pause marker should exist");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn removes_pause_marker_when_resumed() {
        let path = test_pause_path("resume");
        std::fs::write(&path, b"paused\n").expect("seed pause marker");
        set_capture_paused_at_path(&path, false).expect("pause marker removal should succeed");
        assert!(!path.exists(), "pause marker should be removed");
    }

    #[test]
    fn reports_default_state_without_crashing() {
        let _ = is_capture_paused().expect("default paused lookup should not fail");
    }
}
