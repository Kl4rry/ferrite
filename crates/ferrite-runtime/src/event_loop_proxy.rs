use std::time::Duration;

pub trait EventLoopProxy<E: 'static>: Send + Sync {
    fn send(&self, event: E);
    fn request_render(&self, reason: &'static str);
    fn dup(&self) -> Box<dyn EventLoopProxy<E>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLoopControlFlow {
    Poll,
    Wait,
    Exit,
    WaitMax(Duration),
}
