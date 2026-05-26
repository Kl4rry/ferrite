use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

type F<State, Input, Output> = fn(&mut State, Input) -> Output;

pub enum Consumption {
    /// Skip all messages except the latest message
    Latest,
    /// Consume all messages in order
    InOrder,
}

struct InternalState<State, Input, Output> {
    state: State,
    closure: F<State, Input, Output>,
    tx: mpsc::Sender<Output>,
    rx: mpsc::Receiver<Input>,
    consumption: Consumption,
}

pub struct Worker<State, Input, Output> {
    state: Arc<Mutex<InternalState<State, Input, Output>>>,
    running: Arc<AtomicBool>,
    tx: mpsc::Sender<Input>,
    rx: mpsc::Receiver<Output>,
}

impl<State, Input, Output> Worker<State, Input, Output>
where
    State: Send + 'static,
    Input: Send + 'static,
    Output: Send + 'static,
{
    pub fn new(consumption: Consumption, state: State, func: F<State, Input, Output>) -> Self {
        let (input_tx, input_rx) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        let running = Arc::new(AtomicBool::new(false));

        let internal_state = Arc::new(Mutex::new(InternalState {
            state,
            closure: func,
            tx: output_tx,
            rx: input_rx,
            consumption,
        }));

        Self {
            state: internal_state,
            running,
            tx: input_tx,
            rx: output_rx,
        }
    }

    pub fn send(&mut self, input: Input) {
        let _ = self.tx.send(input);
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let state = self.state.clone();
            let running = self.running.clone();
            rayon::spawn(move || {
                let mut guard = state.lock().unwrap();
                loop {
                    let input = match guard.consumption {
                        Consumption::Latest => {
                            let mut latest = None;
                            while let Ok(input) = guard.rx.try_recv() {
                                latest = Some(input);
                            }
                            match latest {
                                Some(input) => input,
                                None => break,
                            }
                        }
                        Consumption::InOrder => match guard.rx.try_recv() {
                            Ok(input) => input,
                            Err(_) => break,
                        },
                    };
                    let output = (guard.closure)(&mut guard.state, input);
                    if guard.tx.send(output).is_err() {
                        break;
                    }
                }
                running.store(false, Ordering::SeqCst);
            });
        }
    }

    pub fn recv(&mut self) -> Option<Output> {
        self.rx.recv().ok()
    }
}
