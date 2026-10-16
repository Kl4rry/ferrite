use std::{cell::Cell, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLoopControlFlow {
    Poll,
    Wait,
    Exit,
    WaitMax(Duration),
}

thread_local! {
    static CONTROL_FLOW: Cell<EventLoopControlFlow> = const { Cell::new(EventLoopControlFlow::Wait) };
}

pub fn get() -> EventLoopControlFlow {
    CONTROL_FLOW.get()
}

pub fn set(control_flow: EventLoopControlFlow) {
    CONTROL_FLOW.set(control_flow);
}

pub fn wait_max(duration: Duration) {
    match CONTROL_FLOW.get() {
        EventLoopControlFlow::Poll => (),
        EventLoopControlFlow::Wait => CONTROL_FLOW.set(EventLoopControlFlow::WaitMax(duration)),
        EventLoopControlFlow::Exit => (),
        EventLoopControlFlow::WaitMax(old) => {
            CONTROL_FLOW.set(EventLoopControlFlow::WaitMax(if old < duration {
                old
            } else {
                duration
            }))
        }
    }
}
