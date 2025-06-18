use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

use encoding_rs::{CoderResult, Encoding};
use ferrite_utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};
use ropey::Rope;

use super::error::BufferError;

pub fn write(
    encoding: &'static Encoding,
    line_ending: LineEnding,
    rope: Rope,
    path: impl AsRef<Path>,
) -> Result<usize, BufferError> {
    let mut file = OpenOptions::new().create(true).write(true).open(path)?;
    #[cfg(unix)]
    let locked = rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive).is_ok();
    let res = write_inner(encoding, line_ending, rope, BufWriter::new(&mut file));
    #[cfg(unix)]
    if locked {
        let _ = rustix::fs::flock(&file, rustix::fs::FlockOperation::Unlock);
    }
    res
}

fn write_inner(
    encoding: &'static Encoding,
    line_ending: LineEnding,
    rope: Rope,
    mut file: BufWriter<&mut File>,
) -> Result<usize, BufferError> {
    const BUFFER_SIZE: usize = 8192;
    file.get_mut().set_len(0)?;

    let mut output_string = String::new();
    for line in rope.lines() {
        if line.get_line_ending().is_some() {
            for chunk in line.line_without_line_ending(0).chunks() {
                output_string.push_str(chunk);
            }
            output_string.push_str(line_ending.as_str());
        } else {
            for chunk in line.chunks() {
                output_string.push_str(chunk);
            }
            break;
        }
    }

    let mut encoder = encoding.new_encoder();
    let mut buffer = [0u8; BUFFER_SIZE];

    let mut total_written = 0;

    let mut remainder = output_string.as_str();
    loop {
        match encoder.encode_from_utf8(remainder, &mut buffer[total_written..], true) {
            (CoderResult::OutputFull, read, written, _) => {
                remainder = &remainder[read..];
                total_written += written;

                file.write_all(&buffer[..total_written])?;
                total_written = 0;
            }
            (CoderResult::InputEmpty, _, written, _) => {
                total_written += written;
                file.write_all(&buffer[..total_written])?;
                break;
            }
        }
    }

    file.flush()?;
    file.get_mut().sync_all()?;

    Ok(total_written)
}
