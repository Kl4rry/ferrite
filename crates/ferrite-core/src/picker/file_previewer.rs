use std::{
    collections::{HashMap, hash_map::Entry},
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

use crate::{
    buffer::Buffer,
    event_loop_proxy::EventLoopProxy,
    picker::{Preview, Previewer},
    promise::Promise,
};

pub fn is_text_file(path: impl AsRef<Path>) -> Result<bool, io::Error> {
    let mut file = File::open(&path)?;

    let mut buf = [0; 1024];
    let read = file.read(&mut buf)?;

    let content_type = content_inspector::inspect(&buf[..read]);
    Ok(content_type.is_text())
}

pub struct FilePreviewer {
    files: HashMap<String, Result<Option<Buffer>, io::Error>>,
    loading: HashMap<String, Promise<Result<Option<Buffer>, io::Error>>>,
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
            Some(Ok(Some(buffer))) => return Preview::Buffer(buffer),
            Some(Ok(None)) => return Preview::Binary,
            Some(Err(_)) => return Preview::Err,
            None => (),
        }

        let path = m.clone();
        if let Ok(metadata) = fs::metadata(&path) {
            const MAX_PREVIEW_SIZE: u64 = 1_000_000;
            if metadata.len() > MAX_PREVIEW_SIZE {
                return Preview::TooLarge;
            }
        }

        self.loading.insert(
            m.clone(),
            Promise::spawn(self.proxy.dup(), move || {
                if !is_text_file(&path)? {
                    return Ok(None);
                }
                Ok(Some(Buffer::builder().from_file(&path).build()?))
            }),
        );
        Preview::Loading
    }
}
