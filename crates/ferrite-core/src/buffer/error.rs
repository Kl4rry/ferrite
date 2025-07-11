use std::{error::Error, fmt, io};

#[derive(Debug)]
pub enum BufferError {
    NoPathSet,
    Io(io::Error),
    #[cfg(unix)]
    Errno(rustix::io::Errno),
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPathSet => writeln!(f, "Error no path set"),
            Self::Io(err) => err.fmt(f),
            #[cfg(unix)]
            Self::Errno(err) => err.fmt(f),
        }
    }
}

impl Error for BufferError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            #[cfg(unix)]
            Self::Errno(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for BufferError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[cfg(unix)]
impl From<rustix::io::Errno> for BufferError {
    fn from(value: rustix::io::Errno) -> Self {
        Self::Errno(value)
    }
}
