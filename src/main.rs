mod theme;
mod ui;

use anyhow::{Context, Result};
use crossterm::event::{self, Event as TerminalEvent, KeyEventKind};
use fs2::FileExt;
use omusic::{Command, Effect, Session, desktop, playback::Mpv};
use serde::Deserialize;
use std::{
    fs::{self, OpenOptions},
    io::IsTerminal,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[derive(Default, Deserialize)]
struct Config {
    #[serde(default)]
    roots: Vec<PathBuf>,
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
fn xdg(name: &str, fallback: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(fallback))
}
fn private_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
fn default_music() -> PathBuf {
    let file = xdg("XDG_CONFIG_HOME", ".config").join("user-dirs.dirs");
    if let Ok(text) = fs::read_to_string(file)
        && let Some(line) = text.lines().find(|line| line.starts_with("XDG_MUSIC_DIR="))
    {
        let value = line
            .trim_start_matches("XDG_MUSIC_DIR=")
            .trim_matches('"')
            .replace("$HOME", &home().to_string_lossy());
        return PathBuf::from(value);
    }
    home().join("Music")
}

fn focus_window() {
    let _ = std::process::Command::new("hyprctl")
        .args(["dispatch", "focuswindow", "class:org.omarchy.omusic"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

fn apply_effects(
    session: &mut Session,
    mpv: &mut Mpv,
    effects: Vec<Effect>,
    desktop: Option<&desktop::Desktop>,
) -> Result<()> {
    for effect in effects {
        if matches!(effect, Effect::Raise) {
            focus_window();
            continue;
        }
        if let Effect::Seek(position) = &effect
            && let Some(desktop) = desktop
        {
            let _ = desktop.seeked(*position);
        }
        mpv.apply(effect, session.state().playing)?;
    }
    Ok(())
}

fn run() -> Result<()> {
    let mut roots = vec![];
    let mut silent = false;
    let mut no_mpris = false;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!(
                    "Omusic — album-first terminal MP3 player\n\nUsage: omusic [--root DIRECTORY]... [--audio-null] [--no-mpris]\n\n--root DIRECTORY  Scan a collection folder (repeatable; remembered)\n--audio-null      Decode without audible output, for integration checks\n--no-mpris        Run without exporting desktop media controls\n--version        Print version\n\nInside: ? help · / search · f add folder · R rescan · q quit and stop"
                );
                return Ok(());
            }
            Some("--version" | "-V") => {
                println!("omusic {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Some("--root") => roots.push(PathBuf::from(
                args.next().context("--root requires a directory")?,
            )),
            Some("--audio-null") => silent = true,
            Some("--no-mpris") => no_mpris = true,
            _ => anyhow::bail!("Unknown argument: {} (try --help)", arg.to_string_lossy()),
        }
    }
    let directory = xdg("XDG_STATE_HOME", ".local/state").join("omusic");
    private_directory(&directory)?;
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| directory.clone())
        .join("omusic");
    private_directory(&runtime)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(runtime.join("session.lock"))?;
    if lock.try_lock_exclusive().is_err() {
        desktop::raise_existing();
        focus_window();
        println!("Omusic is already running; focused the existing session where available.");
        return Ok(());
    }
    anyhow::ensure!(
        std::io::stdin().is_terminal(),
        "Omusic needs an interactive terminal (try --help)"
    );
    let config_path = xdg("XDG_CONFIG_HOME", ".config").join("omusic/config.toml");
    let config: Config = match fs::read_to_string(&config_path) {
        Ok(text) => toml::from_str(&text).context("Read Omusic configuration")?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => return Err(e.into()),
    };
    let mut session = Session::open(&directory.join("collection.sqlite3"))?;
    if roots.is_empty() {
        roots = if !config.roots.is_empty() {
            config.roots
        } else if !session.state().roots.is_empty() {
            session.state().roots.clone()
        } else {
            vec![default_music()]
        };
    }
    roots = roots
        .into_iter()
        .map(|root| {
            if root.is_absolute() {
                root
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| home())
                    .join(root)
            }
        })
        .collect();
    session.rescan(roots)?;
    let socket = runtime.join("mpv.sock");
    let _ = fs::remove_file(&socket);
    let mut mpv = Mpv::spawn(&socket, silent)?;
    mpv.apply(Effect::Volume(session.state().volume), false)?;
    let (tx, commands) = mpsc::channel();
    let desktop = if no_mpris {
        None
    } else {
        match desktop::Desktop::connect(tx) {
            Ok(d) => Some(d),
            Err(error) => {
                session.dispatch(Command::Report(format!(
                    "Desktop media controls unavailable: {error}"
                )))?;
                None
            }
        }
    };
    let terminated = Arc::new(AtomicBool::new(false));
    for signal in [
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
        signal_hook::consts::SIGINT,
    ] {
        signal_hook::flag::register(signal, Arc::clone(&terminated))?;
    }
    let theme_path = std::env::var_os("OMUSIC_THEME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            xdg("XDG_STATE_HOME", ".local/state").join("omarchy/current/theme/colors.toml")
        });
    let mut theme = theme::Theme::read(&theme_path);
    let mut theme_at = Instant::now();
    let mut ui = ui::Ui::new(session.state().browse.clone());
    let mut terminal = ratatui::try_init()?;
    let _terminal_guard = TerminalGuard;
    let mut last_publish = Instant::now() - Duration::from_secs(1);
    let result = (|| -> Result<()> {
        while !session.state().quitting && !terminated.load(Ordering::Relaxed) {
            if let Err(error) = session.poll() {
                session.dispatch(Command::Report(error.to_string()))?;
            }
            ui.sync(&session);
            if theme_at.elapsed() > Duration::from_secs(1) {
                theme = theme::Theme::read(&theme_path);
                theme_at = Instant::now();
            }
            for command in commands.try_iter() {
                let effects = session.dispatch(command)?;
                apply_effects(&mut session, &mut mpv, effects, desktop.as_ref())?;
            }
            let events = mpv.poll()?;
            for event in events {
                let effects = session.event(event)?;
                apply_effects(&mut session, &mut mpv, effects, desktop.as_ref())?;
            }
            if last_publish.elapsed() > Duration::from_millis(500) {
                if let Some(desktop) = &desktop {
                    let _ = desktop.publish(session.state());
                }
                last_publish = Instant::now();
            }
            terminal.draw(|frame| ui.draw(frame, &session, theme))?;
            if event::poll(Duration::from_millis(100))?
                && let TerminalEvent::Key(key) = event::read()?
                && key.kind != KeyEventKind::Release
            {
                match ui.key(&mut session, key) {
                    Ok(effects) => {
                        apply_effects(&mut session, &mut mpv, effects, desktop.as_ref())?
                    }
                    Err(error) => {
                        session.dispatch(Command::Report(error.to_string()))?;
                    }
                }
                if let Some(desktop) = &desktop {
                    let _ = desktop.publish(session.state());
                }
            }
        }
        Ok(())
    })();
    let saved = session.save();
    drop(mpv);
    let _ = fs::remove_file(socket);
    result?;
    saved?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("omusic: {error:#}");
        std::process::exit(1);
    }
}
