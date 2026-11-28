//! Application-session interface shared by terminal and desktop callers.

use rusqlite::OptionalExtension;
use std::path::Path;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
};
mod collection;
pub mod desktop;
pub mod playback;
mod session;
pub use collection::{Album, Collection, Track};
pub use session::{Browse, Command, Effect, Event, Repeat, SearchResult, State};

/// The collection and remembered listening session.
pub struct Session {
    database: rusqlite::Connection,
    collection: Collection,
    scan: Option<Receiver<collection::Scan>>,
    pending_scan: bool,
    state: State,
    failed: std::collections::BTreeSet<usize>,
    visited: std::collections::BTreeSet<usize>,
    rng: rand::rngs::StdRng,
}

impl Session {
    /// Open a collection/session database without starting playback.
    pub fn open(database: &Path) -> anyhow::Result<Self> {
        let database = rusqlite::Connection::open(database)?;
        database.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (name TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )?;
        let value: Option<String> = database
            .query_row(
                "SELECT value FROM snapshots WHERE name='collection'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let collection = value
            .map(|v| serde_json::from_str(&v))
            .transpose()?
            .unwrap_or_default();
        let value: Option<String> = database
            .query_row(
                "SELECT value FROM snapshots WHERE name='session'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let mut state: State = value
            .map(|v| serde_json::from_str(&v))
            .transpose()?
            .unwrap_or_default();
        state.playing = false;
        state.stopped = true;
        Ok(Self {
            database,
            collection,
            scan: None,
            pending_scan: false,
            state,
            failed: Default::default(),
            visited: Default::default(),
            rng: <rand::rngs::StdRng as rand::SeedableRng>::from_os_rng(),
        })
    }

    /// Flush the remembered session to durable storage.
    pub fn save(&self) -> anyhow::Result<()> {
        self.database.execute(
            "INSERT OR REPLACE INTO snapshots VALUES ('session',?1)",
            [serde_json::to_string(&self.state)?],
        )?;
        Ok(())
    }

    pub fn collection(&self) -> &Collection {
        &self.collection
    }
    pub fn is_scanning(&self) -> bool {
        self.scan.is_some()
    }
    pub fn rescan(&mut self, roots: Vec<PathBuf>) -> anyhow::Result<()> {
        self.state.roots = roots.clone();
        self.save()?;
        if self.is_scanning() {
            self.pending_scan = true;
            return Ok(());
        }
        self.pending_scan = false;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(collection::discover(roots));
        });
        self.scan = Some(rx);
        Ok(())
    }
    pub fn poll(&mut self) -> anyhow::Result<bool> {
        let Some(receiver) = &self.scan else {
            return Ok(false);
        };
        let scan = match receiver.try_recv() {
            Ok(scan) => scan,
            Err(TryRecvError::Empty) => return Ok(false),
            Err(error) => {
                self.scan = None;
                return Err(error.into());
            }
        };
        self.scan = None;
        self.collection
            .tracks
            .retain(|t| !scan.successful.iter().any(|root| t.path.starts_with(root)));
        for track in scan.tracks {
            self.collection.tracks.retain(|t| t.id != track.id);
            self.collection.tracks.push(track);
        }
        self.collection.rebuild();
        self.state.errors.extend(scan.errors);
        if self.state.errors.len() > 200 {
            self.state.errors.drain(..self.state.errors.len() - 200);
        }
        self.database.execute(
            "INSERT OR REPLACE INTO snapshots VALUES ('collection',?1)",
            [serde_json::to_string(&self.collection)?],
        )?;
        self.save()?;
        if self.pending_scan {
            self.rescan(self.state.roots.clone())?;
        }
        Ok(true)
    }
}
