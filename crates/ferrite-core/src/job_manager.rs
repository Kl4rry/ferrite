use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

use crate::event_loop_proxy::EventLoopProxy;

pub struct JobHandle<T, P = ()> {
    end_recv: mpsc::Receiver<T>,
    progress_recv: mpsc::Receiver<P>,
    finished: bool,
    killed: Arc<AtomicBool>,
}

pub enum Progress<T, P> {
    Progress(P),
    End(T),
}

impl<T> JobHandle<T, ()> {
    pub fn try_recv(&mut self) -> Result<T, mpsc::TryRecvError> {
        let result = self.end_recv.try_recv();
        self.finished |= result.is_ok();
        result
    }
}

impl<T, P> JobHandle<T, P> {
    pub fn poll_progress(&mut self) -> Result<Progress<T, P>, mpsc::TryRecvError> {
        if let Ok(progress) = self.progress_recv.try_recv() {
            return Ok(Progress::Progress(progress));
        }

        let result = self.end_recv.try_recv()?;
        self.finished = true;
        Ok(Progress::End(result))
    }

    pub fn kill(&mut self) {
        self.killed.store(true, Ordering::Relaxed);
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

pub struct Progressor<T> {
    sender: mpsc::Sender<T>,
}

impl<T> Progressor<T> {
    pub fn make_progress(&mut self, t: T) {
        let _ = self.sender.send(t);
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
            if self.foreground_job[i - removed].is_finished() {
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
        P: Send + 'static,
        F: FnOnce(Arc<AtomicBool>, &mut Progressor<P>, I) -> O + Send + 'static,
    >(
        &mut self,
        f: F,
        input: I,
    ) -> JobHandle<O, P> {
        let killed = Arc::new(AtomicBool::new(false));
        let (end_tx, end_rx) = mpsc::channel();
        let (progress_tx, progress_rx) = mpsc::channel();
        let proxy = self.proxy.dup();
        let thread_killed = killed.clone();
        let handle = thread::spawn(move || {
            let output = f(
                thread_killed,
                &mut Progressor {
                    sender: progress_tx,
                },
                input,
            );
            let _ = end_tx.send(output);
            proxy.request_render();
        });

        self.foreground_job.push(handle);
        JobHandle {
            end_recv: end_rx,
            progress_recv: progress_rx,
            finished: false,
            killed,
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
