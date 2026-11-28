//! MPRIS exports commands through the same application-session interface.

use crate::{Command, Repeat, State};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc::Sender},
};
use zbus::{
    blocking::{Connection, connection::Builder},
    zvariant::{ObjectPath, Value},
};

const PATH: &str = "/org/mpris/MediaPlayer2";
fn status(state: &State) -> &'static str {
    if state.playing {
        "Playing"
    } else if state.stopped {
        "Stopped"
    } else {
        "Paused"
    }
}
pub const NAME: &str = "org.mpris.MediaPlayer2.omusic";

#[derive(Clone)]
struct Player {
    commands: Sender<Command>,
    state: Arc<Mutex<State>>,
}

impl Player {
    fn state(&self) -> State {
        self.state.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

fn metadata(state: &State) -> HashMap<String, Value<'static>> {
    let mut values = HashMap::new();
    if let Some(track) = state.current_track() {
        if let Ok(path) = ObjectPath::try_from(format!("/org/omarchy/omusic/{}", track.id)) {
            values.insert("mpris:trackid".into(), Value::from(path));
        }
        values.insert("xesam:title".into(), Value::from(track.title.clone()));
        values.insert("xesam:album".into(), Value::from(track.album.clone()));
        values.insert(
            "xesam:artist".into(),
            Value::from(vec![track.artist.clone()]),
        );
        values.insert(
            "mpris:length".into(),
            Value::from((state.duration * 1_000_000.0) as i64),
        );
        if let Ok(url) = url::Url::from_file_path(&track.path) {
            values.insert("xesam:url".into(), Value::from(url.to_string()));
        }
    }
    values
}

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl Player {
    fn raise(&self) {
        self.send(Command::Raise);
    }
    fn quit(&self) {
        self.send(Command::Quit);
    }
    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_raise(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn identity(&self) -> &str {
        "Omusic"
    }
    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "org.omarchy.omusic"
    }
    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec![]
    }
    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec!["audio/mpeg".into()]
    }
}

#[derive(Clone)]
struct Controls(Player);

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl Controls {
    fn next(&self) {
        self.0.send(Command::Next);
    }
    fn previous(&self) {
        self.0.send(Command::Previous);
    }
    fn pause(&self) {
        self.0.send(Command::Pause);
    }
    fn play(&self) {
        self.0.send(Command::Play);
    }
    fn play_pause(&self) {
        self.0.send(Command::TogglePlay);
    }
    fn stop(&self) {
        self.0.send(Command::Stop);
    }
    fn seek(&self, offset: i64) {
        self.0.send(Command::Seek(
            self.0.state().position + offset as f64 / 1_000_000.0,
        ));
    }
    fn set_position(&self, track_id: ObjectPath<'_>, position: i64) {
        let state = self.0.state();
        if state
            .current_track()
            .is_some_and(|t| track_id.as_str() == format!("/org/omarchy/omusic/{}", t.id))
        {
            self.0.send(Command::Seek(position as f64 / 1_000_000.0));
        }
    }
    #[zbus(property)]
    fn playback_status(&self) -> &str {
        status(&self.0.state())
    }
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, Value<'static>> {
        metadata(&self.0.state())
    }
    #[zbus(property)]
    fn position(&self) -> i64 {
        (self.0.state().position * 1_000_000.0) as i64
    }
    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.0.state().volume / 100.0
    }
    #[zbus(property)]
    fn set_volume(&self, volume: f64) {
        self.0.send(Command::Volume(volume * 100.0));
    }
    #[zbus(property)]
    fn shuffle(&self) -> bool {
        self.0.state().shuffle
    }
    #[zbus(property)]
    fn set_shuffle(&self, shuffle: bool) {
        self.0.send(Command::Shuffle(shuffle));
    }
    #[zbus(property)]
    fn loop_status(&self) -> &str {
        match self.0.state().repeat {
            Repeat::Off => "None",
            Repeat::Track => "Track",
            Repeat::Queue => "Playlist",
        }
    }
    #[zbus(property)]
    fn set_loop_status(&self, value: &str) {
        self.0.send(Command::Repeat(match value {
            "Track" => Repeat::Track,
            "Playlist" => Repeat::Queue,
            _ => Repeat::Off,
        }));
    }
    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn set_rate(&self, _rate: f64) {}
    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        !self.0.state().queue.is_empty()
    }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        !self.0.state().queue.is_empty()
    }
    #[zbus(property)]
    fn can_play(&self) -> bool {
        !self.0.state().queue.is_empty()
    }
    #[zbus(property)]
    fn can_pause(&self) -> bool {
        self.0.state().current_track().is_some()
    }
    #[zbus(property)]
    fn can_seek(&self) -> bool {
        self.0.state().current_track().is_some()
    }
    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
}

pub struct Desktop {
    connection: Connection,
    player: Player,
}

impl Desktop {
    pub fn connect(commands: Sender<Command>) -> zbus::Result<Self> {
        let player = Player {
            commands,
            state: Arc::new(Mutex::new(State::default())),
        };
        let connection = Builder::session()?
            .name(NAME)?
            .serve_at(PATH, player.clone())?
            .serve_at(PATH, Controls(player.clone()))?
            .build()?;
        Ok(Self { connection, player })
    }
    pub fn publish(&self, state: &State) -> zbus::Result<()> {
        *self.player.state.lock().unwrap_or_else(|p| p.into_inner()) = state.clone();
        let mut changed: HashMap<&str, Value<'_>> = HashMap::new();
        changed.insert("PlaybackStatus", Value::from(status(state)));
        changed.insert("Metadata", Value::from(metadata(state)));
        changed.insert("Volume", Value::from(state.volume / 100.0));
        changed.insert("Shuffle", Value::from(state.shuffle));
        changed.insert(
            "LoopStatus",
            Value::from(match state.repeat {
                Repeat::Off => "None",
                Repeat::Track => "Track",
                Repeat::Queue => "Playlist",
            }),
        );
        self.connection.emit_signal(
            None::<&str>,
            PATH,
            "org.freedesktop.DBus.Properties",
            "PropertiesChanged",
            &("org.mpris.MediaPlayer2.Player", changed, Vec::<&str>::new()),
        )
    }
    pub fn seeked(&self, seconds: f64) -> zbus::Result<()> {
        self.connection.emit_signal(
            None::<&str>,
            PATH,
            "org.mpris.MediaPlayer2.Player",
            "Seeked",
            &((seconds * 1_000_000.0) as i64,),
        )
    }
}

pub fn raise_existing() {
    if let Ok(connection) = Connection::session() {
        let _ = connection.call_method(
            Some(NAME),
            PATH,
            Some("org.mpris.MediaPlayer2"),
            "Raise",
            &(),
        );
    }
}
