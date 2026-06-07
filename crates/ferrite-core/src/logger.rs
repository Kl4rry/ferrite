use std::{
    io::Write,
    sync::{Mutex, mpsc},
};

use ropey::Rope;

use crate::event_loop_proxy::{EventLoopProxy, UserEvent};

static PROXY: Mutex<Option<Box<dyn EventLoopProxy<UserEvent>>>> = Mutex::new(None);

pub fn set_proxy(proxy: Box<dyn EventLoopProxy<UserEvent>>) {
    *PROXY.lock().unwrap() = Some(proxy);
}

pub struct LoggerSink {
    bytes: Vec<u8>,
    sender: mpsc::Sender<String>,
}

impl LoggerSink {
    pub fn new(sender: mpsc::Sender<String>) -> Self {
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
            if let Ok(string) = str::from_utf8(&self.bytes[last_line_start..line_start]) {
                let _ = self.sender.send(string.to_string());
            }

            last_line_start = line_start;
        }

        self.bytes.drain(..last_line_start);

        if last_line_start > 0
            && let Some(proxy) = &*PROXY.lock().unwrap()
        {
            proxy.request_render("new log messages ready");
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct LoggerState {
    pub rope: Rope,
    recv: mpsc::Receiver<String>,
}

impl LoggerState {
    pub fn new(recv: mpsc::Receiver<String>) -> Self {
        Self {
            rope: Rope::new(),
            recv,
        }
    }

    pub fn update(&mut self) {
        while let Ok(string) = self.recv.try_recv() {
            let len = self.rope.len_chars();
            self.rope.insert(len, &string);
        }
    }
}
