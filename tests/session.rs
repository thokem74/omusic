use id3::{Tag, TagLike, Version};
use omusic::{Command as SessionCommand, Event, Session};
use std::{fs, path::Path, process::Command, sync::OnceLock};

fn mp3(path: &Path, title: &str, artist: &str, album_artist: Option<&str>, disc: u32, track: u32) {
    static AUDIO: OnceLock<Vec<u8>> = OnceLock::new();
    let audio = AUDIO.get_or_init(|| {
        let output = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-f",
                "mp3",
                "pipe:1",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "ffmpeg must generate test MP3 fixtures"
        );
        output.stdout
    });
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, audio).unwrap();
    let mut tag = Tag::new();
    tag.set_title(title);
    tag.set_artist(artist);
    tag.set_album("Night Signals");
    if let Some(credit) = album_artist {
        tag.set_album_artist(credit);
    }
    tag.set_disc(disc);
    tag.set_track(track);
    tag.write_to_path(path, Version::Id3v24).unwrap();
}

fn scan(session: &mut Session, root: &Path) {
    session.rescan(vec![root.to_path_buf()]).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while session.is_scanning() {
        session.poll().unwrap();
        assert!(std::time::Instant::now() < deadline, "scan timed out");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn a_new_session_opens_without_music_or_playback() {
    let scratch = tempfile::tempdir().unwrap();
    Session::open(&scratch.path().join("collection.sqlite3")).unwrap();
}

#[test]
fn multidisc_compilations_keep_track_credits_and_separate_editions() {
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path().join("music");
    mp3(
        &root.join("Original/CD 2/a.mp3"),
        "Last",
        "Mira",
        Some("Various Artists"),
        2,
        1,
    );
    mp3(
        &root.join("Original/CD 1/b.mp3"),
        "First",
        "Aster",
        Some("Various Artists"),
        1,
        1,
    );
    mp3(
        &root.join("Remaster/a.mp3"),
        "First",
        "Aster",
        Some("Various Artists"),
        1,
        1,
    );
    let original = fs::read(root.join("Original/CD 1/b.mp3")).unwrap();
    let mut session = Session::open(&scratch.path().join("db")).unwrap();
    scan(&mut session, &root);
    assert_eq!(session.collection().albums.len(), 2);
    let album = session
        .collection()
        .albums
        .iter()
        .find(|a| a.tracks.len() == 2)
        .unwrap();
    assert_eq!(album.artist, "Various Artists");
    let titles: Vec<_> = album
        .tracks
        .iter()
        .map(|id| session.collection().track(id).unwrap().title.as_str())
        .collect();
    assert_eq!(titles, ["First", "Last"]);
    assert_eq!(session.collection().appears_on("Mira").len(), 1);
    assert_eq!(
        fs::read(root.join("Original/CD 1/b.mp3")).unwrap(),
        original
    );
}

#[test]
fn queue_edits_and_playback_position_survive_a_stopped_restore() {
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path().join("music");
    mp3(
        &root.join("Album/1.mp3"),
        "First",
        "Aster",
        Some("Aster"),
        1,
        1,
    );
    mp3(
        &root.join("Album/2.mp3"),
        "Last",
        "Aster",
        Some("Aster"),
        1,
        2,
    );
    let database = scratch.path().join("db");
    let mut session = Session::open(&database).unwrap();
    scan(&mut session, &root);
    let album = session.collection().albums[0].id.clone();
    session
        .dispatch(SessionCommand::PlayAlbum {
            id: album.clone(),
            start: 1,
        })
        .unwrap();
    session
        .dispatch(SessionCommand::AppendAlbum(album))
        .unwrap();
    session
        .dispatch(SessionCommand::Move { from: 1, to: 0 })
        .unwrap();
    session
        .event(Event::Position {
            seconds: 14.0,
            duration: 60.0,
        })
        .unwrap();
    session.dispatch(SessionCommand::Volume(42.0)).unwrap();
    assert_eq!(session.state().current_track().unwrap().title, "Last");
    assert_eq!(session.state().queue.len(), 4);
    session.save().unwrap();
    drop(session);
    let mut restored = Session::open(&database).unwrap();
    assert!(!restored.state().playing);
    assert_eq!(restored.state().position, 14.0);
    assert_eq!(restored.state().volume, 42.0);
    assert_eq!(restored.state().current_track().unwrap().title, "Last");
    restored.dispatch(SessionCommand::Clear).unwrap();
    assert!(restored.state().queue.is_empty());
    assert!(!restored.state().playing);
}

#[test]
fn failed_entries_exhaust_the_queue_even_when_repeat_is_enabled() {
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path().join("music");
    mp3(
        &root.join("Album/1.mp3"),
        "First",
        "Aster",
        Some("Aster"),
        1,
        1,
    );
    mp3(
        &root.join("Album/2.mp3"),
        "Last",
        "Aster",
        Some("Aster"),
        1,
        2,
    );
    let mut session = Session::open(&scratch.path().join("db")).unwrap();
    scan(&mut session, &root);
    let album = session.collection().albums[0].id.clone();
    session
        .dispatch(SessionCommand::PlayAlbum {
            id: album,
            start: 0,
        })
        .unwrap();
    session
        .dispatch(SessionCommand::Repeat(omusic::Repeat::Queue))
        .unwrap();
    for title in ["First", "Last"] {
        assert_eq!(session.state().current_track().unwrap().title, title);
        let id = session.state().current_track().unwrap().id.clone();
        session
            .event(Event::Failed {
                id,
                message: "unreadable file".into(),
            })
            .unwrap();
    }
    assert!(!session.state().playing);
    assert_eq!(session.state().errors.len(), 2);
}

#[test]
fn shuffle_visits_each_entry_once_without_reordering_the_queue() {
    // The application seam accepts a controlled random source for this scenario.
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path().join("music");
    for (number, title) in [(1, "First"), (2, "Middle"), (3, "Last")] {
        mp3(
            &root.join(format!("Album/{number}.mp3")),
            title,
            "Aster",
            Some("Aster"),
            1,
            number,
        );
    }
    let mut session = Session::open(&scratch.path().join("db")).unwrap();
    session.seed_shuffle(7);
    scan(&mut session, &root);
    session
        .dispatch(SessionCommand::PlayAlbum {
            id: session.collection().albums[0].id.clone(),
            start: 0,
        })
        .unwrap();
    let original: Vec<_> = session.state().queue.iter().map(|t| t.id.clone()).collect();
    session.dispatch(SessionCommand::Shuffle(true)).unwrap();
    let mut visited = std::collections::BTreeSet::new();
    for _ in 0..3 {
        let id = session.state().current_track().unwrap().id.clone();
        assert!(visited.insert(id.clone()));
        session.event(Event::Ended { id }).unwrap();
    }
    assert!(!session.state().playing);
    assert_eq!(
        session
            .state()
            .queue
            .iter()
            .map(|t| t.id.clone())
            .collect::<Vec<_>>(),
        original
    );
}

#[test]
fn search_playback_and_controls_share_the_session_interface() {
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path().join("music");
    mp3(
        &root.join("Album/1.mp3"),
        "First",
        "Aster",
        Some("Aster"),
        1,
        1,
    );
    mp3(
        &root.join("Album/2.mp3"),
        "Last",
        "Mira",
        Some("Aster"),
        1,
        2,
    );
    let mut session = Session::open(&scratch.path().join("db")).unwrap();
    scan(&mut session, &root);
    let result = session
        .search("Last")
        .into_iter()
        .find_map(|r| {
            if let omusic::SearchResult::Track(id) = r {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    session.dispatch(SessionCommand::PlayTrack(result)).unwrap();
    assert_eq!(session.state().current_track().unwrap().title, "Last");
    assert_eq!(session.state().queue.len(), 2);
    session
        .event(Event::Position {
            seconds: 14.0,
            duration: 60.0,
        })
        .unwrap();
    session.dispatch(SessionCommand::Previous).unwrap();
    assert_eq!(session.state().position, 0.0);
    assert_eq!(session.state().current_track().unwrap().title, "Last");
    session.dispatch(SessionCommand::Previous).unwrap();
    assert_eq!(session.state().current_track().unwrap().title, "First");
    session.dispatch(SessionCommand::Pause).unwrap();
    assert!(!session.state().playing);
    session.dispatch(SessionCommand::Play).unwrap();
    assert!(session.state().playing);
    session.dispatch(SessionCommand::Remove(0)).unwrap();
    assert_eq!(session.state().current_track().unwrap().title, "Last");
}

#[test]
fn disconnected_roots_preserve_the_index_and_successful_scans_reconcile_deletions() {
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path().join("music");
    let path = root.join("Album/track.mp3");
    mp3(&path, "First", "Aster", Some("Aster"), 1, 1);
    let mut session = Session::open(&scratch.path().join("db")).unwrap();
    scan(&mut session, &root);
    let id = session.collection().albums[0].id.clone();
    session
        .dispatch(SessionCommand::PlayAlbum { id, start: 0 })
        .unwrap();
    fs::rename(&root, scratch.path().join("disconnected")).unwrap();
    scan(&mut session, &root);
    assert_eq!(session.collection().tracks.len(), 1);
    assert_eq!(session.state().queue.len(), 1);
    fs::rename(scratch.path().join("disconnected"), &root).unwrap();
    fs::remove_file(path).unwrap();
    scan(&mut session, &root);
    assert!(session.collection().tracks.is_empty());
    assert_eq!(session.state().queue.len(), 1);
}

#[test]
fn metadata_fallbacks_contributors_and_overlapping_roots_preserve_source_files() {
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path().join("music");
    for (folder, file, artist, credit) in [
        ("Missing", "1", "Aster", None),
        ("Missing", "2", "Mira", None),
        ("Conflict", "1", "Aster", Some("Aster")),
        ("Conflict", "2", "Mira", Some("Mira")),
        ("Contributors", "1", "Guest\0Other", Some("Aster")),
        ("Band", "1", "AC/DC", Some("AC/DC")),
    ] {
        mp3(
            &root.join(format!("{folder}/{file}.mp3")),
            file,
            artist,
            credit,
            1,
            1,
        );
    }
    let untagged = root.join("Untagged/Filename.mp3");
    mp3(&untagged, "Ignored", "Ignored", None, 1, 1);
    Tag::remove_from_path(&untagged).unwrap();
    let mut session = Session::open(&scratch.path().join("db")).unwrap();
    session
        .rescan(vec![root.clone(), root.join("Missing")])
        .unwrap();
    while session.is_scanning() {
        session.poll().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(session.collection().tracks.len(), 7);
    assert_eq!(
        session
            .collection()
            .albums
            .iter()
            .filter(|a| a.artist == "Various Artists")
            .count(),
        3
    );
    assert_eq!(
        session
            .collection()
            .albums
            .iter()
            .filter(|a| a.warning)
            .count(),
        1
    );
    assert_eq!(session.collection().appears_on("Guest").len(), 1);
    assert!(session.collection().artists().contains(&"AC/DC".into()));
    assert!(!session.collection().artists().contains(&"AC".into()));
    assert!(
        session
            .collection()
            .tracks
            .iter()
            .any(|t| t.title == "Filename" && t.album == "Untagged")
    );
}

#[test]
fn adding_a_root_while_scanning_schedules_the_latest_collection_request() {
    let scratch = tempfile::tempdir().unwrap();
    let a = scratch.path().join("A");
    let b = scratch.path().join("B");
    mp3(
        &a.join("Album/1.mp3"),
        "First",
        "Aster",
        Some("Aster"),
        1,
        1,
    );
    mp3(&b.join("Album/1.mp3"), "Last", "Mira", Some("Mira"), 1, 1);
    let mut session = Session::open(&scratch.path().join("db")).unwrap();
    session.rescan(vec![a.clone()]).unwrap();
    session.rescan(vec![a, b.clone()]).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while session.is_scanning() {
        session.poll().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(session.collection().tracks.len(), 2);
    assert!(session.state().roots.contains(&b));
}

#[test]
fn reordering_a_shuffled_queue_does_not_repeat_already_visited_entries() {
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path().join("music");
    for number in 1..=3 {
        mp3(
            &root.join(format!("Album/{number}.mp3")),
            &number.to_string(),
            "Aster",
            Some("Aster"),
            1,
            number,
        );
    }
    let mut session = Session::open(&scratch.path().join("db")).unwrap();
    session.seed_shuffle(7);
    scan(&mut session, &root);
    session
        .dispatch(SessionCommand::PlayAlbum {
            id: session.collection().albums[0].id.clone(),
            start: 0,
        })
        .unwrap();
    session.dispatch(SessionCommand::Shuffle(true)).unwrap();
    let first = session.state().current_track().unwrap().id.clone();
    session.event(Event::Ended { id: first.clone() }).unwrap();
    session
        .dispatch(SessionCommand::Move { from: 0, to: 2 })
        .unwrap();
    let mut visited = std::collections::BTreeSet::from([first]);
    for _ in 0..2 {
        let id = session.state().current_track().unwrap().id.clone();
        assert!(visited.insert(id.clone()));
        session.event(Event::Ended { id }).unwrap();
    }
    assert!(!session.state().playing);
    assert_eq!(visited.len(), 3);
}
