use std::{
    fs,
    path::PathBuf,
    thread,
};

use anyhow::Result;
use sqlite::OpenFlags;

use crate::tui_app::event_loop::TuiEventLoopProxy;

use super::buffer;

pub enum PersistenceRequest {
    InitWorkspace(PathBuf),
    UpdateCursor(PathBuf, buffer::Cursor),
    GetCursorLocation(buffer::Cursor),
}

pub enum PersistenceResponse {
    CursorLocation(buffer::Cursor),
}

pub struct PersistenceManager {
    sender: cb::Sender<PersistenceRequest>,
    receiver: cb::Receiver<PersistenceResponse>,
    thread: Option<thread::JoinHandle<()>>,
}

impl PersistenceManager {
    pub fn get_default_location() -> Result<PathBuf> {
        let Some(directories) = directories::ProjectDirs::from("", "", "ferrite") else {
            return Err(anyhow::Error::msg("Unable to find project directory"));
        };
        Ok(directories.data_dir().join("state.db"))
    }

    pub fn open(proxy: TuiEventLoopProxy) -> Result<Self> {
        let connection = sqlite::open(Self::get_default_location()?)?;
        let (request_sender, request_receiver) = cb::unbounded();
        let (response_sender, response_receiver) = cb::unbounded();

        let thread = thread::spawn(move || while let Ok(request) = request_receiver.recv() {
            match request {
                PersistenceRequest::InitWorkspace(path) => {
                    if let Ok(path) = dunce::canonicalize(path) {
                        let path = path.to_string_lossy().to_string();
                        #[cfg(windows)]
                        let path = path.to_lowercase();
                        let query = "";
                        //connection.se
                    }
                },
                PersistenceRequest::UpdateCursor(_, _) => todo!(),
                PersistenceRequest::GetCursorLocation(_) => {
                    proxy.request_render();
                },
            }
        });

        Ok(Self {
            thread: Some(thread),
            sender: request_sender,
            receiver: response_receiver,
        })
    }

    pub fn init() -> Result<()> {
        let location = Self::get_default_location()?;
        let data_dir = {
            let mut path = location.clone();
            path.pop();
            path
        };

        fs::create_dir_all(data_dir)?;
        let connection = sqlite::Connection::open_with_flags(
            location,
            OpenFlags::new().set_create().set_read_write(),
        )?;
        connection.execute(include_str!("../../sqlite/init.sql"))?;
        Ok(())
    }

    pub fn queue_request(&self, msg: PersistenceRequest) -> bool {
        self.sender.send(msg).is_ok()
    }

    pub fn poll_response(&self) -> Option<PersistenceResponse> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for PersistenceManager {
    fn drop(&mut self) {
        self.thread.take().map(|t| t.join());
    }
}
