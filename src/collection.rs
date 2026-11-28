use id3::{Tag, TagLike};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

/// A track retains its source credits independently of the album display credit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub path: PathBuf,
    pub album_id: String,
    pub album: String,
    pub album_artist: String,
    pub title: String,
    pub artist: String,
    pub contributors: Vec<String>,
    pub disc: u32,
    pub number: u32,
    pub duration: f64,
}

/// An album edition, including all its discs and individual artist credits.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub tracks: Vec<String>,
    pub inferred_artist: bool,
    pub warning: bool,
}

/// Cached, read-only music collection.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Collection {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
}

impl Collection {
    pub fn track(&self, id: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }
    pub fn album(&self, id: &str) -> Option<&Album> {
        self.albums.iter().find(|a| a.id == id)
    }
    pub fn artists(&self) -> Vec<String> {
        self.albums
            .iter()
            .map(|a| a.artist.clone())
            .chain(self.tracks.iter().flat_map(|t| t.contributors.clone()))
            .filter(|a| !a.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn appears_on(&self, artist: &str) -> Vec<&Album> {
        self.albums
            .iter()
            .filter(|a| {
                a.artist != artist
                    && a.tracks.iter().any(|id| {
                        self.track(id)
                            .is_some_and(|t| t.contributors.iter().any(|c| c == artist))
                    })
            })
            .collect()
    }
    pub fn albums_for_artist(&self, artist: &str) -> Vec<&Album> {
        self.albums
            .iter()
            .filter(|a| a.artist == artist)
            .chain(self.appears_on(artist))
            .collect()
    }
    pub(crate) fn rebuild(&mut self) {
        let mut groups: BTreeMap<String, Vec<&Track>> = BTreeMap::new();
        for track in &self.tracks {
            groups
                .entry(track.album_id.clone())
                .or_default()
                .push(track);
        }
        self.albums = groups
            .into_iter()
            .map(|(id, mut tracks)| {
                tracks
                    .sort_by(|a, b| (a.disc, a.number, &a.path).cmp(&(b.disc, b.number, &b.path)));
                let credits: BTreeSet<_> = tracks
                    .iter()
                    .map(|t| t.album_artist.as_str())
                    .filter(|s| !s.is_empty())
                    .collect();
                let warning = credits.len() > 1;
                let artist = if credits.len() == 1 {
                    credits.first().copied().unwrap_or_default().to_string()
                } else if credits.is_empty()
                    && tracks
                        .iter()
                        .all(|t| !t.artist.is_empty() && t.artist == tracks[0].artist)
                {
                    tracks[0].artist.clone()
                } else {
                    "Various Artists".into()
                };
                Album {
                    id,
                    title: tracks[0].album.clone(),
                    artist,
                    tracks: tracks.iter().map(|t| t.id.clone()).collect(),
                    inferred_artist: credits.is_empty(),
                    warning,
                }
            })
            .collect();
        self.albums
            .sort_by(|a, b| (&a.artist, &a.title, &a.id).cmp(&(&b.artist, &b.title, &b.id)));
    }
}

pub(crate) struct Scan {
    pub tracks: Vec<Track>,
    pub successful: Vec<PathBuf>,
    pub errors: Vec<String>,
}

fn album_directory(path: &Path) -> &Path {
    let parent = path.parent().unwrap_or(path);
    let name = parent
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let disc = ["disc", "disk", "cd"].iter().any(|prefix| {
        name.strip_prefix(prefix).is_some_and(|rest| {
            let number = rest.trim_matches([' ', '-', '_', '.']);
            !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
        })
    });
    if disc {
        parent.parent().unwrap_or(parent)
    } else {
        parent
    }
}

fn read_track(path: PathBuf) -> anyhow::Result<Track> {
    let metadata = fs::metadata(&path)?;
    let tag = Tag::read_from_path(&path).unwrap_or_default();
    let directory = album_directory(&path);
    let album = tag
        .album()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            directory
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
    let title = tag
        .title()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
    let artist = tag.artist().unwrap_or_default().trim().to_string();
    let contributors = artist
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    Ok(Track {
        id: format!("t{}_{}", metadata.dev(), metadata.ino()),
        album_id: format!("{}\u{1f}{}", directory.display(), album),
        album,
        album_artist: tag.album_artist().unwrap_or_default().trim().to_string(),
        title,
        artist: artist.replace('\0', " · "),
        contributors,
        disc: tag.disc().unwrap_or(1),
        number: tag.track().unwrap_or(0),
        duration: f64::from(tag.duration().unwrap_or(0)) / 1000.0,
        path,
    })
}

pub(crate) fn discover(roots: Vec<PathBuf>) -> Scan {
    let mut scan = Scan {
        tracks: vec![],
        successful: vec![],
        errors: vec![],
    };
    let mut seen = BTreeSet::new();
    for root in roots {
        let root = match fs::canonicalize(&root) {
            Ok(root) if root.is_dir() => root,
            _ => {
                scan.errors
                    .push(format!("Collection folder unavailable: {}", root.display()));
                continue;
            }
        };
        let mut complete = true;
        for entry in walkdir::WalkDir::new(&root).follow_links(false) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    complete = false;
                    scan.errors.push(error.to_string());
                    continue;
                }
            };
            if !entry.file_type().is_file()
                || !entry
                    .path()
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("mp3"))
            {
                continue;
            }
            match fs::canonicalize(entry.path())
                .map_err(anyhow::Error::from)
                .and_then(read_track)
            {
                Ok(track) => {
                    if seen.insert(track.id.clone()) {
                        scan.tracks.push(track);
                    }
                }
                Err(error) => {
                    complete = false;
                    scan.errors
                        .push(format!("{}: {error}", entry.path().display()));
                }
            }
        }
        if complete {
            scan.successful.push(root);
        }
    }
    scan
}
