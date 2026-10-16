pub trait EventLoopProxy<E: 'static>: Send + Sync {
    fn send(&self, event: E);
    fn request_render(&self, reason: &'static str);
    fn dup(&self) -> Box<dyn EventLoopProxy<E>>;
}
