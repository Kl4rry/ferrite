use self::buffer::Buffer;

mod buffer;

pub struct Editor {
    pub buffer: Buffer,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(""),
        }
    }
}
