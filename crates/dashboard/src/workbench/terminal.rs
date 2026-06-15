//! Interactive PTY sessions for the workbench Terminal panel.

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalClientMessage {
    Input { data: String },
    Resize { cols: u16, rows: u16 },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalServerMessage {
    Output { data: String },
    Exit { code: i32 },
    Error { message: String },
}

pub struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl Clone for PtySession {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
        }
    }
}

impl PtySession {
    pub fn spawn(cwd: &Path) -> Result<(Self, mpsc::UnboundedReceiver<TerminalServerMessage>)> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".into()
            } else {
                "/bin/zsh".into()
            }
        });

        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd);
        if !cfg!(windows) {
            cmd.arg("-l");
        }

        let mut child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().context("clone pty reader")?;
        let writer = pair.master.take_writer()?;
        let writer = Arc::new(Mutex::new(writer));

        let (tx, rx) = mpsc::unbounded_channel();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                        let _ = tx.send(TerminalServerMessage::Output { data });
                    }
                    Err(e) => {
                        let _ = tx.send(TerminalServerMessage::Error {
                            message: e.to_string(),
                        });
                        break;
                    }
                }
            }
            let code = child.wait().map(|s| s.exit_code() as i32).unwrap_or(1);
            let _ = tx.send(TerminalServerMessage::Exit { code });
        });

        Ok((Self { writer }, rx))
    }

    pub fn write_input(&self, data: &str) -> Result<()> {
        let mut w = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("pty writer poisoned"))?;
        w.write_all(data.as_bytes())?;
        w.flush()?;
        Ok(())
    }

    pub fn resize(&self, _cols: u16, _rows: u16) -> Result<()> {
        // portable-pty resize requires master handle; best-effort no-op for now.
        Ok(())
    }
}
