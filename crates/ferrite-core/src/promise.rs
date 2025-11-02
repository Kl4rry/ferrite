use std::{
    mem,
    thread::{self, JoinHandle},
};

use crate::event_loop_proxy::{EventLoopProxy, UserEvent};

enum Kind<T> {
    Thread(JoinHandle<T>),
    Ready(T),
    Consumed,
}

pub struct Promise<T> {
    inner: Kind<T>,
}

impl<T: 'static + Send> Promise<T> {
    pub fn spawn<F: 'static + FnOnce() -> T + Send>(
        proxy: Box<dyn EventLoopProxy<UserEvent>>,
        f: F,
    ) -> Self {
        let thread = thread::spawn(move || {
            let value = (f)();
            proxy.request_render("promise ready");
            value
        });
        Self {
            inner: Kind::Thread(thread),
        }
    }

    pub fn poll(&mut self) -> Option<T> {
        let mut inner = Kind::Consumed;
        mem::swap(&mut self.inner, &mut inner);
        match inner {
            Kind::Thread(thread) => Some(thread.join().unwrap()),
            Kind::Ready(value) => Some(value),
            Kind::Consumed => None,
        }
    }
}

impl<T> Promise<T> {
    pub fn ready(value: T) -> Self {
        Self {
            inner: Kind::Ready(value),
        }
    }
}
