use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use gtk::prelude::*;
use libappindicator::{AppIndicator as LibAppIndicator, AppIndicatorStatus};

use crate::clipboard::backend::GtkClipboardBackend;
use crate::clipboard::poller::{start_gtk_polling, ClipboardPoller};
use crate::clipboard::{ClipboardEntry, SharedClipboardState};
use crate::config::{ClipboardBackend, ClipboardConfig};
use crate::core::active_window::provider_from_config;
use crate::core::capture_control::{is_capture_paused, set_capture_paused};

pub struct TrayIndicator {
    _gtk_thread: JoinHandle<()>,
}

const BUNDLED_TRAY_ICON_NAME: &str = "slyboard";
const BUNDLED_TRAY_ICON_SVG: &[u8] = include_bytes!("slyboard.svg");
const CLIPBOARD_POLL_INTERVAL_MS: u64 = 750;
const MENU_LABEL_CHAR_LIMIT: usize = 70;
const CLIPBOARD_NOTIFICATION_TITLE: &str = "slyboard";
const CLIPBOARD_TEXT_NOTIFICATION_BODY: &str = "text copied to clipboard";
const CLIPBOARD_IMAGE_NOTIFICATION_BODY: &str = "image copied to clipboard";
const RUNNING_LABEL: &str = "Running";
const PAUSED_LABEL: &str = "Paused";
const PAUSE_CAPTURE_LABEL: &str = "Pause Capture";
const RESUME_CAPTURE_LABEL: &str = "Resume Capture";

pub fn start(
    shared_state: SharedClipboardState,
    clipboard_config: ClipboardConfig,
) -> Option<TrayIndicator> {
    if env::var_os("DISPLAY").is_none() {
        eprintln!("warning: DISPLAY is not set; cannot create tray icon");
        return None;
    }
    if env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        eprintln!("warning: DBus session is not set; appindicator may not be visible");
    }

    let (ready_tx, ready_rx) = mpsc::channel();
    let gtk_thread = std::thread::spawn(move || {
        if let Err(err) = run_indicator(ready_tx, shared_state, clipboard_config) {
            eprintln!("tray thread exited: {err}");
        }
    });

    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => Some(TrayIndicator {
            _gtk_thread: gtk_thread,
        }),
        Ok(Err(err)) => {
            eprintln!("failed to start tray icon: {err}");
            let _ = gtk_thread.join();
            None
        }
        Err(_) => {
            eprintln!("warning: tray startup timed out; keeping tray thread running");
            Some(TrayIndicator {
                _gtk_thread: gtk_thread,
            })
        }
    }
}

fn run_indicator(
    ready_tx: Sender<Result<(), String>>,
    shared_state: SharedClipboardState,
    clipboard_config: ClipboardConfig,
) -> Result<(), String> {
    if let Err(err) = gtk::init() {
        let msg = err.to_string();
        let _ = ready_tx.send(Err(msg.clone()));
        return Err(msg);
    }

    let tray_icon_name = install_bundled_icon().unwrap_or("input-keyboard");
    let mut indicator = LibAppIndicator::new("slyboard", tray_icon_name);
    indicator.set_title("slyboard");
    indicator.set_status(AppIndicatorStatus::Active);

    let clipboard = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
    let poller = match clipboard_config.backend {
        ClipboardBackend::Gtk => Rc::new(RefCell::new(ClipboardPoller::new(
            GtkClipboardBackend::new(
                &clipboard,
                provider_from_config(&clipboard_config.active_window.backend),
            ),
            clipboard_config.active_window.blacklist.clone(),
        ))),
    };
    if let Some(entry) = poller.borrow_mut().poll_once() {
        if let Err(err) = shared_state.record_entry(entry) {
            eprintln!("failed to seed clipboard history: {err}");
        }
    }

    let mut menu = gtk::Menu::new();
    let running_item = gtk::MenuItem::with_label(RUNNING_LABEL);
    running_item.set_sensitive(false);
    menu.append(&running_item);
    running_item.show();

    let capture_paused = Rc::new(RefCell::new(match is_capture_paused() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("warning: failed to read capture pause state: {err}");
            false
        }
    }));

    let pause_item = gtk::MenuItem::with_label(PAUSE_CAPTURE_LABEL);
    update_capture_menu_state(&running_item, &pause_item, *capture_paused.borrow());
    let capture_paused_for_toggle = capture_paused.clone();
    let running_item_for_toggle = running_item.clone();
    let pause_item_for_toggle = pause_item.clone();
    pause_item.connect_activate(move |_| {
        let next_state = !*capture_paused_for_toggle.borrow();
        if let Err(err) = set_capture_paused(next_state) {
            eprintln!("failed to update capture pause state: {err}");
            return;
        }

        *capture_paused_for_toggle.borrow_mut() = next_state;
        update_capture_menu_state(&running_item_for_toggle, &pause_item_for_toggle, next_state);
    });
    menu.append(&pause_item);
    pause_item.show();

    let separator = gtk::SeparatorMenuItem::new();
    menu.append(&separator);
    separator.show();

    let history_root_item = gtk::MenuItem::with_label("History");
    let history_menu = gtk::Menu::new();
    history_root_item.set_submenu(Some(&history_menu));
    menu.append(&history_root_item);
    history_root_item.show();
    refresh_history_menu(&history_menu, &clipboard, &shared_state.history_snapshot());

    let clear_history_item = gtk::MenuItem::with_label("Clear History");
    let shared_state_for_clear = shared_state.clone();
    let history_menu_for_clear = history_menu.clone();
    let clipboard_for_clear = clipboard.clone();
    clear_history_item.connect_activate(move |_| {
        if let Err(err) = shared_state_for_clear.clear_history() {
            eprintln!("failed to clear clipboard history: {err}");
            return;
        }
        refresh_history_menu(
            &history_menu_for_clear,
            &clipboard_for_clear,
            &shared_state_for_clear.history_snapshot(),
        );
    });
    menu.append(&clear_history_item);
    clear_history_item.show();

    let separator = gtk::SeparatorMenuItem::new();
    menu.append(&separator);
    separator.show();

    let quit_item = gtk::MenuItem::with_label("Quit");
    quit_item.connect_activate(|_| process::exit(0));
    menu.append(&quit_item);
    quit_item.show();

    menu.show_all();
    indicator.set_menu(&mut menu);

    let shared_state_for_poll = shared_state.clone();
    let history_menu_for_poll = history_menu.clone();
    let clipboard_for_menu = clipboard.clone();
    let capture_paused_for_poll = capture_paused.clone();
    let running_item_for_poll = running_item.clone();
    let pause_item_for_poll = pause_item.clone();
    start_gtk_polling(
        poller,
        Duration::from_millis(CLIPBOARD_POLL_INTERVAL_MS),
        move |entry| {
            let paused = match is_capture_paused() {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("warning: failed to read capture pause state: {err}");
                    *capture_paused_for_poll.borrow()
                }
            };
            {
                let mut pause_state = capture_paused_for_poll.borrow_mut();
                if *pause_state != paused {
                    *pause_state = paused;
                    update_capture_menu_state(&running_item_for_poll, &pause_item_for_poll, paused);
                }
            }
            if paused {
                return;
            }

            let notification_body = notification_body_for_entry(&entry);
            let changed = match shared_state_for_poll.record_entry(entry) {
                Ok(changed) => changed,
                Err(err) => {
                    eprintln!("failed to record clipboard history: {err}");
                    false
                }
            };

            if changed {
                println!("clipboard event: {notification_body}");
                send_clipboard_notification(notification_body);
                let history = shared_state_for_poll.history_snapshot();
                refresh_history_menu(&history_menu_for_poll, &clipboard_for_menu, &history);
            }
        },
    );

    let _ = ready_tx.send(Ok(()));
    gtk::main();
    Ok(())
}

fn update_capture_menu_state(
    running_item: &gtk::MenuItem,
    pause_item: &gtk::MenuItem,
    paused: bool,
) {
    if paused {
        running_item.set_label(PAUSED_LABEL);
        pause_item.set_label(RESUME_CAPTURE_LABEL);
    } else {
        running_item.set_label(RUNNING_LABEL);
        pause_item.set_label(PAUSE_CAPTURE_LABEL);
    }
}

fn send_clipboard_notification(body: &str) {
    if let Err(err) = Command::new("notify-send")
        .arg(CLIPBOARD_NOTIFICATION_TITLE)
        .arg(body)
        .status()
    {
        eprintln!("warning: failed to send clipboard notification: {err}");
    }
}

fn refresh_history_menu(
    history_menu: &gtk::Menu,
    clipboard: &gtk::Clipboard,
    history: &[ClipboardEntry],
) {
    for child in history_menu.children() {
        history_menu.remove(&child);
    }

    if history.is_empty() {
        let empty_item = gtk::MenuItem::with_label("No clipboard history yet");
        empty_item.set_sensitive(false);
        history_menu.append(&empty_item);
        empty_item.show();
        return;
    }

    for entry in history.iter().cloned() {
        let label = format_menu_label(&entry);
        let item = gtk::MenuItem::with_label(&label);
        let clipboard = clipboard.clone();
        item.connect_activate(move |_| {
            set_clipboard_value(&clipboard, &entry);
        });
        history_menu.append(&item);
        item.show();
    }
}

fn format_menu_label(entry: &ClipboardEntry) -> String {
    match entry {
        ClipboardEntry::Text { value, .. } => format_text_menu_label(value),
        ClipboardEntry::Image { width, height, .. } => {
            format!("[image] {}x{}", width, height)
        }
    }
}

fn format_text_menu_label(value: &str) -> String {
    let sanitized = value.replace('\n', "\\n").replace('\r', "\\r");
    let char_count = sanitized.chars().count();
    if char_count <= MENU_LABEL_CHAR_LIMIT {
        return sanitized;
    }

    let truncated: String = sanitized.chars().take(MENU_LABEL_CHAR_LIMIT).collect();
    format!("{truncated}...")
}

fn set_clipboard_value(clipboard: &gtk::Clipboard, entry: &ClipboardEntry) {
    match entry {
        ClipboardEntry::Text { value, .. } => {
            clipboard.set_text(value);
            clipboard.store();
        }
        ClipboardEntry::Image {
            width,
            height,
            rowstride,
            has_alpha,
            bits_per_sample,
            pixels,
            ..
        } => {
            let bytes = gtk::glib::Bytes::from(pixels.as_slice());
            let image = gtk::gdk_pixbuf::Pixbuf::from_bytes(
                &bytes,
                gtk::gdk_pixbuf::Colorspace::Rgb,
                *has_alpha,
                *bits_per_sample,
                *width,
                *height,
                *rowstride,
            );
            clipboard.set_image(&image);
            clipboard.store();
        }
    }
}

fn notification_body_for_entry(entry: &ClipboardEntry) -> &'static str {
    match entry {
        ClipboardEntry::Text { .. } => CLIPBOARD_TEXT_NOTIFICATION_BODY,
        ClipboardEntry::Image { .. } => CLIPBOARD_IMAGE_NOTIFICATION_BODY,
    }
}

fn install_bundled_icon() -> Option<&'static str> {
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".local/share")))?;

    let icon_path = data_home
        .join("icons")
        .join("hicolor")
        .join("scalable")
        .join("apps")
        .join(format!("{BUNDLED_TRAY_ICON_NAME}.svg"));

    if let Some(parent) = icon_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("warning: failed to create icon directory: {err}");
            return None;
        }
    }

    if let Err(err) = fs::write(&icon_path, BUNDLED_TRAY_ICON_SVG) {
        eprintln!("warning: failed to write bundled tray icon: {err}");
        return None;
    }

    Some(BUNDLED_TRAY_ICON_NAME)
}
