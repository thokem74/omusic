# Omusic

An album-first terminal MP3 player for Omarchy and Linux. The selected Library UX keeps artists, albums, and tracks in three columns with a persistent playback strip. Closing the interface stops playback; reopening restores your position without starting audio.

## Build and run

Install Rust through rustup and the `mpv` package, then:

```sh
cargo build --release --locked
./target/release/omusic --root ~/Music
```

The pinned stable Rust toolchain includes rustfmt and Clippy. The executable uses mpv over a private Unix socket; no mpv scripts or `playerctl` are required. SQLite is bundled at build time.

Use repeated `--root` arguments for multiple collection folders. Roots are remembered. Without explicit roots, Omusic uses configured roots, remembered roots, or your XDG music directory, falling back to `~/Music`. Press **f** to add another folder if the collection is empty or needs another location. Startup scans happen in the background; **R** rescans manually. Omusic only reads your music files.

Optional configuration at `$XDG_CONFIG_HOME/omusic/config.toml` (normally `~/.config/omusic/config.toml`):

```toml
roots = ["/absolute/path/to/music", "/absolute/path/to/more-music"]
```

Use absolute paths in this file; the shell expands `~` in CLI arguments, and the in-app folder entry accepts `~/`. Session and collection data are stored at `$XDG_STATE_HOME/omusic/collection.sqlite3` (normally `~/.local/state/omusic/collection.sqlite3`). The private socket and single-instance lock live under `$XDG_RUNTIME_DIR/omusic`, with the state directory used as fallback. A second launch focuses the existing window when available and never starts competing audio.

## Controls

| Key | Action |
| --- | --- |
| Tab / Shift-Tab, h/l, left/right | Change browser column |
| j/k, up/down | Move selection |
| Enter | Open next column; play selected track within its complete album |
| P | Replace queue and play the selected album from the beginning |
| a | Append selected album in Albums, or selected track in Tracks |
| Space | Play/pause; resume a restored queue |
| n / N | Next / previous; previous restarts after three seconds |
| [ / ] | Seek backward/forward five seconds |
| - / + | Volume down/up |
| z / r | Toggle shuffle / cycle repeat off, track, queue |
| / | Search artists, albums, and tracks; up/down to select, Enter to open/play |
| Q | Open/close the queue |
| d / J / K | Remove / move down / move up in the queue |
| C | Clear the queue and stop (in queue view) |
| f / R | Add collection folder / rescan |
| e | Inspect errors |
| ? / Esc | Help / close the current overlay |
| q / Ctrl-C | Save session, stop playback, and quit |

On narrow terminals, one browser column is shown at a time; the same column-navigation keys remain available.

## Albums and credits

Album artist identifies the album; individual track artists remain visible. Compilations and guest credits do not split an album. Artists have Appears on entries that open the complete album and highlight their contributions. Separate editions in separate folders remain distinct, while numbered `CD`, `Disc`, or `Disk` subfolders are combined. Disc and track numbers determine order, with a stable filename fallback for missing numbers.

When album-artist credits are missing, a shared nonempty track artist is used if all tracks agree; otherwise the display credit is Various Artists. Conflicting album-artist credits produce a warning and Various Artists. Explicit multiple ID3v2.4 artist values are recognized; combined strings such as `A feat. B`, `AC/DC`, or `Earth, Wind & Fire` remain intact and searchable. No credit inference rewrites tags.

Temporarily unavailable roots keep their cached tracks and queue entries. A successful scan of an available root reconciles deleted tracks in the browser. Missing or corrupt queued files are reported and skipped; exhausting playable entries stops even under repeat.

## Desktop integration

Omusic exports `org.mpris.MediaPlayer2.omusic` on the session bus. Media controls use the same session as the terminal. Without a session bus, terminal playback still works; `--no-mpris` disables the export explicitly. An Arch package provides a launcher that follows Omarchy's terminal conventions, with a standard terminal fallback outside Omarchy.

Colors follow the active Omarchy semantic theme and reload while the app runs, including replacement of its theme directory. Standard terminal colors are used when no theme is available. `OMUSIC_THEME=/path/to/colors.toml` can override the source for testing.

For an optional shortcut, point your Omarchy binding at `omusic-launch`; the launcher focuses an existing window when possible. This repository does not modify desktop keybindings automatically.

## Arch package

With Arch's base-devel tools and rustup installed:

```sh
bash scripts/package.sh
```

The local source archive and package appear under `target/package/`. Review and install the resulting package with `sudo pacman -U target/package/omusic-*.pkg.tar.zst`; remove it with `sudo pacman -R omusic`. The build does not install packages or alter your desktop configuration.

## Development and verification

See [the development policy](docs/development.md) for official Rust conventions and required local/CI gates. Integration checks also require Python 3, ffmpeg, mpv, D-Bus tools (`dbus-run-session` and `busctl`), and a working pseudo-terminal. Tests generate temporary MP3 fixtures and a scratch database; they do not access your collection. The process check uses an isolated bus and `--audio-null` to exercise real decoding without audible output.

The real-collection path has not been supplied, so fixture validation must not be represented as validation against that collection. Audible playback and the live Omarchy launcher/media-key path require a target-desktop check before claiming that validation.

The [specification](https://github.com/thokem74/omusic/issues/1), [design](docs/design.md), [glossary](GLOSSARY.md), and [architecture decision](docs/adr/0001-application-owned-playback-session.md) describe the agreed scope. The five browser UX prototypes are preserved on the local `prototype/ux-library` branch; A — Library is the selected reference. They are throwaway code and are not shipped as the application.
