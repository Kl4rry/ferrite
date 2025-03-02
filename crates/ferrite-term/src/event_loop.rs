use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use ferrite_core::event_loop_proxy::{EventLoopControlFlow, EventLoopProxy, UserEvent};

pub enum TuiEvent {
    StartOfEvents,
    Render,
    AppEvent(UserEvent),
    Crossterm(crossterm::event::Event),
}

pub struct TuiEventLoop {
    proxy_tx: Sender<UserEvent>,
    proxy_rx: Receiver<UserEvent>,
    waker_tx: Sender<()>,
    waker_rx: Receiver<()>,
}

impl Default for TuiEventLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiEventLoop {
    pub fn new() -> Self {
        let (proxy_tx, proxy_rx) = mpsc::channel();
        let (waker_tx, waker_rx) = mpsc::channel();
        Self {
            proxy_tx,
            proxy_rx,
            waker_tx,
            waker_rx,
        }
    }

    pub fn create_proxy(&self) -> TuiEventLoopProxy {
        TuiEventLoopProxy {
            proxy_tx: self.proxy_tx.clone(),
            waker_tx: self.waker_tx.clone(),
        }
    }

    pub fn run<F>(self, mut handler: F)
    where
        F: FnMut(&TuiEventLoopProxy, TuiEvent, &mut EventLoopControlFlow),
    {
        let Self {
            proxy_tx,
            proxy_rx,
            waker_tx,
            waker_rx,
        } = self;
        let (crossterm_tx, crossterm_rx) = mpsc::channel();

        let proxy = TuiEventLoopProxy {
            proxy_tx,
            waker_tx: waker_tx.clone(),
        };

        thread::spawn(move || {
            loop {
                if let Ok(event) = crossterm::event::read() {
                    let _ = crossterm_tx.send(event);
                    let _ = waker_tx.send(());
                }
            }
        });

        'main: loop {
            let mut control_flow = EventLoopControlFlow::Wait;
            handler(&proxy, TuiEvent::StartOfEvents, &mut control_flow);

            while let Ok(event) = crossterm_rx.try_recv() {
                handler(&proxy, TuiEvent::Crossterm(event), &mut control_flow);
                if control_flow == EventLoopControlFlow::Exit {
                    break 'main;
                }
            }
            while let Ok(event) = proxy_rx.try_recv() {
                handler(&proxy, TuiEvent::AppEvent(event), &mut control_flow);
                if control_flow == EventLoopControlFlow::Exit {
                    break 'main;
                }
            }
            handler(&proxy, TuiEvent::Render, &mut control_flow);

            match control_flow {
                EventLoopControlFlow::Poll => {
                    let _ = waker_rx.try_recv();
                }
                EventLoopControlFlow::Wait => {
                    let _ = waker_rx.recv();
                }
                EventLoopControlFlow::Exit => break,
                EventLoopControlFlow::WaitMax(timeout) => {
                    let _ = waker_rx.recv_timeout(timeout);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct TuiEventLoopProxy {
    proxy_tx: mpsc::Sender<UserEvent>,
    waker_tx: mpsc::Sender<()>,
}

impl EventLoopProxy for TuiEventLoopProxy {
    fn send(&self, event: ferrite_core::event_loop_proxy::UserEvent) {
        let _ = self.proxy_tx.send(event);
        let _ = self.waker_tx.send(());
    }

    fn request_render(&self) {
        let _ = self.waker_tx.send(());
    }

    fn dup(&self) -> Box<dyn EventLoopProxy> {
        Box::new(self.clone())
    }
}
