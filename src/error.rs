use std::{fmt, error};

#[derive(Debug)]
pub enum Error {
    System(String),
    Message(serde_json::Error),
    Rusql(rusqlite::Error)
}

impl fmt::Display for Error{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::System(err) => write!(f, "system error {}", err),
            Error::Message(err) => write!(f, "Invalid message {}", err),
            Error::Rusql(err) => write!(f, "Rusql error {}", err),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Message(err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Rusql(err)
    }
}

impl error::Error for Error {}
