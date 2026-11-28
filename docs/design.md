# Omusic design

Status: agreed. The user confirmed shared understanding after the final interview round. This document defines the first-version scope; implementation and verification are tracked in [issue #1](https://github.com/thokem74/omusic/issues/1).

## Confirmed decisions

- Build a terminal user interface for the user's local MP3 collection.
- Primary browsing flow: artist → album → tracks, with quick search available throughout.
- Follow the active Omarchy theme.
- Support launching from the desktop app launcher or a shortcut.
- Support desktop media keys and expose the current track to desktop media controls.
- Closing the TUI stops playback; playback does not continue in the background.
- Browse and play existing files without modifying them.
- Tag editing, file renaming, downloading, and streaming are outside the first version.
- Browse using artist, album, and track-number tags; fall back to folder names and filenames when tags are missing.
- Playing an album immediately replaces the queue. Separate add-to-queue actions append an album or individual track.
- Restore browsing position, volume, and queue across launches; remain stopped until the user starts playback.
- Use a compact artist/album/track browser with a bottom playback strip, arrow keys and Vim-style navigation, `/` for search, and a visible help shortcut.
- Selected UX: A — Library from the five-variant prototype comparison. Use three columns for artists → albums → tracks, with the playback strip at the bottom.
- Include seeking, volume, previous/next, shuffle, and repeat off/track/queue.
- Support one or more configurable collection folders, defaulting to ~/Music; offer folder selection when no tracks are found. The actual collection path has not been supplied.
- Scan in the background at startup, showing previously indexed music immediately; provide manual rescanning. Continuous filesystem watching is deferred.
- Search artists, albums, and tracks across the collection. Artist and album results open for browsing; playing a track result replaces the queue with its album and starts at that track.
- Restore the current track and its playback position, waiting for explicit play after launch.
- Show an error and skip missing or corrupt queued files, stopping if none remain playable; retain errors for inspection.
- Allow viewing, removing, reordering, and clearing queue entries. Removing the current entry advances to the next track or stops if none remain. Saved named playlists are deferred.
- Prefer album artist for grouping, retain compilations as albums, and order tracks by disc then track number. Without usable tags, treat each folder as an album.
- Keep distinct files as distinct tracks even when tags match; avoid duplicate discovery of the same file through overlapping collection roots.
- Shuffle preserves the visible queue order and current playback, visiting each remaining entry once before repeating. Disabling shuffle returns to normal queue order.
- Permit one active Omusic session per user; repeated launches focus the existing window where possible instead of producing competing audio.
- Apply Omarchy theme changes live, with standard terminal colors as fallback. Adapt to narrow windows by reducing browser columns while retaining playback controls.
- Album artwork and visualizers are outside version one.
- Implement in Rust with Ratatui, a private mpv subprocess for playback, and SQLite for the collection index and session. Omusic owns the queue and desktop integration and stops its playback subprocess when the interface exits.
- Follow official Rust style guidance, naming conventions, and tooling under the agreed policy in docs/development.md. Use default rustfmt formatting, required local and CI checks, compiler/default Clippy warnings as failures, and an exact stable toolchain pin with rustfmt and Clippy. Additional lint groups are selective; nightly features require a separate decision.
- Target the user's Omarchy machine first with an Arch package, launcher entry, and documented optional shortcut; retain ordinary Linux terminal usage.
- Keep albums in different folders separate even when tags match, while keeping disc subfolders belonging to one album together.
- Preserve indexed tracks and queue entries when a collection root is temporarily unavailable. Remove vanished tracks from the browser only following a successful scan of their available root; unavailable queued entries follow the normal skip-and-report rule.
- Previous restarts the current track after more than three seconds of playback; otherwise it moves to the previous track. Clearing the queue immediately stops playback.
- Initial verification uses generated MP3 fixtures. Verification against the user's actual collection remains pending until its location is available.

## Verified local integration context

- The repository initially contains only an empty AGENTS.md.
- Omarchy is installed at /usr/share/omarchy.
- The current theme's semantic colors are in ~/.local/state/omarchy/current/theme/colors.toml. Theme switching replaces the directory, so live reload must account for directory replacement.
- The preferred terminal is Ghostty. Omarchy provides terminal launch and launch-or-focus helpers with app identity support.
- The Omarchy shell consumes MPRIS for desktop media controls. Merely starting a playback executable does not establish MPRIS support.
- mpv and libmpv are installed. The selected playback implementation uses an mpv subprocess.
- The configured XDG music directory is /home/thokem/Music; it exists but is empty. Actual collection location is unresolved.
- Rust/Cargo, Go, Python, and ffprobe are installed.

## Acceptance criteria

- Browse and search a fixture collection covering tagged albums, compilations, multiple discs, separate editions, missing tags, and overlapping collection roots.
- Album playback, track search playback, queue editing, shuffle, repeat, seeking, volume, and previous/next obey the confirmed rules.
- Exiting the TUI stops its playback subprocess. Reopening restores the stopped session, including playback position.
- Missing or corrupt tracks report errors and do not prevent subsequent playable entries from playing. Exhausting playable entries stops playback, including under repeat modes.
- A temporarily unavailable collection root retains its index and queue entries; a successful rescan of an available root reconciles deleted tracks.
- Desktop media controls operate through MPRIS and show the current track. A repeated launch does not create competing playback.
- Live theme changes and terminal resizing retain a usable interface, with terminal-color fallback outside Omarchy.
- The Arch package installs the executable and launcher entry; documentation explains collection configuration, controls, optional shortcut setup, and runtime dependencies.
- Verify against the real collection when its location becomes available; report fixture-only validation as such until then.

## Albums with different artists

The user confirmed the following refinements in a follow-up design interview:

- Album artist and track artist are separate concepts. A compilation credited to Various Artists remains one complete album; differing track artists and guest credits do not split a coherent album.
- Show each track's artist in album track lists, the queue, search results, and the playback strip when it differs from the album artist.
- An artist's browser page includes an Appears on section for compilations and collaborations credited to other album artists. Opening one shows the complete album with that artist's tracks highlighted.
- Verification must cover a compilation with multiple track artists and an album with guest credits, including track-credit display and Appears on navigation.
- When all album-artist credits are absent, use the shared track-artist credit if every track has the same nonempty credit; otherwise display Various Artists. This is an inferred display credit and does not modify file tags.
- When album-artist credits conflict within an otherwise coherent album, keep the album intact, display Various Artists, show a metadata warning, and preserve original credits. Differing track-artist credits alone are not a metadata error.
- Recognize separate contributors when the metadata explicitly lists them. Preserve combined credits such as A feat. B as written without guessing at separators. The full credit remains searchable; only explicitly identified contributors get individual artist pages and Appears on membership.
- Verification must cover absent and conflicting album-artist credits, explicitly listed contributors, and combined credits or band names containing punctuation. Inferred display credits and warnings must leave the source files unchanged.

The user confirmed all album-credit refinements. Opening an album from Appears on retains the existing full-album browsing and playback rules; highlighted contributions do not filter or replace the album's contents.

## Handoff

### UX prototype comparison

The user selected **A — Library**, the three-column artists → albums → tracks layout with a bottom playback strip. This settles the layout comparison; the remaining agreed behavior above still applies. The user did not provide an additional rationale or request a combination with other variants.

The throwaway browser simulation is preserved on the local `prototype/ux-library` branch at commit `94dac23ae233eae69434b72e2b4a110db29f3b8a`. Its `prototype-ux/README.md` explains the comparison, and variant A is the visual reference. The production implementation is a Rust TUI; the browser prototypes are removed from the implementation branch.

The [interview answer audit](interview-audit.md) reconciles all 35 questions with the user's answers. No previously asked design question remains unanswered.

The original design interview and the follow-up interviews on album credits and Rust development policy are complete. The actual collection path is still unknown; it does not block implementation or fixture-based verification. Routine implementation details may be chosen within the confirmed behavior. Revisit this design if implementation reveals a consequential trade-off that changes the user experience.
