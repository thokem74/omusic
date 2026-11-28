use crate::{Session, Track};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    #[default]
    Off,
    Track,
    Queue,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Browse {
    pub artist: String,
    pub album: String,
    pub track: usize,
    pub pane: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub roots: Vec<PathBuf>,
    pub browse: Browse,
    pub queue: Vec<Track>,
    pub current: Option<usize>,
    pub playing: bool,
    #[serde(default)]
    pub stopped: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f64,
    pub errors: Vec<String>,
    #[serde(default)]
    pub repeat: Repeat,
    #[serde(default)]
    pub shuffle: bool,
    #[serde(skip)]
    pub quitting: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            roots: vec![],
            browse: Browse::default(),
            queue: vec![],
            current: None,
            playing: false,
            stopped: true,
            position: 0.0,
            duration: 0.0,
            volume: 70.0,
            errors: vec![],
            repeat: Repeat::Off,
            shuffle: false,
            quitting: false,
        }
    }
}

impl State {
    pub fn current_track(&self) -> Option<&Track> {
        self.current.and_then(|i| self.queue.get(i))
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    PlayAlbum { id: String, start: usize },
    AppendAlbum(String),
    Move { from: usize, to: usize },
    Volume(f64),
    Clear,
    Repeat(Repeat),
    Shuffle(bool),
    PlayTrack(String),
    AppendTrack(String),
    PlayQueue(usize),
    Remove(usize),
    Play,
    Pause,
    TogglePlay,
    Stop,
    Next,
    Previous,
    Seek(f64),
    Browse(Browse),
    Report(String),
    Quit,
    Raise,
}

#[derive(Debug)]
pub enum Event {
    Position { seconds: f64, duration: f64 },
    Failed { id: String, message: String },
    Ended { id: String },
}

#[derive(Debug, Clone)]
pub enum Effect {
    Load { track: Box<Track>, position: f64 },
    Volume(f64),
    Stop,
    Pause(bool),
    Seek(f64),
    Raise,
}

#[derive(Clone, Debug)]
pub enum SearchResult {
    Artist(String),
    Album(String),
    Track(String),
}

impl Session {
    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn dispatch(&mut self, command: Command) -> anyhow::Result<Vec<Effect>> {
        let mut effects = vec![];
        match command {
            Command::Raise => effects.push(Effect::Raise),
            Command::PlayTrack(id) => {
                let track = self
                    .collection
                    .track(&id)
                    .ok_or_else(|| anyhow::anyhow!("Track no longer available"))?;
                let album = self
                    .collection
                    .album(&track.album_id)
                    .ok_or_else(|| anyhow::anyhow!("Album no longer available"))?;
                return self.dispatch(Command::PlayAlbum {
                    id: album.id.clone(),
                    start: album.tracks.iter().position(|t| t == &id).unwrap_or(0),
                });
            }
            Command::AppendTrack(id) => {
                if let Some(track) = self.collection.track(&id) {
                    self.state.queue.push(track.clone());
                }
            }
            Command::PlayQueue(index) => {
                anyhow::ensure!(
                    index < self.state.queue.len(),
                    "Queue selection no longer available"
                );
                self.state.current = Some(index);
                self.state.position = 0.0;
                self.state.playing = true;
                self.state.stopped = false;
                self.failed.clear();
                effects.push(self.load_current());
            }
            Command::Play | Command::TogglePlay if !self.state.playing => {
                if self.state.current.is_none() && !self.state.queue.is_empty() {
                    self.state.current = Some(0);
                }
                if self.state.current_track().is_some() {
                    self.failed.clear();
                    self.state.playing = true;
                    self.state.stopped = false;
                    effects.push(self.load_current());
                }
            }
            Command::Pause | Command::TogglePlay => {
                self.state.playing = false;
                self.state.stopped = false;
                effects.push(Effect::Pause(true));
            }
            Command::Play => {}
            Command::Stop => {
                self.state.playing = false;
                self.state.stopped = true;
                self.state.position = 0.0;
                effects.push(Effect::Stop);
            }
            Command::Next => {
                self.failed.clear();
                effects.extend(self.advance(false));
            }
            Command::Previous => {
                if self.state.position > 3.0 {
                    self.state.position = 0.0;
                    effects.push(Effect::Seek(0.0));
                } else if let Some(i) = self.state.current {
                    self.state.current = Some(i.saturating_sub(1));
                    self.state.position = 0.0;
                    effects.push(self.load_current());
                }
            }
            Command::Seek(seconds) => {
                if seconds.is_finite() {
                    self.state.position = seconds.max(0.0);
                    if self.state.duration > 0.0 {
                        self.state.position = self.state.position.min(self.state.duration);
                    }
                    effects.push(Effect::Seek(self.state.position));
                }
            }
            Command::Browse(browse) => self.state.browse = browse,
            Command::Report(error) => {
                self.state.errors.push(error);
                if self.state.errors.len() > 200 {
                    self.state.errors.remove(0);
                }
            }
            Command::Quit => self.state.quitting = true,
            Command::Remove(index) => {
                if index < self.state.queue.len() {
                    self.state.queue.remove(index);
                    self.failed = self
                        .failed
                        .iter()
                        .filter(|i| **i != index)
                        .map(|i| if *i > index { *i - 1 } else { *i })
                        .collect();
                    self.visited = self
                        .visited
                        .iter()
                        .filter(|i| **i != index)
                        .map(|i| if *i > index { *i - 1 } else { *i })
                        .collect();
                    match self.state.current {
                        Some(i) if i == index => {
                            self.state.position = 0.0;
                            if index < self.state.queue.len() {
                                effects.push(self.load_current());
                            } else {
                                self.state.current = if self.state.queue.is_empty() {
                                    None
                                } else {
                                    Some(self.state.queue.len() - 1)
                                };
                                self.state.playing = false;
                                self.state.stopped = true;
                                effects.push(Effect::Stop);
                            }
                        }
                        Some(i) if index < i => self.state.current = Some(i - 1),
                        _ => {}
                    }
                }
            }
            Command::Shuffle(shuffle) => {
                self.state.shuffle = shuffle;
                self.visited.clear();
                if let Some(i) = self.state.current {
                    self.visited.insert(i);
                }
            }
            Command::Repeat(repeat) => self.state.repeat = repeat,
            Command::PlayAlbum { id, start } => {
                let album = self
                    .collection
                    .album(&id)
                    .ok_or_else(|| anyhow::anyhow!("Album no longer available"))?;
                let queue: Vec<_> = album
                    .tracks
                    .iter()
                    .filter_map(|id| self.collection.track(id).cloned())
                    .collect();
                anyhow::ensure!(start < queue.len(), "Track selection is outside the album");
                self.state.queue = queue;
                self.failed.clear();
                self.visited.clear();
                self.state.current = Some(start);
                self.state.position = 0.0;
                self.state.playing = true;
                self.state.stopped = false;
                effects.push(self.load_current());
            }
            Command::AppendAlbum(id) => {
                if let Some(album) = self.collection.album(&id) {
                    self.state.queue.extend(
                        album
                            .tracks
                            .iter()
                            .filter_map(|id| self.collection.track(id).cloned()),
                    );
                }
            }
            Command::Move { from, to } => {
                if from < self.state.queue.len() && to < self.state.queue.len() {
                    let track = self.state.queue.remove(from);
                    self.state.queue.insert(to, track);
                    let remap = |i| {
                        if i == from {
                            to
                        } else if from < i && i <= to {
                            i - 1
                        } else if to <= i && i < from {
                            i + 1
                        } else {
                            i
                        }
                    };
                    self.state.current = self.state.current.map(remap);
                    self.failed = self.failed.iter().copied().map(remap).collect();
                    self.visited = self.visited.iter().copied().map(remap).collect();
                }
            }
            Command::Volume(volume) => {
                if !volume.is_finite() {
                    return Ok(effects);
                }
                self.state.volume = volume.clamp(0.0, 100.0);
                effects.push(Effect::Volume(self.state.volume));
            }
            Command::Clear => {
                self.state.queue.clear();
                self.state.current = None;
                self.state.position = 0.0;
                self.state.playing = false;
                self.state.stopped = true;
                self.state.duration = 0.0;
                self.failed.clear();
                self.visited.clear();
                effects.push(Effect::Stop);
            }
        }
        self.save()?;
        Ok(effects)
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query = query.to_lowercase();
        let matches = |value: &str| value.to_lowercase().contains(&query);
        self.collection
            .artists()
            .into_iter()
            .filter(|a| matches(a))
            .map(SearchResult::Artist)
            .chain(
                self.collection
                    .albums
                    .iter()
                    .filter(|a| matches(&format!("{} {}", a.title, a.artist)))
                    .map(|a| SearchResult::Album(a.id.clone())),
            )
            .chain(
                self.collection
                    .tracks
                    .iter()
                    .filter(|t| matches(&format!("{} {} {}", t.title, t.artist, t.album)))
                    .map(|t| SearchResult::Track(t.id.clone())),
            )
            .collect()
    }

    pub fn event(&mut self, event: Event) -> anyhow::Result<Vec<Effect>> {
        match event {
            Event::Position { seconds, duration } => {
                self.state.position = seconds.max(0.0);
                self.state.duration = duration.max(0.0);
            }
            Event::Failed { id, message } => {
                if self.state.current_track().is_some_and(|t| t.id == id) {
                    self.state.errors.push(message);
                    self.failed.insert(self.state.current.unwrap());
                    let effects = self.advance(false);
                    self.save()?;
                    return Ok(effects);
                }
            }
            Event::Ended { id } => {
                if self.state.current_track().is_some_and(|t| t.id == id) {
                    self.failed.clear();
                    let effects = self.advance(true);
                    self.save()?;
                    return Ok(effects);
                }
            }
        }
        self.save()?;
        Ok(vec![])
    }

    fn load_current(&mut self) -> Effect {
        let track = self
            .state
            .current_track()
            .expect("current entry checked by caller")
            .clone();
        self.state.duration = track.duration;
        if let Some(i) = self.state.current {
            self.visited.insert(i);
        }
        Effect::Load {
            track: Box::new(track),
            position: self.state.position,
        }
    }

    fn advance(&mut self, natural: bool) -> Vec<Effect> {
        let Some(current) = self.state.current else {
            return vec![];
        };
        let next = if natural && self.state.repeat == Repeat::Track {
            Some(current)
        } else if self.state.shuffle {
            let mut candidates: Vec<_> = (0..self.state.queue.len())
                .filter(|i| !self.visited.contains(i) && !self.failed.contains(i))
                .collect();
            if candidates.is_empty() && self.state.repeat == Repeat::Queue {
                self.visited.clear();
                candidates = (0..self.state.queue.len())
                    .filter(|i| !self.failed.contains(i))
                    .collect();
            }
            if candidates.is_empty() {
                None
            } else {
                Some(candidates[self.rng.random_range(0..candidates.len())])
            }
        } else {
            (current + 1..self.state.queue.len())
                .find(|i| !self.failed.contains(i))
                .or_else(|| {
                    (self.state.repeat == Repeat::Queue)
                        .then(|| (0..self.state.queue.len()).find(|i| !self.failed.contains(i)))
                        .flatten()
                })
        };
        self.state.position = 0.0;
        if let Some(next) = next {
            self.state.current = Some(next);
            vec![self.load_current()]
        } else {
            self.state.playing = false;
            self.state.stopped = true;
            vec![Effect::Stop]
        }
    }

    /// Supply a reproducible shuffle source for deterministic session scenarios.
    pub fn seed_shuffle(&mut self, seed: u64) {
        self.rng = rand::rngs::StdRng::seed_from_u64(seed);
    }
}
