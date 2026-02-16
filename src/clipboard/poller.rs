use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::clipboard::backend::ClipboardBackend;
use crate::clipboard::ClipboardEntry;

pub struct ClipboardPoller<B: ClipboardBackend> {
    backend: B,
    last_seen_value: Option<ClipboardEntry>,
}

impl<B: ClipboardBackend> ClipboardPoller<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            last_seen_value: None,
        }
    }

    pub fn poll_once(&mut self) -> Option<ClipboardEntry> {
        let value = self.backend.read_entry()?;
        if value.is_empty() {
            return None;
        }

        if self.last_seen_value.as_ref() == Some(&value) {
            return None;
        }

        self.last_seen_value = Some(value.clone());
        Some(value)
    }
}

#[cfg(target_os = "linux")]
pub fn start_gtk_polling<B, F>(
    poller: Rc<RefCell<ClipboardPoller<B>>>,
    interval: Duration,
    mut on_change: F,
) where
    B: ClipboardBackend + 'static,
    F: FnMut(ClipboardEntry) + 'static,
{
    gtk::glib::timeout_add_local(interval, move || {
        if let Some(value) = poller.borrow_mut().poll_once() {
            on_change(value);
        }
        gtk::glib::ControlFlow::Continue
    });
}
