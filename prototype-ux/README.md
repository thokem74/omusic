# Omusic UX comparison — throwaway prototype

Question: which navigation model makes album listening and queue management feel most natural in a terminal?

Run from the repository root:

```sh
python3 prototype-ux/serve.py
```

Open http://127.0.0.1:8765/?variant=A. No dependencies to install. You can also open index.html directly in a browser.

| Variant | Navigation model | Main trade-off |
| --- | --- | --- |
| A | Three columns: artists → albums → tracks | More context, denser layout |
| B | Album shelf with artist filters and adjacent tracks | Releases are easier to scan; less emphasis on artist hierarchy |
| C | Search-first with grouped results and album context | Fast when you know what you want; less passive exploration |
| D | Queue desk with library and current-track inspector | Queue control takes priority over album browsing |
| E | Single-pane artist → album → track drill-down | Calmer and narrow-friendly; more navigation steps |

Use the floating comparison bar or left/right arrow keys to change variants. The URL stores the variant, so each layout can be linked directly. Other state lives only in memory; switching variants retains it, reloading resets it. Arrow keys in text fields retain normal editing behavior.

All layouts use the same fictional collection. Try compilations, guest artists, separate album editions, search, album playback, queue additions, queue editing in D, and the simulated playback controls. Press `?` for help. Expand “Inspect live prototype state” to see the full relevant state.

This is a browser simulation of a TUI, not the Rust implementation. Playback, timing, volume, repeat, and shuffle are simulated; no audio, scanning, database, MPRIS, or desktop integration is performed. Theme switching demonstrates two static palettes, not live Omarchy integration. No personal music files are read or changed. The comparison switcher and page framing belong only to this standalone prototype and must not ship in the application.

Selected: **A — Library**. The user chose the three-column artists → albums → tracks layout with a bottom playback strip. The authoritative decision is recorded in [the design](../docs/design.md). No combination with other variants was requested.

Archiving these variants on a throwaway branch remains pending before implementation. Implement the selected UX in Rust rather than promoting this prototype code.
