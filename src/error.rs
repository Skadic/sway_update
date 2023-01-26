use std::{error::Error, fmt::Debug};
use thiserror::Error;

// ---------------------- Message Error ----------------------

#[derive(Debug,Error)]
pub enum ResponseDeserializeError {
    #[error("io error while reading response")]
    Io(#[from] std::io::Error),
    #[error("invalid magic string: \"{0}\", must be \"i3-msg\"")]
    InvalidMagicString(String),
    #[error("invalid type: {0}")]
    InvalidType(u32),
}

// ---------------------- Event Error ----------------------

#[derive(Debug, Error)]
pub enum EventError {
    #[error("error reading event from socket")]
    Read(#[from] ResponseDeserializeError),
    #[error("error processing request")]
    Request(#[from] RequestError),
    #[error("error deserializing payload")]
    DeserializePayload(#[from] serde_json::error::Error),
    #[error("error communicating with eww")]
    Eww(#[from] EwwError<Box<dyn Error>>)
}

// ---------------------- Eww Error ----------------------

#[derive(Debug, Error)]
pub enum EwwError<Err> {
    #[error("error communicating with eww")]
    Io(#[from] std::io::Error),
    #[error("error parsing variable content")]
    ParseVar(Err)
}

impl<Err> EwwError<Err> where Err: 'static + Error {
    pub fn boxed(self) -> EwwError<Box<dyn Error>> {
        match self {
            Self::Io(e) => EwwError::Io(e),
            Self::ParseVar(e) => EwwError::ParseVar(Box::new(e))
        }
    }
}


// ---------------------- Event Loop Error ----------------------

#[derive(Debug, Error)]
pub enum EventLoopError {
    #[error("error subscribing to events")]
    Subscription(#[from] RequestError),
    #[error("error reading event from socket")]
    Read(#[from] ResponseDeserializeError),
    #[error("error during event handling")]
    Event(#[from] EventError),
}

// ---------------------- Request Error ----------------------

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("error reading response from socket")]
    Read(#[from] ResponseDeserializeError),
    #[error("error interacting with eww")]
    Eww(#[from] EwwError<Box<dyn Error>>),
    #[error("error deserializing payload")]
    Deserialize(serde_json::error::Error),
    #[error("error serializing payload")]
    Serialize(serde_json::error::Error),
    #[error("could not subscribe to event bus")]
    UnsuccessfulSubscription
}
