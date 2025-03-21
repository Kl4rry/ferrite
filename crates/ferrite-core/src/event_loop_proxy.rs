use std::time::Duration;

use crate::palette::{PaletteMode, PalettePromptEvent};

#[derive(Debug)]
pub enum UserEvent {
    PaletteEvent { mode: PaletteMode, content: String },
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
