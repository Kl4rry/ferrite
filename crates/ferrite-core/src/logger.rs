use std::{
    collections::VecDeque,
    io::Write,
    sync::{mpsc, Mutex},
};

use serde::Deserialize;

use crate::{cmd::Cmd, event_loop_proxy::EventLoopProxy};

static PROXY: Mutex<Option<Box<dyn EventLoopProxy>>> = Mutex::new(None);

pub fn set_proxy(proxy: Box<dyn EventLoopProxy>) {
    *PROXY.lock().unwrap() = Some(proxy);
}

pub struct LoggerSink {
    bytes: Vec<u8>,
    sender: mpsc::Sender<LogMessage>,
}

impl LoggerSink {
    pub fn new(sender: mpsc::Sender<LogMessage>) -> Self {
        Self {
            bytes: Vec::new(),
            sender,
        }
    }
}

impl Write for LoggerSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes.extend_from_slice(buf);
        let mut line_starts = Vec::new();
        for (i, byte) in self.bytes.iter().enumerate() {
            if *byte == b'\n' && i != 0 {
                line_starts.push(i);
            }
        }

        let mut last_line_start = 0;
        for line_start in line_starts {
            if let Ok(msg) = serde_json::from_slice(&self.bytes[last_line_start..line_start]) {
                let _ = self.sender.send(msg);
            }

            last_line_start = line_start;
        }

        self.bytes.drain(..last_line_start);

        if last_line_start > 0 {
            if let Some(proxy) = &*PROXY.lock().unwrap() {
                proxy.request_render();
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct LogMessage {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub fields: Fields,
}

#[derive(Debug, Deserialize)]
pub struct Fields {
    pub message: String,
}

#[derive(Debug)]
pub struct LoggerState {
    pub lines_scrolled_up: usize,
    pub messages: VecDeque<LogMessage>,
    recv: mpsc::Receiver<LogMessage>,
}

impl LoggerState {
    pub fn new(recv: mpsc::Receiver<LogMessage>) -> Self {
        Self {
            lines_scrolled_up: 0,
            messages: VecDeque::new(),
            recv,
        }
    }

    pub fn update(&mut self) {
        while let Ok(msg) = self.recv.try_recv() {
            self.messages.push_front(msg);
            if self.lines_scrolled_up != 0 {
                self.lines_scrolled_up += 1;
            }
        }

        while self.messages.len() > 5000 {
            self.messages.pop_back();
        }
    }

    pub fn handle_input(&mut self, input: Cmd) {
        match input {
            Cmd::VerticalScroll { distance } => {
                self.lines_scrolled_up = (self.lines_scrolled_up as i64 - distance).max(0) as usize;
            }
            Cmd::End { .. } => self.lines_scrolled_up = 0,
            Cmd::Escape { .. } => self.lines_scrolled_up = 0,
            _ => (),
        }
    }
}
