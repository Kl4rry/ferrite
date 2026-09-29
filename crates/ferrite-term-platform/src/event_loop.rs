use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use ferrite_runtime::{control_flow::EventLoopControlFlow, event_loop_proxy::EventLoopProxy};

pub enum TuiEvent<UserEvent> {
    StartOfEvents,
    Render,
    UserEvent(UserEvent),
    Crossterm(crossterm::event::Event),
}

enum InternalEvent<UserEvent> {
    UserEvent(UserEvent),
    Crossterm(crossterm::event::Event),
    Wake(&'static str),
}

pub struct TuiEventLoop<UserEvent> {
    tx: Sender<InternalEvent<UserEvent>>,
    rx: Receiver<InternalEvent<UserEvent>>,
}

impl<UserEvent> Default for TuiEventLoop<UserEvent>
where
    UserEvent: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<UserEvent> TuiEventLoop<UserEvent>
where
    UserEvent: Send + 'static,
{
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }

    pub fn create_proxy(&self) -> TuiEventLoopProxy<UserEvent> {
        TuiEventLoopProxy {
            tx: self.tx.clone(),
        }
    }

    pub fn run<F>(self, mut handler: F)
    where
        F: FnMut(&TuiEventLoopProxy<UserEvent>, TuiEvent<UserEvent>),
    {
        let Self { tx, rx } = self;

        let proxy = TuiEventLoopProxy { tx: tx.clone() };

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

                    if let Err(_) = tx.send(InternalEvent::Crossterm(event)) {
                        break;
                    }
                }
            }
        });

        let mut events = Vec::new();
        'main: loop {
            handler(&proxy, TuiEvent::StartOfEvents);

            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }

            for event in events.drain(..) {
                match event {
                    InternalEvent::UserEvent(user_event) => {
                        handler(&proxy, TuiEvent::UserEvent(user_event));
                        if ferrite_runtime::control_flow::get() == EventLoopControlFlow::Exit {
                            break 'main;
                        }
                    }
                    InternalEvent::Crossterm(crossterm_event) => {
                        handler(&proxy, TuiEvent::Crossterm(crossterm_event));
                        if ferrite_runtime::control_flow::get() == EventLoopControlFlow::Exit {
                            break 'main;
                        }
                    }
                    InternalEvent::Wake(reason) => {
                        tracing::debug!("term eventloop forced to wake because: {reason}");
                    }
                }
            }
            handler(&proxy, TuiEvent::Render);

            let control_flow = ferrite_runtime::control_flow::get();
            match control_flow {
                EventLoopControlFlow::Poll => {
                    if let Ok(event) = rx.try_recv() {
                        events.push(event);
                    }
                }
                EventLoopControlFlow::Wait => {
                    if let Ok(event) = rx.recv() {
                        events.push(event);
                    }
                }
                EventLoopControlFlow::Exit => break,
                EventLoopControlFlow::WaitMax(timeout) => {
                    if let Ok(event) = rx.recv_timeout(timeout) {
                        events.push(event);
                    }
                }
            }
            // If we where woken by some other cause we reset the control flow to wait
            // if the application has a reason to wake early it will because animations
            // use wait max controlflow and the dirty flag should force the application
            // code to run.
            ferrite_runtime::control_flow::set(
                ferrite_runtime::control_flow::EventLoopControlFlow::Wait,
            );
        }
    }
}

pub struct TuiEventLoopProxy<UserEvent> {
    tx: mpsc::Sender<InternalEvent<UserEvent>>,
}

impl<UserEvent> Clone for TuiEventLoopProxy<UserEvent> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<UserEvent: Send + 'static> EventLoopProxy<UserEvent> for TuiEventLoopProxy<UserEvent> {
    fn send(&self, event: UserEvent) {
        let _ = self.tx.send(InternalEvent::UserEvent(event));
    }

    fn request_render(&self, reason: &'static str) {
        let _ = self.tx.send(InternalEvent::Wake(reason));
    }

    fn dup(&self) -> Box<dyn EventLoopProxy<UserEvent>> {
        Box::new(self.clone())
    }
}
