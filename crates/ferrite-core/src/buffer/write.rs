use std::{
    ffi::OsString,
    fs,
    fs::{File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use encoding_rs::{CoderResult, Encoding};
use ferrite_utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};
use ropey::Rope;

use super::error::BufferError;

fn create_tmp_file_path(path: impl AsRef<Path>) -> Result<PathBuf, BufferError> {
    let path = path.as_ref();
    let Some(file_name) = path.file_name() else {
        return Err(io::Error::other("path has no filename").into());
    };
    let Some(parent) = path.parent() else {
        return Err(io::Error::other("path has no parent").into());
    };
    let tmp_file_path = parent.with_file_name({
        let prefix = ".~";
        let postfix = ".tmp";
        let mut tmp = OsString::with_capacity(file_name.len() + prefix.len() + postfix.len());
        tmp.push(prefix);
        tmp.push(file_name);
        tmp.push(postfix);
        tmp
    });
    Ok(tmp_file_path)
}

pub fn write(
    encoding: &'static Encoding,
    line_ending: LineEnding,
    rope: Rope,
    path: impl AsRef<Path>,
) -> Result<usize, BufferError> {
    let tmp_file_path = create_tmp_file_path(&path)?;
    let mut create = true;
    let mut atomic_write = false;
    // This has a TOCTU but I don't really care
    if let Ok(metadata) = fs::metadata(&path)
        && metadata.is_file()
    {
        // Copy file to keep metadata
        if fs::copy(&path, &tmp_file_path).is_ok() {
            atomic_write = true;
            create = false;
        }
    }

    let mut open_options = OpenOptions::new();
    open_options.create(create);
    open_options.truncate(false);
    open_options.write(true);

    // If we faild to copy the file fall back to writing to the real file
    // this might lead to non atomic writes but it's better then failing
    let mut file = match open_options.open(&tmp_file_path) {
        Ok(file) => {
            atomic_write = true;
            file
        }
        Err(_) => open_options.open(&path)?,
    };

    let bytes_written = match write_inner(encoding, line_ending, rope, BufWriter::new(&mut file)) {
        Ok(bytes_written) => bytes_written,
        Err(err) => {
            if atomic_write {
                fs::remove_file(tmp_file_path)?;
            }
            return Err(err);
        }
    };

    if atomic_write && let Err(err) = fs::rename(&tmp_file_path, &path) {
        fs::remove_file(tmp_file_path)?;
        return Err(err.into());
    }

    Ok(bytes_written)
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
