use std::{sync::OnceLock, time::Duration};

use crate::palette::{PaletteMode, PalettePromptEvent};

static PROXY: OnceLock<Box<dyn EventLoopProxy>> = OnceLock::new();

pub fn set_proxy(proxy: Box<dyn EventLoopProxy>) {
    if PROXY.set(proxy).is_err() {
        tracing::error!("Error attempted to set buffer proxy twice");
    }
}

pub fn get_proxy() -> Box<dyn EventLoopProxy> {
    PROXY.get().unwrap().dup()
}

#[derive(Debug)]
pub enum UserEvent {
    PaletteFinished { mode: PaletteMode, content: String },
    PalettePreview { mode: PaletteMode, content: String },
    PromptEvent(PalettePromptEvent),
    Wake,
}

pub trait EventLoopProxy: Send + Sync {
    fn send(&self, event: UserEvent);
    fn request_render(&self);
    fn dup(&self) -> Box<dyn EventLoopProxy>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLoopControlFlow {
    Poll,
    Wait,
    Exit,
    WaitMax(Duration),
}

pub struct NoopEventLoop;

impl EventLoopProxy for NoopEventLoop {
    fn send(&self, _: UserEvent) {}
    fn request_render(&self) {}
    fn dup(&self) -> Box<dyn EventLoopProxy> {
        Box::new(NoopEventLoop)
    }
}
