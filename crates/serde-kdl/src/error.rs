use {
    serde::{de, ser},
    std::fmt,
    thiserror::Error,
};

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    Custom(String),
    IO(#[from] std::io::Error),
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Error::Custom(message.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Error::Custom(message.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Custom(message) => f.write_str(message),
            Error::IO(_) => write!(f, "IO error"),
        }
    }
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
