use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use arboard::Clipboard;
use once_cell::sync::OnceCell;

static CLIPBOARD: OnceCell<Mutex<Option<Clipboard>>> = OnceCell::new();
static LOCAL_CLIPBOARD: OnceCell<Mutex<String>> = OnceCell::new();
static IS_USING_LOCAL_CLIPBOARD: AtomicBool = AtomicBool::new(false);

pub fn init(local_clipboard: bool) {
    LOCAL_CLIPBOARD.set(Mutex::new(String::new())).unwrap();
    IS_USING_LOCAL_CLIPBOARD.store(local_clipboard, Ordering::SeqCst);
    let Ok(clipboard) = Clipboard::new() else {
        IS_USING_LOCAL_CLIPBOARD.store(true, Ordering::SeqCst);
        return;

    };
    if CLIPBOARD.set(Mutex::new(Some(clipboard))).is_err() {
        IS_USING_LOCAL_CLIPBOARD.store(true, Ordering::SeqCst);
    }
}

pub fn uninit() {
    *CLIPBOARD.get().unwrap().lock().unwrap() = None;
}

pub fn set_contents(text: impl Into<String>) {
    let text: String = text.into();
    if IS_USING_LOCAL_CLIPBOARD.load(Ordering::SeqCst) {
        *LOCAL_CLIPBOARD.get().unwrap().lock().unwrap() = text;
        return;
    }

    {
        let mut clipboard = CLIPBOARD.get().unwrap().lock().unwrap();
        if clipboard.as_mut().unwrap().set_text(&text).is_err() {
            *LOCAL_CLIPBOARD.get().unwrap().lock().unwrap() = text;
        }
    }
}

pub fn get_contents() -> String {
    if IS_USING_LOCAL_CLIPBOARD.load(Ordering::SeqCst) {
        return LOCAL_CLIPBOARD.get().unwrap().lock().unwrap().clone();
    }

    let mut clipboard = CLIPBOARD.get().unwrap().lock().unwrap();

    match clipboard.as_mut().unwrap().get_text() {
        Ok(text) => text,
        Err(_) => LOCAL_CLIPBOARD.get().unwrap().lock().unwrap().clone(),
    }
}

#[cfg(target_os = "linux")]
pub fn set_primary(text: impl Into<String>) {
    use arboard::{SetExtLinux, LinuxClipboardKind, Set};
    if let Some(clipboard) = CLIPBOARD.get() {
        let mut clipboard = clipboard.lock().unwrap();
        let _ = clipboard.as_mut().unwrap().set().clipboard(LinuxClipboardKind::Primary).set_text(&text.into());
    }
}

pub fn get_primary() -> String {
    #[cfg(target_os = "linux")]
    {
        use arboard::{GetExtLinux, LinuxClipboardKind, Get};
        if let Some(clipboard) = CLIPBOARD.get() {
            let mut clipboard = clipboard.lock().unwrap();
            clipboard.as_mut().unwrap().get().clipboard(LinuxClipboardKind::Primary).get_text().unwrap_or_default()
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        String::new()
    }
}

pub fn set_local_clipboard(local_clipboard: bool) {
    IS_USING_LOCAL_CLIPBOARD.store(local_clipboard, Ordering::SeqCst);
}
