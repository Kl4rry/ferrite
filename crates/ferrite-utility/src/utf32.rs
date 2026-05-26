use ferrite_ctx::{Arena, ArenaVec};

pub enum ArenaUtf32<'a> {
    /// Must only contains ascii chars
    Ascii(&'a [u8]),
    /// Can contain any char
    Unicode(ArenaVec<'a, char>),
}

impl<'a> ArenaUtf32<'a> {
    pub fn from_str_in(s: &'a str, arena: &'a Arena) -> Self {
        if s.is_ascii() {
            ArenaUtf32::Ascii(s.as_bytes())
        } else {
            let mut buffer = ArenaVec::new_in(arena);
            buffer.extend(s.chars());
            ArenaUtf32::Unicode(buffer)
        }
    }

    pub fn as_utf32_str(&self) -> nucleo::Utf32Str {
        match self {
            ArenaUtf32::Ascii(bytes) => nucleo::Utf32Str::Ascii(bytes),
            ArenaUtf32::Unicode(vec) => nucleo::Utf32Str::Unicode(vec),
        }
    }
}
