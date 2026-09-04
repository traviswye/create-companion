//! Local IPC for the configuration UI: a named pipe (`\\.\pipe\NayaCompanion`
//! on Windows, a Unix socket elsewhere) that broadcasts newline-delimited JSON.
//!
//! The engine only *talks*; the UI reads and writes the config file itself and
//! the engine's file watcher picks changes up. Messages:
//!
//! ```json
//! {"type":"hello","version":"0.1.0","config":"C:\\...\\config.toml"}
//! {"type":"status","profile":"Browser","paused":false}
//! {"type":"event","event":"TUNE_CW","profile":"Browser","action":"Ctrl+Tab","repeat":1}
//! {"type":"config_applied"}
//! {"type":"config_rejected","error":"..."}
//! ```

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, ToNsName};
use serde::Serialize;
use std::io::Write;
use std::sync::{Arc, Mutex};

pub const SOCKET_NAME: &str = "NayaCompanion.sock";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcMessage {
    Hello {
        version: String,
        config: String,
    },
    Status {
        profile: String,
        paused: bool,
    },
    Event {
        event: String,
        profile: String,
        action: String,
        repeat: u32,
    },
    ConfigApplied,
    ConfigRejected {
        error: String,
    },
}

type Clients = Arc<Mutex<Vec<Sender<String>>>>;
/// The most recent `status` line, replayed to every new client after `hello`.
type LastStatus = Arc<Mutex<Option<String>>>;

/// Start the server. Every message received on `rx` is fanned out to all
/// connected clients; `hello` is sent to each client on connect.
pub fn start(rx: Receiver<IpcMessage>, hello: IpcMessage) -> Result<()> {
    let name = SOCKET_NAME
        .to_ns_name::<GenericNamespaced>()
        .context("socket name")?;
    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()
        .context("creating IPC listener")?;
    let clients: Clients = Arc::new(Mutex::new(Vec::new()));
    let hello_line = serde_json::to_string(&hello)? + "\n";
    let last_status: LastStatus = Arc::new(Mutex::new(None));

    // Accept loop: one writer thread per client.
    {
        let clients = Arc::clone(&clients);
        let last_status = Arc::clone(&last_status);
        std::thread::Builder::new()
            .name("naya-ipc-accept".into())
            .spawn(move || {
                for conn in listener.incoming() {
                    let mut stream = match conn {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::debug!("ipc accept error: {e}");
                            continue;
                        }
                    };
                    if stream.write_all(hello_line.as_bytes()).is_err() {
                        continue;
                    }
                    let replay = last_status.lock().ok().and_then(|s| s.clone());
                    if let Some(line) = replay {
                        if stream.write_all(line.as_bytes()).is_err() {
                            continue;
                        }
                    }
                    let (tx, rx) = crossbeam_channel::unbounded::<String>();
                    if let Ok(mut c) = clients.lock() {
                        c.push(tx);
                    }
                    std::thread::Builder::new()
                        .name("naya-ipc-client".into())
                        .spawn(move || {
                            for line in rx.iter() {
                                if stream.write_all(line.as_bytes()).is_err() {
                                    break;
                                }
                            }
                        })
                        .ok();
                    tracing::debug!("ipc client connected");
                }
            })
            .context("spawning IPC accept thread")?;
    }

    // Fan-out loop.
    std::thread::Builder::new()
        .name("naya-ipc-broadcast".into())
        .spawn(move || {
            for msg in rx.iter() {
                let Ok(mut line) = serde_json::to_string(&msg) else {
                    continue;
                };
                line.push('\n');
                if matches!(msg, IpcMessage::Status { .. }) {
                    if let Ok(mut s) = last_status.lock() {
                        *s = Some(line.clone());
                    }
                }
                if let Ok(mut c) = clients.lock() {
                    c.retain(|tx| tx.send(line.clone()).is_ok());
                }
            }
        })
        .context("spawning IPC broadcast thread")?;
    Ok(())
}
