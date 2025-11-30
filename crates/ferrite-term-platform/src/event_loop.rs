use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use ferrite_runtime::event_loop_proxy::{EventLoopControlFlow, EventLoopProxy};

pub enum TuiEvent<UserEvent> {
    StartOfEvents,
    Render,
    UserEvent(UserEvent),
    Crossterm(crossterm::event::Event),
}

pub struct TuiEventLoop<UserEvent> {
    proxy_tx: Sender<UserEvent>,
    proxy_rx: Receiver<UserEvent>,
    waker_tx: Sender<&'static str>,
    waker_rx: Receiver<&'static str>,
}

impl<UserEvent> Default for TuiEventLoop<UserEvent> {
    fn default() -> Self {
        Self::new()
    }
}

impl<UserEvent> TuiEventLoop<UserEvent> {
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

    pub fn create_proxy(&self) -> TuiEventLoopProxy<UserEvent> {
        TuiEventLoopProxy {
            proxy_tx: self.proxy_tx.clone(),
            waker_tx: self.waker_tx.clone(),
        }
    }

    pub fn run<F>(self, mut handler: F)
    where
        F: FnMut(&TuiEventLoopProxy<UserEvent>, TuiEvent<UserEvent>, &mut EventLoopControlFlow),
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
                    // Skip mouse moved to save CPU/battery
                    if let crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
                        kind, ..
                    }) = event
                        && kind == crossterm::event::MouseEventKind::Moved
                    {
                        continue;
                    }

                    let _ = crossterm_tx.send(event);
                    let _ = waker_tx.send("recv crossterm event");
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
                handler(&proxy, TuiEvent::UserEvent(event), &mut control_flow);
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

pub struct TuiEventLoopProxy<UserEvent> {
    proxy_tx: mpsc::Sender<UserEvent>,
    waker_tx: mpsc::Sender<&'static str>,
}

impl<UserEvent> Clone for TuiEventLoopProxy<UserEvent> {
    fn clone(&self) -> Self {
        Self {
            proxy_tx: self.proxy_tx.clone(),
            waker_tx: self.waker_tx.clone(),
        }
    }
}

impl<UserEvent: Send + 'static> EventLoopProxy<UserEvent> for TuiEventLoopProxy<UserEvent> {
    fn send(&self, event: UserEvent) {
        let _ = self.proxy_tx.send(event);
        let _ = self.waker_tx.send("recv user event");
    }

    fn request_render(&self, reason: &'static str) {
        let _ = self.waker_tx.send(reason);
    }

    fn dup(&self) -> Box<dyn EventLoopProxy<UserEvent>> {
        Box::new(self.clone())
    }
}
