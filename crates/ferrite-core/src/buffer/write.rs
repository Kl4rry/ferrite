use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
};

use encoding_rs::{CoderResult, Encoding};
use ferrite_utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};
use ropey::{Rope, RopeBuilder};

use super::error::BufferError;

pub fn write(
    encoding: &'static Encoding,
    line_ending: LineEnding,
    rope: Rope,
    path: impl AsRef<Path>,
) -> Result<usize, BufferError> {
    let path = path.as_ref().to_path_buf();
    const BUFFER_SIZE: usize = 8192;

    let mut file = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?,
    );

    let mut output_rope = RopeBuilder::new();
    for line in rope.lines() {
        if line.get_line_ending().is_some() {
            for chunk in line.line_without_line_ending(0).chunks() {
                output_rope.append(chunk);
            }
            output_rope.append(line_ending.as_str());
        } else {
            for chunk in line.chunks() {
                output_rope.append(chunk);
            }
            break;
        }
    }
    let rope = output_rope.finish();

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
    file.get_mut().sync_all()?;

    Ok(total_written)
}
