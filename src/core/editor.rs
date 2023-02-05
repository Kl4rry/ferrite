use std::{io, path::Path};

use self::buffer::Buffer;

mod buffer;

/*pub enum EditorCommand {
    Scroll(i64),
}*/

pub struct Editor {
    pub buffer: Buffer,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let path = path.as_ref();
        Ok(Self {
            buffer: Buffer::from_file(path)?,
        })
    }

    /*pub fn send_cmd(&mut self, cmd: EditorCommand) {
        match cmd {
            EditorCommand::Scroll(lines) => {
                self.buffer.scroll(lines);
            }
        }
    }*/
}
