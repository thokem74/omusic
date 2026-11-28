# Keep playback within the terminal application's lifetime

Omusic must stop playing when its terminal interface closes, while also supporting a persistent queue and desktop media controls. Use a Rust/Ratatui application that owns the queue and MPRIS integration, controls a private mpv subprocess, and persists the collection index and stopped session in SQLite. This avoids a separate long-running playback service and delegates decoding to mpv, at the cost of an external runtime dependency and explicit subprocess lifecycle management; restoring a session never starts playback automatically.
