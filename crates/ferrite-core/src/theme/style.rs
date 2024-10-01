use core::{fmt, str};
use std::{error::Error, str::FromStr};

#[derive(Debug)]
pub struct ParseColorError(&'static str);

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for ParseColorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl FromStr for Color {
    type Err = ParseColorError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 7 {
            return Err(ParseColorError("Color is not 7 chars long"));
        }

        if !s.starts_with("#") {
            return Err(ParseColorError("Color does not start with #"));
        }

        let bytes = s.as_bytes();
        for byte in &bytes[1..] {
            if !byte.is_ascii_hexdigit() {
                return Err(ParseColorError("All chars must by valid ascii hex digits"));
            }
        }

        let r = f32::from(unsafe {
            u8::from_str_radix(str::from_utf8_unchecked(&bytes[1..3]), 16).unwrap_unchecked()
        }) / 255.0;
        let g = f32::from(unsafe {
            u8::from_str_radix(str::from_utf8_unchecked(&bytes[3..5]), 16).unwrap_unchecked()
        }) / 255.0;
        let b = f32::from(unsafe {
            u8::from_str_radix(str::from_utf8_unchecked(&bytes[5..7]), 16).unwrap_unchecked()
        }) / 255.0;

        Ok(Self { r, g, b })
    }
}

#[derive(Debug, Default, Clone)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}
