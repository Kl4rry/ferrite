use std::{mem, sync::mpsc};

use crate::event_loop_proxy::{EventLoopProxy, UserEvent};

enum Kind<T> {
    Waiting(mpsc::Receiver<T>),
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
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let value = (f)();
            let _ = tx.send(value);
            proxy.request_render("promise ready");
        });
        Self {
            inner: Kind::Waiting(rx),
        }
    }

    pub fn poll(&mut self) -> Option<T> {
        match &mut self.inner {
            Kind::Waiting(rx) => {
                let value = rx.try_recv().ok()?;
                self.inner = Kind::Consumed;
                Some(value)
            }
            Kind::Ready(_) => {
                let mut inner = Kind::Consumed;
                mem::swap(&mut inner, &mut self.inner);
                match inner {
                    Kind::Ready(value) => Some(value),
                    _ => unsafe { std::hint::unreachable_unchecked() },
                }
            }
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

    pub fn empty() -> Self {
        Self {
            inner: Kind::Consumed,
        }
    }
}
