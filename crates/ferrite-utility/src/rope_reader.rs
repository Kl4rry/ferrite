use std::{io::Read, iter::Peekable, ptr};

use ropey::{RopeSlice, iter::Chunks};

pub struct RopeReader<'a> {
    chunks: Peekable<Chunks<'a>>,
    bytes_read: usize,
}

impl<'a> RopeReader<'a> {
    pub fn new(rope: RopeSlice<'a>) -> Self {
        Self {
            chunks: rope.chunks().peekable(),
            bytes_read: 0,
        }
    }
}

impl Read for RopeReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.chunks.peek() {
            Some(chunk) => {
                let current = &chunk.as_bytes()[self.bytes_read..];
                let bytes_to_read = buf.len().min(current.len());
                unsafe {
                    ptr::copy_nonoverlapping(
                        current[..bytes_to_read].as_ptr(),
                        buf.as_mut_ptr(),
                        bytes_to_read,
                    );
                }
                self.bytes_read += bytes_to_read;

                if bytes_to_read == current.len() {
                    self.chunks.next();
                    self.bytes_read = 0;
                }
                Ok(bytes_to_read)
            }
            None => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use ropey::Rope;

    use super::*;

    #[test]
    fn rope_reader() {
        let text = include_str!("../../../Cargo.toml");
        let rope = Rope::from(text);
        let mut buffer = Vec::new();
        let mut reader = RopeReader::new(rope.slice(..));
        let _ = reader.read_to_end(&mut buffer);
        assert_eq!(rope.to_string().as_bytes(), buffer);
    }
}
