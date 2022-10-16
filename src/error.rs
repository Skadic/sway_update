use std::{error::Error, fmt::{Display, Debug}};

/// Implements simple from conversions for Error enum types
macro_rules! quick_from_err {
    // Non generic variant
    {$err:ident, $($src:path => $var:ident),+ $(,)?} => {
        $(
            impl From<$src> for $err {
                fn from(err: $src) -> Self {
                    Self::$var(err)
                }
            }
        )+
    };
    // Generic with just one 
    { $err:ident<$($gen:tt),+>, $src:path => $var:ident $(,)? } => {
        impl<$($gen)+> From<$src> for $err<$($gen)+> {
            fn from(err: $src) -> Self {
                Self::$var(err)
            }
        }
    };
    {
        $err:ident<$($gen:tt),+>, 
        $src:path => $var:ident, 
        $($srces:path => $vars:ident),+ $(,)?
    } => {
        impl<$($gen)+> From<$src> for $err<$($gen)+> {
            fn from(err: $src) -> Self {
                Self::$var(err)
            }
        }
        quick_from_err!{$err<$($gen),+>, $($srces => $vars),+}
    };
}

// ---------------------- Message Error ----------------------

#[derive(Debug)]
pub enum ResponseDeserializeError {
    Io(std::io::Error),
    InvalidMagicString(String),
    InvalidType(u32),
}

quick_from_err!{
    ResponseDeserializeError, 
    std::io::Error => Io
}


impl Display for ResponseDeserializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(_) => write!(f, "io error while reading response"),
            Self::InvalidMagicString(s) => write!(f, "invalid magic string: \"{s}\", must be \"i3-msg\""),
            Self::InvalidType(type_int) => write!(f, "invalid type: {type_int}"),
        }
    }
}

impl Error for ResponseDeserializeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Io(e) => e,
            _ => return None
        })
    }
}

// ---------------------- Event Error ----------------------

#[derive(Debug)]
pub enum EventError {
    Read(ResponseDeserializeError),
    Request(RequestError),
    DeserializePayload(serde_json::error::Error),
    Eww(EwwError<Box<dyn Error>>)
}

quick_from_err!{
    EventError, 
    ResponseDeserializeError => Read,
    serde_json::error::Error => DeserializePayload,
    EwwError<Box<dyn Error>> => Eww,
    RequestError => Request
}

impl Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} while handling event", match self {
            Self::Read(_) => "error reading event from socket",
            Self::DeserializePayload(_) => "error deserializing payload",
            Self::Eww(_) => "error communicating with eww",
            Self::Request(_) => "error processing request",
        })
    }
}

impl Error for EventError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            EventError::Read(e) => e,
            EventError::DeserializePayload(e) => e,
            EventError::Request(e) => e,
            Self::Eww(EwwError::Io(e)) => e,
            Self::Eww(EwwError::ParseVar(e)) => e.as_ref(),
        })
    }
}

// ---------------------- Eww Error ----------------------

#[derive(Debug)]
pub enum EwwError<Err> {
    Io(std::io::Error),
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

impl<Err> Display for EwwError<Err> where Err: Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(_) => write!(f, "error communicating with eww"),
            Self::ParseVar(_) => write!(f, "error parsing variable content")
        }
    }
}

impl<Err> Error for EwwError<Err> where Err: 'static + Error {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Io(e) => e,
            Self::ParseVar(e) => e,
        })
    }
}


// ---------------------- Event Loop Error ----------------------

#[derive(Debug)]
pub enum EventLoopError {
    Subscription(RequestError),
    Read(ResponseDeserializeError),
    Event(EventError),
}

quick_from_err! {
    EventLoopError,
    RequestError => Subscription,
    ResponseDeserializeError => Read,
    EventError => Event,
}

impl Display for EventLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Subscription(_) => write!(f, "error subscribing to events"),
            Self::Read(_) => write!(f, "error reading event from socket"),
            Self::Event(_) => write!(f, "error during event handling"),
        }
    }
}

impl Error for EventLoopError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Subscription(e) => e,
            Self::Read(e) => e,
            Self::Event(e) => e,
        })
    }
}

// ---------------------- Request Error ----------------------

#[derive(Debug)]
pub enum RequestError {
    Io(std::io::Error),
    Read(ResponseDeserializeError),
    Eww(EwwError<Box<dyn Error>>),
    Deserialize(serde_json::error::Error),
    Serialize(serde_json::error::Error),
    UnsuccessfulSubscription
}

quick_from_err!{
    RequestError, 
    std::io::Error => Io,
    ResponseDeserializeError => Read,
    EwwError<Box<dyn Error>> => Eww,
}

impl Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RequestError::*;
        match self {
            Eww(EwwError::ParseVar(_)) => write!(f, "error parsing eww variable content"),
            Eww(EwwError::Io(_)) | Io(_) => write!(f, "io error"),
            Read(_) => write!(f, "error reading response from socket"),
            Deserialize(_) => write!(f, "error deserializing payload"),
            Serialize(_) => write!(f, "error serializing data to be sent to eww"),
            UnsuccessfulSubscription => write!(f, "subscription unsuccessful")
        }
    }
} 

impl Error for RequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use RequestError::*;
        Some(match self {
            Eww(EwwError::ParseVar(e)) => e.as_ref(),
            Io(e) | Eww(EwwError::Io(e)) => e,
            Read(e) => e,
            Deserialize(e) => e,
            Serialize(e) => e,
            _ => return None
        })
    }
}