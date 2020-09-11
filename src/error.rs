use std::convert::From;
use std::fmt;
use std::io;
use std::string::FromUtf8Error;

use tokio::sync::mpsc::error::SendError;

use crate::Message;

pub type WebsocketResult<T> = Result<T, WebsocketError>;

#[derive(Debug)]
pub enum WebsocketError {
    IoError(io::Error),
    SendError(SendError<Message>),
    FromUtf8Error(FromUtf8Error),
}

impl fmt::Display for WebsocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            WebsocketError::IoError(ref err) => write!(f, "{}", err),
            WebsocketError::SendError(ref err) => write!(f, "{}", err),
            WebsocketError::FromUtf8Error(ref err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for WebsocketError {

}

impl From<io::Error> for WebsocketError {
    fn from(err: io::Error) -> Self {
        WebsocketError::IoError(err)
    }
}

impl From<SendError<Message>> for WebsocketError {
    fn from(err: SendError<Message>) -> Self {
        WebsocketError::SendError(err)
    }
}

impl From<FromUtf8Error> for WebsocketError {
    fn from(err: FromUtf8Error) -> Self {
        WebsocketError::FromUtf8Error(err)
    }
}
