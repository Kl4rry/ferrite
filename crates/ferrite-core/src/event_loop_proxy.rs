use std::sync::OnceLock;

use crate::palette::{PaletteMode, PalettePromptEvent};

static PROXY: OnceLock<Box<dyn EventLoopProxy<UserEvent>>> = OnceLock::new();

pub fn set_proxy(proxy: Box<dyn EventLoopProxy<UserEvent>>) {
    if PROXY.set(proxy).is_err() {
        tracing::error!("Error attempted to set buffer proxy twice");
    }
}

pub fn get_proxy() -> Box<dyn EventLoopProxy<UserEvent>> {
    PROXY.get().unwrap().dup()
}

#[derive(Debug)]
pub enum UserEvent {
    PaletteFinished { mode: PaletteMode, content: String },
    PalettePreview { mode: PaletteMode, content: String },
    PromptEvent(PalettePromptEvent),
    Wake,
}

// TODO: Remove
pub use ferrite_runtime::event_loop_proxy::{EventLoopControlFlow, EventLoopProxy};

pub struct NoopEventLoop;

impl<E: 'static> EventLoopProxy<E> for NoopEventLoop {
    fn send(&self, _: E) {}
    fn request_render(&self, _: &'static str) {}
    fn dup(&self) -> Box<dyn EventLoopProxy<E>> {
        Box::new(NoopEventLoop)
    }
}
