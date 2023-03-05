use std::{
    fs,
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
};

use encoding_rs::{CoderResult, Encoding};
use ropey::Rope;

use super::error::BufferError;

pub fn write(
    encoding: &'static Encoding,
    rope: Rope,
    path: impl AsRef<Path>,
) -> Result<(), BufferError> {
    let path = path.as_ref().to_path_buf();
    const BUFFER_SIZE: usize = 8192;

    let Some(parent) = path.parent() else {
        return Err(BufferError::InvalidPath(path))
    };

    let Some(filename) = path.file_name() else {
        return Err(BufferError::InvalidPath(path))
    };

    let temp = Path::new(parent).join(format!(".{}.part", filename.to_string_lossy()));
    let mut file = BufWriter::new(
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?,
    );

    let mut encoder = encoding.new_encoder();
    let mut buffer = [0u8; BUFFER_SIZE];

    let mut total_written = 0;

    let mut write = |chunk: &str, last: bool| -> Result<(), BufferError> {
        let mut remainder = chunk;
        loop {
            match encoder.encode_from_utf8(remainder, &mut buffer[total_written..], last) {
                (CoderResult::OutputFull, read, written, _) => {
                    remainder = &remainder[read..];
                    total_written += written;

                    file.write_all(&buffer[..total_written])?;
                    total_written = 0;
                }
                (CoderResult::InputEmpty, _, written, _) => {
                    total_written += written;
                    if last {
                        file.write_all(&buffer[..total_written])?;
                    }
                    break;
                }
            }
        }
        Ok(())
    };

    for chunk in rope.chunks() {
        write(chunk, false)?;
    }

    write("", true)?;

    file.flush()?;
    fs::rename(temp, path)?;

    Ok(())
}
