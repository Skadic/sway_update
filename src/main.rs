use error::{
    DaemonError, EventError, EventLoopError, EwwError, RequestError, ResponseDeserializeError,
    SwayUpdateError,
};
use event::{EventType, ModeEvent, WindowEvent};
use message::{Message, MessageType};

use objects::{Workspace, WorkspaceInfo};
use std::{collections::HashMap, error::Error, path::Path, process::Command, str::FromStr};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixStream,
};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::EnvFilter;

use crate::event::Event;

#[macro_use]
extern crate enum_primitive;

mod error;
mod event;
mod message;
mod objects;

const I3_MAGIC_STRING: [u8; 6] = *b"i3-ipc";
const HEADER_LENGTH: usize = 14;

#[tokio::main]
async fn main() -> Result<(), SwayUpdateError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    let subscription = {
        let tokens = std::env::args().skip(1).collect::<Vec<_>>();
        if tokens.is_empty() {
            return Err(SwayUpdateError::NoSubscriptionEvents);
        };
        format!("{tokens:?}")
    };

    debug!(?subscription, "Enabled Subscriptions");

    let sway_socket_addr = std::env::var("I3SOCK")
        .or_else(|_| std::env::var("SWAYSOCK"))
        .or_else(|_| {
            std::process::Command::new("sway")
                .arg("--get-socketpath")
                .output()
                .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        })
        .ok()
        .filter(|s| !s.is_empty())
        .expect("Could not determine socket path. Is sway running?");

    // This object checks if it can find an eww instance in your path
    let eww = Eww::new()?;

    debug!(address = sway_socket_addr, "Sway Socket Address");
    debug!("Eww executable: {}", eww.binary);

    let mut daemon = Daemon::new(&sway_socket_addr, eww).await?;

    let res = daemon.subscribe_event_loop(&subscription).await;

    if let Err(e) = res {
        error!("Error in event loop: {e}");
        return Err(e.into());
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct Eww {
    pub binary: String,
}

impl Eww {
    pub fn new() -> Result<Self, EwwError<()>> {
        let eww_executable = {
            let output = Command::new("which").arg("eww").output()?.stdout;

            // SAFETY Either the output of this is empty or it returns the path to eww
            // so this is always valid utf8
            let eww_path_str = {
                let mut temp = unsafe { String::from_utf8_unchecked(output) };
                // Trim ending whitespace in-place
                temp.truncate(temp.trim_end().len());
                temp
            };
            let eww_path = Path::new(&eww_path_str);

            if !eww_path.exists() {
                error!("eww executable not found. If it can't be found by \"which\" there is probably something wrong.");
                return Err(EwwError::NoEwwExecutable);
            }

            eww_path_str
        };

        Ok(Self {
            binary: eww_executable,
        })
    }

    pub fn set_var<T: FromStr + ToString>(
        &self,
        var: &str,
        val: &T,
    ) -> Result<bool, EwwError<<T as FromStr>::Err>> {
        let val = val.to_string();
        let success = Command::new(&self.binary)
            .arg("update")
            .arg(format!("{var}={val}"))
            .spawn()
            .map_err(EwwError::Io)?
            .wait()
            .map_err(EwwError::Io)?
            .success();
        if success {
            debug!("Updated eww variable \"{var}\" to value \"{val}\"")
        } else {
            warn!("Error updating eww variable \"{var}\"")
        }
        Ok(success)
    }

    #[allow(unused)]
    pub fn get_var<T: FromStr>(&self, var: &str) -> Result<Option<T>, EwwError<<T as FromStr>::Err>>
    where
        <T as FromStr>::Err: 'static + Error,
    {
        let out = Command::new(&self.binary)
            .arg("get")
            .arg(var)
            .output()
            .map_err(EwwError::Io)?
            .stdout;

        // Whether an error or the actual value, this always returns a valid string
        let out = unsafe { String::from_utf8_unchecked(out) };

        if out == format!("Variable not found \"{var}\"") {
            warn!("Eww variable \"{var}\" not found");
            Ok(None)
        } else {
            let parsed = T::from_str(&out).map_err(EwwError::ParseVar)?;

            info!("Retrieved eww variable \"{var}\"'s value: {}", &out);
            Ok(Some(parsed))
        }
    }
}

struct Daemon {
    sway_socket: BufReader<UnixStream>,
    eww: Eww,
}

impl Daemon {
    #[tracing::instrument]
    pub async fn new(socket_path: &str, eww: Eww) -> Result<Self, DaemonError> {
        Ok(Self {
            sway_socket: BufReader::new(UnixStream::connect(socket_path).await?),
            eww,
        })
    }

    async fn read_response(&mut self) -> Result<Message, ResponseDeserializeError> {
        Message::from_read(&mut self.sway_socket).await
    }

    async fn read_event(&mut self) -> Result<Event, ResponseDeserializeError> {
        Event::from_read(&mut self.sway_socket).await
    }

    async fn request(
        &mut self,
        request_type: MessageType,
        payload: Option<impl AsRef<str>>,
    ) -> Result<(), RequestError> {
        let payload = payload.map_or(String::new(), |s| s.as_ref().to_owned());
        let payload_len = payload.len() as u32;

        // Build the message
        let msg = I3_MAGIC_STRING
            .into_iter()
            .chain(payload_len.to_ne_bytes().into_iter())
            .chain(request_type.bytes())
            .chain(payload.bytes())
            .collect::<Vec<_>>();

        // Send the message to the socket
        self.sway_socket.write_all(&msg).await?;

        let msg = match self.read_response().await {
            Ok(msg) => msg,
            Err(e) => {
                warn!("Error while reading response. It will not be handled: {e}");
                return Err(e.into());
            }
        };

        info!("Received response of type {:?}", msg.message_type);
        trace!("Event Payload: {}", &msg.payload);

        self.handle_response(msg.message_type, &msg.payload)?;

        Ok(())
    }

    #[tracing::instrument(skip_all,fields(payload_type))]
    fn handle_response(
        &self,
        payload_type: MessageType,
        payload: impl AsRef<str>,
    ) -> Result<(), RequestError> {
        let payload = payload.as_ref();

        trace!(payload = %AsRef::<str>::as_ref(payload), "handling response");
        match payload_type {
            MessageType::GetWorkspaces => {
                let workspaces = {
                    let workspaces: Vec<Workspace> =
                        serde_json::from_str(payload).map_err(RequestError::Deserialize)?;
                    workspaces
                        .into_iter()
                        // All workspaces we can get from the get_workspace command are active workspaces
                        .map(|workspace| WorkspaceInfo {
                            name: workspace.name,
                            num: workspace.num,
                            active: true,
                            focused: workspace.focused,
                            urgent: workspace.urgent,
                            visible: workspace.visible.unwrap(),
                        })
                        .map(|workspace| (workspace.num, workspace))
                        .collect::<HashMap<_, _>>()
                };

                debug!(?workspaces);

                // The remaining workspaces are filled in with default-constructed ones
                let workspace_infos = (1..=8)
                    .map(|i| {
                        workspaces
                            .get(&i)
                            .cloned()
                            .unwrap_or_else(|| WorkspaceInfo::new(&i.to_string(), i))
                    })
                    .collect::<Vec<_>>();

                let workspace_info_json =
                    serde_json::to_string(&workspace_infos).map_err(RequestError::Serialize)?;

                self.eww
                    .set_var("ws_info", &workspace_info_json)
                    .map_err(|e| e.boxed())?;
            }
            MessageType::Subscribe => {
                use serde::Deserialize;

                #[derive(Deserialize, Debug, Clone, Copy)]
                struct SubscribeResponse {
                    pub success: bool,
                }
                let response: SubscribeResponse =
                    serde_json::from_str(payload).map_err(RequestError::Deserialize)?;
                if response.success {
                    info!("Successfully subscribed to sway events");
                } else {
                    return Err(RequestError::UnsuccessfulSubscription);
                }
            }
            _ => {
                trace!("{payload_type:?} payload: {payload}")
            }
        }

        Ok(())
    }

    async fn subscribe_event_loop(&mut self, events: &str) -> Result<(), EventLoopError> {
        info!("Starting event loop");

        // Subscribe to Window and Workspace events
        self.request(MessageType::Subscribe, Some(events)).await?;

        loop {
            let event = self.read_event().await?;

            info!("Received event of type {:?}", event.event_type);
            trace!("Message Payload: {}", &event.payload);

            let shutdown = match self.handle_event(event.event_type, event.payload).await {
                Ok(b) => b,
                Err(e) => {
                    warn!("Error occurred during event handling: {e}");
                    continue;
                }
            };

            if shutdown {
                break;
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all,fields(?event_type))]
    async fn handle_event(
        &mut self,
        event_type: EventType,
        payload: impl AsRef<str>,
    ) -> Result<bool, EventError> {
        let payload = payload.as_ref();

        match event_type {
            EventType::Window => {
                let response: WindowEvent = serde_json::from_str(payload)?;
                if let Some(name) = response.container.name {
                    self.eww
                        .set_var("active_window", &name)
                        .map_err(|e| e.boxed())?;
                }
            }
            EventType::Workspace => {
                // We request this, to update our workspace data
                self.request(MessageType::GetWorkspaces, None::<String>)
                    .await?;
                //self.request(MessageType::SendTick, None::<String>).await?;
            }
            EventType::Shutdown => {
                info!("Shutdown event received. Shutting down");
                // We want to shutdown this service too if the IPC is shutting down
                return Ok(true);
            }
            EventType::Mode => {
                let mode = serde_json::from_str::<ModeEvent>(payload)?.change;
                match &mode[..] {
                    "default" => {
                        self.eww
                            .set_var("binding_active", &false)
                            .map_err(|e| e.boxed())?;
                    }
                    _ => {
                        self.eww
                            .set_var("binding_mode", &mode)
                            .map_err(|e| e.boxed())?;
                        self.eww
                            .set_var("binding_active", &true)
                            .map_err(|e| e.boxed())?;
                    }
                }
            }
            _ => {
                trace!("Received {event_type:?} event with payload: {payload}")
            }
        }

        Ok(false)
    }
}
