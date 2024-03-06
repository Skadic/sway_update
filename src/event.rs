use std::str::FromStr;

use enum_primitive::FromPrimitive;
use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

use crate::{
    error::{ResponseDeserializeError, WorkspaceEventParseError},
    objects::{Window, Workspace},
    HEADER_LENGTH, I3_MAGIC_STRING,
};

#[derive(PartialEq, Eq, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}

impl Event {
    pub async fn from_read(read: impl AsyncRead + Unpin) -> Result<Self, ResponseDeserializeError> {
        let mut reader = BufReader::new(read);

        let header = &mut [0u8; HEADER_LENGTH];
        // Read the header
        reader.read_exact(header).await?;

        // Check if the magic string is correct
        if header[0..6] != I3_MAGIC_STRING {
            return Err(ResponseDeserializeError::InvalidMagicString(
                String::from_utf8_lossy(&header[0..6]).to_string(),
            ));
        }

        // The first 6 bytes of the header are "i3-msg", so we skip them and read the payload length and type
        let payload_len = u32::from_ne_bytes(header[6..10].try_into().unwrap()) as usize;
        let event_type = {
            let payload_type_int = u32::from_ne_bytes(header[10..14].try_into().unwrap());
            let reply_type_opt = EventType::from_u32(payload_type_int);

            // Check that the payload type is valid in the reply
            if let Some(payload_type) = reply_type_opt {
                payload_type
            } else {
                return Err(ResponseDeserializeError::InvalidEventType(payload_type_int));
            }
        };

        // Read the actual payload
        let mut buf = vec![0u8; payload_len];
        reader.read_exact(&mut buf).await?;
        let payload = String::from_utf8_lossy(&buf).to_string();

        Ok(Self {
            event_type,
            payload,
        })
    }
}

enum_from_primitive! {
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(unused)]
pub enum EventType {
    Workspace = 0x8000_0000,
    Mode = 0x8000_0002,
    Window = 0x8000_0003,
    BarConfigUpdate = 0x8000_0004,
    Binding = 0x8000_0005,
    Shutdown = 0x8000_0006,
    Tick = 0x8000_0007,
    BarStateUpdate = 0x8000_0014,
    Input = 0x8000_0015,
}
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum WorkspaceEventChange {
    Init,
    Empty,
    Focus,
    Move,
    Rename,
    Urgent,
    Reload,
}

impl FromStr for WorkspaceEventChange {
    type Err = WorkspaceEventParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use WorkspaceEventChange::*;
        Ok(match s {
            "init" => Init,
            "empty" => Empty,
            "focus" => Focus,
            "move" => Move,
            "rename" => Rename,
            "urgent" => Urgent,
            "reload" => Reload,
            _ => return Err(WorkspaceEventParseError::Invalid(s.to_string())),
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct WorkspaceEvent {
    pub change: WorkspaceEventChange,
    pub old: Option<Workspace>,
    pub current: Workspace,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WindowEvent {
    pub change: String,
    pub container: Window,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModeEvent {
    pub change: String,
    pub pango_markup: bool,
}
