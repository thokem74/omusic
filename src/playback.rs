//! Owned mpv process and its asynchronous JSON IPC stream.

use crate::{Effect, Event};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::{net::UnixStream, process::CommandExt},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

pub struct Mpv {
    child: Child,
    socket: UnixStream,
    events: Receiver<Value>,
    track: Option<String>,
    waiting: bool,
    start: f64,
    paused: bool,
    position: f64,
    duration: f64,
}

impl Mpv {
    pub fn spawn(socket_path: &Path, silent: bool) -> Result<Self> {
        let mut command = Command::new("mpv");
        command
            .args([
                "--idle=yes",
                "--no-config",
                "--no-terminal",
                "--vid=no",
                "--audio-display=no",
                "--input-terminal=no",
            ])
            .arg(format!("--input-ipc-server={}", socket_path.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if silent {
            command.arg("--ao=null");
        }
        let parent = std::process::id() as libc::pid_t;
        // SAFETY: only async-signal-safe libc calls run in the forked child. The
        // parent check closes the race between fork and installing PDEATHSIG.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    libc::_exit(1);
                }
                Ok(())
            });
        }
        let mut child = command
            .spawn()
            .context("Start mpv (install the mpv package)")?;
        let deadline = Instant::now() + Duration::from_secs(3);
        let socket = loop {
            match UnixStream::connect(socket_path) {
                Ok(socket) => break socket,
                Err(error) => {
                    if Instant::now() >= deadline || child.try_wait()?.is_some() {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(error).context("Connect to private mpv IPC");
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        };
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;
        let reader = socket.try_clone()?;
        let (tx, events) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(reader).lines() {
                let Ok(line) = line else { break };
                if let Ok(value) = serde_json::from_str(&line)
                    && tx.send(value).is_err()
                {
                    break;
                }
            }
        });
        let mut player = Self {
            child,
            socket,
            events,
            track: None,
            waiting: false,
            start: 0.0,
            paused: false,
            position: 0.0,
            duration: 0.0,
        };
        for (id, name) in [(1, "time-pos"), (2, "duration")] {
            player.send(json!(["observe_property", id, name]))?;
        }
        Ok(player)
    }

    fn send(&mut self, command: Value) -> Result<()> {
        serde_json::to_writer(&mut self.socket, &json!({"command":command}))?;
        self.socket.write_all(b"\n")?;
        Ok(())
    }

    pub fn apply(&mut self, effect: Effect, playing: bool) -> Result<()> {
        match effect {
            Effect::Raise => {}
            Effect::Load { track, position } => {
                self.track = Some(track.id);
                self.waiting = true;
                self.start = position;
                self.paused = !playing;
                self.position = position;
                self.duration = track.duration;
                self.send(json!(["loadfile", track.path, "replace"]))?;
            }
            Effect::Pause(paused) => {
                self.paused = paused;
                self.send(json!(["set_property", "pause", paused]))?;
            }
            Effect::Seek(position) => {
                self.position = position;
                if self.waiting {
                    self.start = position;
                } else {
                    self.send(json!(["seek", position, "absolute+exact"]))?;
                }
            }
            Effect::Volume(volume) => self.send(json!(["set_property", "volume", volume]))?,
            Effect::Stop => {
                self.track = None;
                self.waiting = false;
                self.send(json!(["stop"]))?;
            }
        }
        Ok(())
    }

    pub fn poll(&mut self) -> Result<Vec<Event>> {
        anyhow::ensure!(self.child.try_wait()?.is_none(), "mpv exited unexpectedly");
        let mut events = vec![];
        while let Ok(message) = self.events.try_recv() {
            match message["event"].as_str().unwrap_or_default() {
                "file-loaded" => {
                    self.waiting = false;
                    self.send(json!(["set_property", "pause", self.paused]))?;
                    if self.start > 0.0 {
                        self.send(json!(["seek", self.start, "absolute+exact"]))?;
                    }
                }
                "property-change" if !self.waiting && self.track.is_some() => {
                    if let Some(value) = message["data"].as_f64() {
                        match message["name"].as_str() {
                            Some("time-pos") => self.position = value,
                            Some("duration") => self.duration = value,
                            _ => {}
                        }
                    }
                }
                "end-file" => {
                    if let Some(id) = self.track.clone() {
                        match message["reason"].as_str() {
                            Some("eof") => {
                                events.push(Event::Ended { id });
                                self.track = None;
                            }
                            Some("error") => {
                                events.push(Event::Failed {
                                    id,
                                    message: format!(
                                        "mpv: {}",
                                        message["file_error"]
                                            .as_str()
                                            .unwrap_or("Unable to decode track")
                                    ),
                                });
                                self.track = None;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        if self.track.is_some() && !self.waiting {
            events.push(Event::Position {
                seconds: self.position,
                duration: self.duration,
            });
        }
        Ok(events)
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(std::net::Shutdown::Both);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
