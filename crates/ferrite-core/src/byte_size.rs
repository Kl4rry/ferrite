use humansize::{BINARY, format_size};

pub fn format_byte_size(bytes: usize) -> String {
    format_size(bytes, BINARY.space_after_value(false))
}
