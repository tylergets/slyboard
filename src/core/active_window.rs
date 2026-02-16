use std::process::Command;

use serde_json::Value;

use crate::config::ActiveWindowBackend;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActiveWindowContext {
    pub backend: String,
    pub title: String,
    pub app_id: Option<String>,
    pub initial_app_id: Option<String>,
    pub initial_title: Option<String>,
    pub window_id: Option<String>,
    pub pid: Option<i64>,
    pub workspace_id: Option<i64>,
    pub workspace_name: Option<String>,
    pub is_xwayland: Option<bool>,
}

pub trait ActiveWindowProvider: Send + Sync {
    fn capture(&self) -> Option<ActiveWindowContext>;
}

pub struct DisabledActiveWindowProvider;

impl ActiveWindowProvider for DisabledActiveWindowProvider {
    fn capture(&self) -> Option<ActiveWindowContext> {
        None
    }
}

pub struct CommandActiveWindowProvider {
    program: String,
    args: Vec<String>,
    parser: fn(&str) -> Option<ActiveWindowContext>,
}

impl CommandActiveWindowProvider {
    pub fn new(
        program: impl Into<String>,
        args: Vec<String>,
        parser: fn(&str) -> Option<ActiveWindowContext>,
    ) -> Self {
        Self {
            program: program.into(),
            args,
            parser,
        }
    }
}

impl ActiveWindowProvider for CommandActiveWindowProvider {
    fn capture(&self) -> Option<ActiveWindowContext> {
        let output = Command::new(&self.program).args(&self.args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let raw = String::from_utf8(output.stdout).ok()?;
        (self.parser)(raw.trim())
    }
}

pub struct AutoActiveWindowProvider {
    providers: Vec<Box<dyn ActiveWindowProvider>>,
}

impl AutoActiveWindowProvider {
    pub fn new() -> Self {
        Self {
            providers: vec![
                Box::new(CommandActiveWindowProvider::new(
                    "hyprctl",
                    vec!["activewindow".into(), "-j".into()],
                    parse_hyprctl_active_window,
                )),
                Box::new(CommandActiveWindowProvider::new(
                    "xdotool",
                    vec!["getactivewindow".into(), "getwindowname".into()],
                    parse_xdotool_active_window,
                )),
            ],
        }
    }
}

impl ActiveWindowProvider for AutoActiveWindowProvider {
    fn capture(&self) -> Option<ActiveWindowContext> {
        for provider in &self.providers {
            if let Some(context) = provider.capture() {
                return Some(context);
            }
        }
        None
    }
}

pub fn provider_from_config(config: &ActiveWindowBackend) -> Box<dyn ActiveWindowProvider> {
    match config {
        ActiveWindowBackend::Disabled => Box::new(DisabledActiveWindowProvider),
        ActiveWindowBackend::Command { program, args } => {
            Box::new(CommandActiveWindowProvider::new(
                program.clone(),
                args.clone(),
                parse_command_active_window,
            ))
        }
        ActiveWindowBackend::Auto => Box::new(AutoActiveWindowProvider::new()),
    }
}

fn parse_hyprctl_active_window(raw: &str) -> Option<ActiveWindowContext> {
    let parsed: Value = serde_json::from_str(raw).ok()?;
    let title = parsed.get("title")?.as_str()?.trim();
    if title.is_empty() {
        return None;
    }

    let app_id = optional_trimmed_string(parsed.get("class"));
    let initial_app_id = optional_trimmed_string(parsed.get("initialClass"));
    let initial_title = optional_trimmed_string(parsed.get("initialTitle"));
    let window_id = optional_trimmed_string(parsed.get("address"));
    let pid = parsed.get("pid").and_then(|value| value.as_i64());

    let workspace_id = parsed
        .get("workspace")
        .and_then(|workspace| workspace.get("id"))
        .and_then(|value| value.as_i64());
    let workspace_name = parsed
        .get("workspace")
        .and_then(|workspace| workspace.get("name"))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let is_xwayland = parsed.get("xwayland").and_then(|value| value.as_bool());

    Some(ActiveWindowContext {
        backend: "hyprctl".to_string(),
        title: title.to_string(),
        app_id,
        initial_app_id,
        initial_title,
        window_id,
        pid,
        workspace_id,
        workspace_name,
        is_xwayland,
    })
}

fn parse_xdotool_active_window(raw: &str) -> Option<ActiveWindowContext> {
    parse_title_with_backend("xdotool", raw)
}

fn parse_command_active_window(raw: &str) -> Option<ActiveWindowContext> {
    parse_title_with_backend("command", raw)
}

fn parse_title_with_backend(backend: &str, raw: &str) -> Option<ActiveWindowContext> {
    let title = raw.trim();
    if title.is_empty() {
        return None;
    }
    Some(ActiveWindowContext {
        backend: backend.to_string(),
        title: title.to_string(),
        app_id: None,
        initial_app_id: None,
        initial_title: None,
        window_id: None,
        pid: None,
        workspace_id: None,
        workspace_name: None,
        is_xwayland: None,
    })
}

fn optional_trimmed_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
