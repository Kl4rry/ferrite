use std::{
    collections::{hash_map::Entry, HashMap},
    io,
};

use crate::{
    buffer::Buffer,
    event_loop_proxy::EventLoopProxy,
    promise::Promise,
    search_buffer::{Preview, Previewer},
};

pub struct FilePreviewer {
    files: HashMap<String, Result<Buffer, io::Error>>,
    loading: HashMap<String, Promise<Result<Buffer, io::Error>>>,
    proxy: Box<dyn EventLoopProxy>,
}

impl FilePreviewer {
    pub fn new(proxy: Box<dyn EventLoopProxy>) -> Self {
        Self {
            files: HashMap::new(),
            loading: HashMap::new(),
            proxy,
        }
    }
}

impl Previewer<String> for FilePreviewer {
    fn request_preview(&mut self, m: &String) -> Preview {
        if let Entry::Occupied(mut entry) = self.loading.entry(m.clone()) {
            if let Some(result) = entry.get_mut().poll() {
                let (k, _) = entry.remove_entry();
                self.files.insert(k, result);
            }
        }

        match self.files.get_mut(m) {
            Some(Ok(buffer)) => return Preview::Buffer(buffer),
            Some(Err(_)) => return Preview::Err,
            None => (),
        }

        let path = m.clone();
        self.loading.insert(
            m.clone(),
            Promise::spawn(self.proxy.dup(), move || Buffer::from_file(path)),
        );
        Preview::Loading
    }
}
