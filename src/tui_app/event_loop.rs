use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiEventLoopControlFlow {
    Poll,
    Wait,
    Exit,
    WaitMax(Duration),
}

pub enum TuiAppEvent {
    PaletteEvent { mode: String, content: String },
}

pub enum TuiEvent {
    Render,
    AppEvent(TuiAppEvent),
    Crossterm(crossterm::event::Event),
}

pub struct TuiEventLoop {
    proxy_tx: Sender<TuiAppEvent>,
    proxy_rx: Receiver<TuiAppEvent>,
    waker_tx: Sender<()>,
    waker_rx: Receiver<()>,
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
        F: FnMut(&TuiEventLoopProxy, TuiEvent, &mut TuiEventLoopControlFlow),
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

        thread::spawn(move || loop {
            if let Ok(event) = crossterm::event::read() {
                let _ = crossterm_tx.send(event);
                let _ = waker_tx.send(());
            }
        });

        'main: loop {
            let mut control_flow = TuiEventLoopControlFlow::Wait;

            while let Ok(event) = crossterm_rx.try_recv() {
                handler(&proxy, TuiEvent::Crossterm(event), &mut control_flow);
                if control_flow == TuiEventLoopControlFlow::Exit {
                    break 'main;
                }
            }
            while let Ok(event) = proxy_rx.try_recv() {
                handler(&proxy, TuiEvent::AppEvent(event), &mut control_flow);
                if control_flow == TuiEventLoopControlFlow::Exit {
                    break 'main;
                }
            }
            handler(&proxy, TuiEvent::Render, &mut control_flow);

            match control_flow {
                TuiEventLoopControlFlow::Poll => {
                    let _ = waker_rx.try_recv();
                }
                TuiEventLoopControlFlow::Wait => {
                    let _ = waker_rx.recv();
                }
                TuiEventLoopControlFlow::Exit => break,
                TuiEventLoopControlFlow::WaitMax(timeout) => {
                    let _ = waker_rx.recv_timeout(timeout);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct TuiEventLoopProxy {
    proxy_tx: mpsc::Sender<TuiAppEvent>,
    waker_tx: mpsc::Sender<()>,
}

impl TuiEventLoopProxy {
    pub fn send(&self, event: TuiAppEvent) {
        let _ = self.proxy_tx.send(event);
        let _ = self.waker_tx.send(());
    }

    pub fn request_render(&self) {
        let _ = self.waker_tx.send(());
    }
}
