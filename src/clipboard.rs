use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use once_cell::sync::OnceCell;

static CLIPBOARD: OnceCell<Mutex<String>> = OnceCell::new();
static LOCAL_CLIPBOARD: AtomicBool = AtomicBool::new(false);

pub fn init(local_clipboard: bool) {
    CLIPBOARD.set(Mutex::new(String::new())).unwrap();
    LOCAL_CLIPBOARD.store(local_clipboard, Ordering::SeqCst);
}

pub fn set_contents(text: impl Into<String>) {
    let text: String = text.into();
    if LOCAL_CLIPBOARD.load(Ordering::SeqCst) {
        *CLIPBOARD.get().unwrap().lock().unwrap() = text;
        return;
    }

    if cli_clipboard::set_contents(text.clone()).is_err() {
        *CLIPBOARD.get().unwrap().lock().unwrap() = text;
    }
}

pub fn get_contents() -> String {
    if LOCAL_CLIPBOARD.load(Ordering::SeqCst) {
        return CLIPBOARD.get().unwrap().lock().unwrap().clone();
    }

    match cli_clipboard::get_contents() {
        Ok(text) => text,
        Err(_) => CLIPBOARD.get().unwrap().lock().unwrap().clone(),
    }
}
