use crate::clipboard::ClipboardEntry;
use crate::core::active_window::{ActiveWindowContext, ActiveWindowProvider};

pub trait ClipboardBackend {
    fn read_entry(&self) -> Option<ClipboardEntry>;
    fn read_active_window(&self) -> Option<ActiveWindowContext> {
        None
    }
}

#[cfg(target_os = "linux")]
pub struct GtkClipboardBackend {
    clipboard: gtk::Clipboard,
    active_window_provider: Box<dyn ActiveWindowProvider>,
}

#[cfg(target_os = "linux")]
impl GtkClipboardBackend {
    pub fn new(
        clipboard: &gtk::Clipboard,
        active_window_provider: Box<dyn ActiveWindowProvider>,
    ) -> Self {
        Self {
            clipboard: clipboard.clone(),
            active_window_provider,
        }
    }
}

#[cfg(target_os = "linux")]
impl ClipboardBackend for GtkClipboardBackend {
    fn read_entry(&self) -> Option<ClipboardEntry> {
        if let Some(text) = self.clipboard.wait_for_text() {
            let value = text.to_string();
            if !value.is_empty() {
                return Some(ClipboardEntry::Text {
                    value,
                    source_window: None,
                });
            }
        }

        let image = self.clipboard.wait_for_image()?;
        let pixel_bytes = image.pixel_bytes()?;
        let pixels = pixel_bytes.as_ref().to_vec();
        if pixels.is_empty() {
            return None;
        }

        Some(ClipboardEntry::Image {
            width: image.width(),
            height: image.height(),
            rowstride: image.rowstride(),
            has_alpha: image.has_alpha(),
            bits_per_sample: image.bits_per_sample(),
            channels: image.n_channels(),
            pixels,
            source_window: None,
        })
    }

    fn read_active_window(&self) -> Option<ActiveWindowContext> {
        self.active_window_provider.capture()
    }
}
