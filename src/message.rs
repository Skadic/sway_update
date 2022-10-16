use enum_primitive::FromPrimitive;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

use crate::{HEADER_LENGTH, I3_MAGIC_STRING, error::ResponseDeserializeError};

#[derive(Clone)]
pub struct Message {
    pub message_type: MessageType,
    pub payload: String,
}

impl Message {
    pub async fn from_read(read: impl AsyncRead + Unpin) -> Result<Self, ResponseDeserializeError> {
        let mut reader = BufReader::new(read);

        let header = &mut [0u8; HEADER_LENGTH];
        // Read the header
        reader.read_exact(header).await?;

        // Check if the magic string is correct
        if header[0..6] != I3_MAGIC_STRING {
            return Err(ResponseDeserializeError::InvalidMagicString(String::from_utf8_lossy(&header[0..6]).to_string()));
        }

        // The first 6 bytes of the header are "i3-msg", so we skip them and read the payload length and type
        let payload_len = u32::from_ne_bytes(header[6..10].try_into().unwrap()) as usize;
        let message_type = {
            let payload_type_int = u32::from_ne_bytes(header[10..14].try_into().unwrap());
            let reply_type_opt = MessageType::from_u32(payload_type_int);

            // Check that the payload type is valid in the reply
            if let Some(payload_type) = reply_type_opt {
                payload_type
            } else {
                return Err(ResponseDeserializeError::InvalidType(payload_type_int));
            }
        };

        // Read the actual payload
        let mut buf = vec![0u8; payload_len];
        reader.read_exact(&mut buf).await?;
        let payload = String::from_utf8_lossy(&buf).to_string();

        Ok(Self {
            message_type,
            payload,
        })
    }
}

enum_from_primitive! {
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(unused)]
pub enum MessageType {
    RunCommands = 0,
    GetWorkspaces = 1,
    Subscribe = 2,
    GetOutputs = 3,
    GetTree = 4,
    GetMarks = 5,
    GetBarConfig = 6,
    GetVersion = 7,
    GetBindingModes = 8,
    GetConfig = 9,
    SendTick = 10,
    Sync = 11,
    GetBindingState = 12,
    GetInputs = 100,
    GetSeats = 101,
}
}

impl MessageType {
    pub fn as_bytes(self) -> [u8; 4] {
        (self as u32).to_ne_bytes()
    }

    pub fn bytes(self) -> std::array::IntoIter<u8, 4> {
        self.as_bytes().into_iter()
    }
}

