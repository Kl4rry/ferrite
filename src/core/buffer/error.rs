use std::{error::Error, fmt, io, path::PathBuf};

#[derive(Debug)]
pub enum BufferError {
    NoPathSet,
    InvalidPath(PathBuf),
    Io(io::Error),
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPathSet => writeln!(f, "Error no path set"),
            Self::InvalidPath(path) => writeln!(f, "Invalid path: '{}'", path.to_string_lossy()),
            Self::Io(err) => err.fmt(f),
        }
    }
}

impl Error for BufferError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for BufferError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
