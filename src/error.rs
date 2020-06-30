use std::{fmt, error};

#[derive(Debug)]
pub enum Error {
    System(String),
    Message(serde_json::Error),
}

impl fmt::Display for Error{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::System(err) => write!(f, "system error {}", err),
            Error::Message(err) => write!(f, "Invalid message {}", err),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Message(err)
    }
}

impl error::Error for Error {}
