use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppConfig {}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub config: AppConfig,
}

impl AppConfig {
    pub fn load(config_path_override: Option<PathBuf>) -> Result<LoadedConfig> {
        let path = if let Some(path) = config_path_override {
            path
        } else {
            resolve_default_config_path()?
        };

        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        let config: AppConfig = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse YAML config: {}", path.display()))?;

        Ok(LoadedConfig { path, config })
    }

    pub fn validate(&self) -> Result<()> {
        Ok(())
    }
}

fn resolve_default_config_path() -> Result<PathBuf> {
    let cwd_file = std::env::current_dir()?.join("slyboard.yaml");
    if cwd_file.exists() {
        return Ok(cwd_file);
    }

    let home_config = dirs::config_dir()
        .context("unable to resolve config directory from environment")?
        .join("slyboard")
        .join("config.yaml");
    if home_config.exists() {
        return Ok(home_config);
    }

    bail!(
        "no config file found; expected one of:\n- {}\n- {}",
        cwd_file.display(),
        home_config.display()
    );
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn validate_accepts_empty_config() {
        let cfg = AppConfig::default();
        cfg.validate().expect("empty config should be valid");
    }
}
