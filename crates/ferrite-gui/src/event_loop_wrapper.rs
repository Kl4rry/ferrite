use ferrite_core::event_loop_proxy::{EventLoopProxy, UserEvent};

#[derive(Debug, Clone)]
pub struct EventLoopProxyWrapper(winit::event_loop::EventLoopProxy<UserEvent>);

impl EventLoopProxyWrapper {
    pub fn new(proxy: winit::event_loop::EventLoopProxy<UserEvent>) -> Self {
        Self(proxy)
    }
}

impl EventLoopProxy for EventLoopProxyWrapper {
    fn send(&self, event: UserEvent) {
        let _ = self.0.send_event(event);
    }

    fn request_render(&self) {
        let _ = self.0.send_event(UserEvent::Wake);
    }

    fn dup(&self) -> Box<dyn EventLoopProxy> {
        Box::new(self.clone())
    }
}
