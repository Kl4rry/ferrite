use crate::{buffer::Buffer, palette::PalettePromptEvent};


pub enum UserEvent {
    PaletteEvent { mode: String, content: String },
    PromptEvent(PalettePromptEvent),
    ShellResult(Result<Buffer, anyhow::Error>),
}

pub trait EventLoopProxy: Send + Sync {
    fn send(&self, event: UserEvent);
    fn request_render(&self);
    fn dup(&self) -> Box<dyn EventLoopProxy>;
}