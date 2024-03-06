use std::{error::Error, fmt::Debug};
use thiserror::Error;

// ---------------------- Message Error ----------------------

#[derive(Debug, Error)]
pub enum ResponseDeserializeError {
    #[error("io error while reading response")]
    Io(#[from] std::io::Error),
    #[error("invalid magic string: \"{0}\", must be \"i3-msg\"")]
    InvalidMagicString(String),
    #[error("invalid message type: {0}")]
    InvalidMessageType(u32),
    #[error("invalid event type: {0}")]
    InvalidEventType(u32),
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
    Eww(#[from] EwwError<Box<dyn Error>>),
}

// ---------------------- Eww Error ----------------------

#[derive(Debug, Error)]
pub enum EwwError<Err> {
    #[error("error communicating with eww")]
    Io(#[from] std::io::Error),
    #[error("error parsing variable content")]
    ParseVar(Err),
    #[error("eww executable not found")]
    NoEwwExecutable,
}

impl<Err> EwwError<Err>
where
    Err: 'static + Error,
{
    pub fn boxed(self) -> EwwError<Box<dyn Error>> {
        match self {
            Self::Io(e) => EwwError::Io(e),
            Self::ParseVar(e) => EwwError::ParseVar(Box::new(e)),
            Self::NoEwwExecutable => EwwError::NoEwwExecutable,
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
    UnsuccessfulSubscription,
}

#[derive(Debug, Error)]
pub enum WorkspaceEventParseError {
    #[error("invalid workspace change: {0}")]
    Invalid(String),
}

#[derive(Debug, Error)]
pub enum SwayUpdateError {
    #[error("no events to subscribe to")]
    NoSubscriptionEvents,
    #[error("no active i3/sway ipc socket found")]
    NoSocket,
    #[error("error creating eww instance")]
    Eww(#[from] EwwError<()>),
    #[error("error creating daemon")]
    Daemon(#[from] DaemonError),
    #[error("error in event loop")]
    EventLoop(#[from] EventLoopError),
}

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("error connecting to unix sockete")]
    Connect(#[from] std::io::Error),
}
