use std::{
    fs::File,
    io::{self},
    path::Path,
};

use encoding_rs::{CoderResult, Encoding};
use ropey::{Rope, RopeBuilder};

pub fn read(mut reader: impl io::Read) -> Result<(&'static Encoding, Rope), io::Error> {
    const BUFFER_SIZE: usize = 8192;
    let mut encoding_detector = chardetng::EncodingDetector::new();
    let mut content = Vec::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    let encoding = loop {
        let len = reader.read(&mut buffer)?;
        let filled = &buffer[..len];
        encoding_detector.feed(filled, len == 0);
        content.extend_from_slice(filled);

        if let (e, true) = encoding_detector.guess_assess(None, true) {
            break e;
        }
    };

    let mut decoder = encoding.new_decoder();
    let mut rope_builder = RopeBuilder::new();
    let mut output = String::with_capacity(BUFFER_SIZE);

    let mut input = &content[..];
    let mut file_empty = false;
    loop {
        if input.is_empty() {
            let read = reader.read(&mut buffer)?;
            input = &buffer[..read];
            if read == 0 {
                file_empty = true;
            }
        }
        let (result, read, _) = decoder.decode_to_string(input, &mut output, file_empty);
        input = &input[read..];
        match result {
            CoderResult::InputEmpty => {
                input = &[];
                if file_empty {
                    rope_builder.append(&output);
                    break;
                }
            }
            CoderResult::OutputFull => {
                rope_builder.append(&output);
                output.clear();
            }
        };
    }

    let rope = rope_builder.finish();

    Ok((encoding, rope))
}

pub fn read_from_file(path: impl AsRef<Path>) -> Result<(&'static Encoding, Rope), io::Error> {
    let mut file = File::open(path)?;
    // let locked = file.lock().is_ok();
    let res = read(&mut file);
    /*if locked {
        let _ = file.unlock();
    }*/
    res
}
