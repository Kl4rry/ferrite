use std::{
    sync::mpsc,
    thread::{self, JoinHandle},
};

use crate::event_loop_proxy::EventLoopProxy;

pub struct JobHandle<T> {
    recv: mpsc::Receiver<T>,
    finished: bool,
}

impl<T> JobHandle<T> {
    pub fn recv_try(&mut self) -> Result<T, mpsc::TryRecvError> {
        let result = self.recv.try_recv();
        self.finished |= result.is_ok();
        result
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

pub struct JobManager {
    proxy: Box<dyn EventLoopProxy>,
    foreground_job: Vec<JoinHandle<()>>,
}

impl JobManager {
    pub fn new(proxy: Box<dyn EventLoopProxy>) -> Self {
        Self {
            proxy,
            foreground_job: Vec::new(),
        }
    }

    pub fn poll_jobs(&mut self) {
        let mut removed = 0;
        for i in 0..self.foreground_job.len() {
            if self.foreground_job[i].is_finished() {
                let _ = self.foreground_job.remove(i - removed).join();
                removed += 1;
            }
        }
    }

    /// A foreground job is a job that displays a working spinner
    /// All foreground jobs are required to finish before application exit
    pub fn spawn_foreground_job<
        I: Send + 'static,
        O: Send + 'static,
        F: FnOnce(I) -> O + Send + 'static,
    >(
        &mut self,
        f: F,
        input: I,
    ) -> JobHandle<O> {
        let (tx, rx) = mpsc::channel();
        let proxy = self.proxy.dup();
        let handle = thread::spawn(move || {
            let output = f(input);
            let _ = tx.send(output);
            proxy.request_render();
        });

        self.foreground_job.push(handle);
        JobHandle {
            recv: rx,
            finished: false,
        }
    }
}

impl Drop for JobManager {
    fn drop(&mut self) {
        for handle in self.foreground_job.drain(..) {
            let _ = handle.join();
        }
    }
}
