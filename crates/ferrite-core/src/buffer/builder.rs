use std::path::PathBuf;

use crate::buffer::Buffer;

pub(super) enum Source<'a> {
    Bytes(&'a [u8]),
    Text(&'a str),
    File(PathBuf),
    Empty,
}

pub struct BufferBuilder<'a> {
    pub(super) simple: bool,
    pub(super) read_only: bool,
    pub(super) source: Source<'a>,
    pub(super) path: Option<PathBuf>,
    pub(super) name: Option<String>,
}

impl Default for BufferBuilder<'_> {
    fn default() -> Self {
        Self {
            simple: false,
            read_only: false,
            source: Source::Empty,
            path: None,
            name: None,
        }
    }
}

impl<'a> BufferBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn simple(&mut self, simple: bool) -> &mut Self {
        self.simple = simple;
        self
    }

    pub fn read_only(&mut self, read_only: bool) -> &mut Self {
        self.read_only = read_only;
        self
    }

    pub fn with_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_name(&mut self, name: impl Into<String>) -> &mut Self {
        self.name = Some(name.into());
        self
    }

    pub fn from_file(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        let path = path.into();
        self.source = Source::File(path.clone());
        self.path = Some(path);
        self
    }

    pub fn with_bytes(&'a mut self, bytes: &'a [u8]) -> &'a mut Self {
        self.source = Source::Bytes(bytes);
        self
    }

    pub fn with_text(&'a mut self, text: &'a str) -> &'a mut Self {
        self.source = Source::Text(text);
        self
    }

    pub fn build(&'a mut self) -> Result<Buffer, std::io::Error> {
        Buffer::from_builder(self)
    }
}
