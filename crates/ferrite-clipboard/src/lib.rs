use std::sync::Mutex;

use arboard::Clipboard;
use crossterm::{clipboard::CopyToClipboard, execute};

pub enum ClipboardKind {
    Terminal,
    System,
    Local,
}

enum ClipboardState {
    Terminal(Clipboard), // System clipboard is used to paste
    System(Clipboard),
    Local(LocalClipboard),
    Nop,
}

#[derive(Default)]
struct LocalClipboard {
    primary: String,
    system: String,
}

static CLIPBOARD: Mutex<ClipboardState> = Mutex::new(ClipboardState::Nop);

pub fn init(kind: ClipboardKind) {
    match kind {
        ClipboardKind::Terminal => {
            *CLIPBOARD.lock().unwrap() = ClipboardState::Terminal(Clipboard::new().unwrap());
        }
        ClipboardKind::System => {
            *CLIPBOARD.lock().unwrap() = ClipboardState::System(Clipboard::new().unwrap());
        }
        ClipboardKind::Local => {
            *CLIPBOARD.lock().unwrap() = ClipboardState::Local(LocalClipboard::default());
        }
    }
}

pub fn uninit() {
    *CLIPBOARD.lock().unwrap() = ClipboardState::Nop;
}

pub fn set_contents(text: impl Into<String>) {
    let text: String = text.into();
    let mut guard = CLIPBOARD.lock().unwrap();
    match &mut *guard {
        ClipboardState::Terminal(_) => {
            if let Err(err) = execute!(std::io::stdout(), CopyToClipboard::to_clipboard_from(text))
            {
                tracing::error!("{err}");
            }
        }
        ClipboardState::System(clipboard) => {
            if let Err(err) = clipboard.set_text(&text) {
                tracing::error!("{err}");
            }
        }
        ClipboardState::Local(local) => local.system = text,
        ClipboardState::Nop => (),
    }
}

pub fn get_contents() -> String {
    let mut guard = CLIPBOARD.lock().unwrap();
    match &mut *guard {
        ClipboardState::Terminal(clipboard) => clipboard.get_text().unwrap_or_default(),
        ClipboardState::System(clipboard) => clipboard.get_text().unwrap_or_default(),
        ClipboardState::Local(local) => local.system.clone(),
        ClipboardState::Nop => String::new(),
    }
}

#[cfg(target_os = "linux")]
pub fn set_primary(text: impl Into<String>) {
    use arboard::{LinuxClipboardKind, SetExtLinux};
    let text: String = text.into();
    let mut guard = CLIPBOARD.lock().unwrap();
    match &mut *guard {
        ClipboardState::Terminal(_) => {
            if let Err(err) = execute!(std::io::stdout(), CopyToClipboard::to_primary_from(text)) {
                tracing::error!("{err}");
            }
        }
        ClipboardState::System(clipboard) => {
            if let Err(err) = clipboard
                .set()
                .clipboard(LinuxClipboardKind::Primary)
                .text(text)
            {
                tracing::error!("{err}");
            }
        }
        ClipboardState::Local(local) => local.system = text,
        ClipboardState::Nop => (),
    }
}

pub fn get_primary() -> String {
    #[cfg(target_os = "linux")]
    {
        use arboard::{GetExtLinux, LinuxClipboardKind};
        let mut guard = CLIPBOARD.lock().unwrap();
        match &mut *guard {
            ClipboardState::Terminal(clipboard) => clipboard
                .get()
                .clipboard(LinuxClipboardKind::Primary)
                .text()
                .unwrap_or_default(),
            ClipboardState::System(clipboard) => clipboard
                .get()
                .clipboard(LinuxClipboardKind::Primary)
                .text()
                .unwrap_or_default(),
            ClipboardState::Local(local) => local.primary.clone(),
            ClipboardState::Nop => String::new(),
        }
    }
    #[cfg(not(target_os = "linux"))]
    String::new()
}
