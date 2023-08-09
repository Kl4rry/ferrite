use std::{fmt, num};

#[derive(Debug)]
pub enum CommandParseError {
    UnkownCommand(String),
    MissingArgs(String),
    UnknownArg(String),
    IntParse(num::ParseIntError),
    Custom(String),
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnkownCommand(cmd) => writeln!(f, "Unknown command: '{cmd}'"),
            Self::MissingArgs(usage) => writeln!(f, "Missing args usage: '{usage}'"),
            Self::UnknownArg(arg) => writeln!(f, "Unknown argument: '{arg}'"),
            Self::Custom(msg) => msg.fmt(f),
            Self::IntParse(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for CommandParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IntParse(err) => Some(err),
            _ => None,
        }
    }
}

impl From<num::ParseIntError> for CommandParseError {
    fn from(value: num::ParseIntError) -> Self {
        Self::IntParse(value)
    }
}
