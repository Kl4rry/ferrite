use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use ferrite_utility::worker::{Consumption, Worker};
use ropey::Rope;

use crate::event_loop_proxy;

pub struct GitDiff {
    worker: Worker<WorkerState, Input, ()>,
    output: Arc<Mutex<Option<imara_diff::Diff>>>,
}

impl GitDiff {
    pub fn new(after: Rope) -> Self {
        let output = Arc::new(Mutex::new(None));
        let worker = Worker::new(
            Consumption::InOrder,
            WorkerState {
                after,
                before: None,
                output: output.clone(),
            },
            |state: &mut WorkerState, input: Input| {
                match input {
                    Input::BeforeUpdate(path) => {
                        let Ok(before) = ferrite_git::diff::get_diff_base(path) else {
                            return;
                        };
                        state.before = Some(before);
                    }
                    Input::AfterUpdate(rope) => {
                        state.after = rope;
                    }
                }

                if let Some(before) = &state.before {
                    let diff = ferrite_git::diff::line_diff(before.clone(), state.after.clone());
                    *state.output.lock().unwrap() = Some(diff);
                    event_loop_proxy::get_proxy().request_render("syntax update parsed");
                }
            },
        );
        Self { worker, output }
    }

    pub fn update_before(&mut self, path: PathBuf) {
        self.worker.send(Input::BeforeUpdate(path));
    }

    pub fn update_after(&mut self, rope: Rope) {
        self.worker.send(Input::AfterUpdate(rope));
    }

    pub fn diff(&mut self) -> MutexGuard<Option<imara_diff::Diff>> {
        self.output.lock().unwrap()
    }
}

#[derive(Debug)]
enum Input {
    BeforeUpdate(PathBuf),
    AfterUpdate(Rope),
}

struct WorkerState {
    before: Option<Rope>,
    after: Rope,
    output: Arc<Mutex<Option<imara_diff::Diff>>>,
}
