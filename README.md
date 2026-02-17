# slyboard

`slyboard` is a small Linux clipboard daemon with a tray menu and persistent history.

It watches clipboard changes, stores recent entries on disk, and lets you restore old entries from the tray icon menu.

## Features

- Clipboard history for text and images
- Tray icon menu with clickable history items
- `rofi` picker binary for quick keyboard-driven restore
- Persistent history cache across restarts
- Optional active-window metadata capture
- Single-instance lock to avoid duplicate daemons

## Requirements

- Linux desktop session (GTK/AppIndicator)
- `rofi` (optional, required for `slyboard-rofi`)
- Nix (recommended), or a Rust toolchain plus GTK/AppIndicator dev libraries

## Quick Start (Nix)

```bash
nix develop
cargo run -- run
```

If no config is found, create one in the project root as `slyboard.yaml` (or use the config paths described below).

## Configuration

Config is YAML.

Discovery order:

1. `./slyboard.yaml` (current working directory)
2. `~/.config/slyboard/config.yaml`

You can always override with `--config /path/to/config.yaml`.

Example:

```yaml
clipboard:
  backend: gtk
  active_window:
    backend:
      kind: auto
    blacklist:
      - keepassxc
      - slack
```

### Active Window Backend

- `kind: auto` - tries `hyprctl` first, then `xdotool`
- `kind: disabled` - do not capture active-window metadata
- `kind: command` - run a custom command and parse stdout as a title

When available, `source_window` metadata includes more than `title` and `app_id`.
`hyprctl` provides the richest details (`initial_app_id`, `initial_title`,
`window_id`, `pid`, `workspace_id`, `workspace_name`, and `is_xwayland`), while
`xdotool` includes `window_id`, `pid`, and `workspace_id`.

### Active Window Blacklist

Use `clipboard.active_window.blacklist` to skip clipboard capture when the focused
window matches a blocked value.

- For backends that provide an app id/class (for example `hyprctl`), values match
  app ids exactly (case-insensitive).
- If app id is unavailable, values are matched as case-insensitive substrings
  against the focused window title.

Command example:

```yaml
clipboard:
  active_window:
    backend:
      kind: command
      program: xdotool
      args:
        - getactivewindow
        - getwindowname
```

## CLI

```bash
# Run daemon (default command)
slyboard run

# Print history
slyboard history

# Print history as JSON
slyboard history --json

# Include image pixel bytes in history output
slyboard history --json --images

# Pause clipboard capture (daemon keeps running)
slyboard pause-capture

# Resume clipboard capture
slyboard resume-capture

# Print capture status (running|paused)
slyboard capture-status

# Validate config and exit
slyboard validate-config

# Use explicit config path
slyboard --config /path/to/config.yaml run

# Open clipboard history in rofi and restore selection
slyboard-rofi

# Customize rofi invocation details
slyboard-rofi --prompt "clipboard" --lines 25 --rofi-bin rofi
```

## Data and Runtime Paths

- History cache: `~/.cache/slyboard/history.json` (or platform cache dir equivalent)
- Instance lock socket: `${XDG_RUNTIME_DIR}/slyboard-<user>.sock` (falls back to temp dir)

## Home Manager Module

This flake exports a Home Manager module at `homeManagerModules.default`.

Example:

```nix
{
  inputs.slyboard.url = "path:/path/to/slyboard";

  outputs = { self, nixpkgs, home-manager, slyboard, ... }: {
    homeConfigurations.me = home-manager.lib.homeManagerConfiguration {
      # ...
      modules = [
        slyboard.homeManagerModules.default
        {
          programs.slyboard.enable = true;
        }
      ];
    };
  };
}
```

## Development

```bash
nix develop
cargo test
nix build
```
