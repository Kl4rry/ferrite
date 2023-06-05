use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use arboard::Clipboard;

static CLIPBOARD: Mutex<Option<Clipboard>> = Mutex::new(None);
static LOCAL_CLIPBOARD: Mutex<String> = Mutex::new(String::new());
static IS_USING_LOCAL_CLIPBOARD: AtomicBool = AtomicBool::new(false);

pub fn init(local_clipboard: bool) {
    IS_USING_LOCAL_CLIPBOARD.store(local_clipboard, Ordering::SeqCst);
    let Ok(clipboard) = Clipboard::new() else {
        IS_USING_LOCAL_CLIPBOARD.store(true, Ordering::SeqCst);
        return;

    };
    *CLIPBOARD.lock().unwrap() = Some(clipboard);
}

pub fn uninit() {
    *CLIPBOARD.lock().unwrap() = None;
}

pub fn set_contents(text: impl Into<String>) {
    let text: String = text.into();
    if IS_USING_LOCAL_CLIPBOARD.load(Ordering::SeqCst) {
        *LOCAL_CLIPBOARD.lock().unwrap() = text;
        return;
    }

    let mut clipboard = CLIPBOARD.lock().unwrap();
    if let Some(clipboard) = &mut *clipboard {
        if clipboard.set_text(&text).is_ok() {
            return;
        }
    }

    *LOCAL_CLIPBOARD.lock().unwrap() = text;
}

pub fn get_contents() -> String {
    if IS_USING_LOCAL_CLIPBOARD.load(Ordering::SeqCst) {
        return LOCAL_CLIPBOARD.lock().unwrap().clone();
    }

    let mut clipboard = CLIPBOARD.lock().unwrap();
    if let Some(clipboard) = &mut *clipboard {
        if let Ok(text) = clipboard.get_text() {
            return text;
        }
    }

    LOCAL_CLIPBOARD.lock().unwrap().clone()
}

#[cfg(target_os = "linux")]
pub fn set_primary(text: impl Into<String>) {
    use arboard::{LinuxClipboardKind, SetExtLinux};
    if let Some(clipboard) = CLIPBOARD.lock().unwrap().as_mut() {
        let _ = clipboard
            .set()
            .clipboard(LinuxClipboardKind::Primary)
            .text(&text.into());
    }
}

pub fn get_primary() -> String {
    #[cfg(target_os = "linux")]
    {
        use arboard::{GetExtLinux, LinuxClipboardKind};
        if let Some(clipboard) = CLIPBOARD.lock().unwrap().as_mut() {
            return clipboard
                .get()
                .clipboard(LinuxClipboardKind::Primary)
                .text()
                .unwrap_or_default();
        }
    }
    String::new()
}

pub fn set_local_clipboard(local_clipboard: bool) {
    IS_USING_LOCAL_CLIPBOARD.store(local_clipboard, Ordering::SeqCst);
}
