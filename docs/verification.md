# Issue #1 verification

Implemented [Omusic v1](https://github.com/thokem74/omusic/issues/1) on the current `main` branch. The selected UX prototype is preserved on local branch `prototype/ux-library`; production code is Rust.

## Required gates

The pinned Rust 1.98.1 toolchain passed formatting, all-target compilation, Clippy with warnings denied, workspace tests including documentation-test discovery, and documentation generation with warnings denied. The full suite passed once after the review fixes: ten application-session tests and two process integration tests.

Session tests use generated MP3s and real scratch SQLite storage. They cover compilation/multidisc grouping, editions, credit fallbacks, contributors, overlapping roots, queue edits and restored position/volume, search playback, controls, failure exhaustion under repeat, shuffle invariants, and disconnected roots.

The isolated process checks exercise the real terminal, mpv with audio-free decoding, MPRIS metadata and controls, seek, queue clearing, resizing, repeated launch, restored-but-stopped startup, normal exit, and SIGHUP cleanup. A protocol fixture reproduces a delayed EOF during track replacement and verifies that it cannot skip the replacement track.

## Review

Two independent reviews compared the implementation with starting commit `94dac23ae233eae69434b72e2b4a110db29f3b8a` and issue #1.

- Standards: two maintainability judgments were addressed by centralizing artist album membership and MPRIS repeat conversion. Follow-up review reported no manually actionable unresolved findings.
- Spec: two correctness findings were fixed with regressions that failed before the fixes: folder requests dropped during an active scan, and delayed mpv events attributed to a replacement track. Follow-up review reported no unresolved material findings or serious new regressions.

## Package

Built the x86_64 Arch package at `target/package/omusic-0.1.0-1-x86_64.pkg.tar.zst`. Disabled makepkg's GCC LTO for compatibility with bundled SQLite and Rust's linker; the Cargo release profile retains its Rust optimization policy. Disabled an empty debug package because the release executable is stripped.

Verified the packaged executable's version and runtime linkage, packaged desktop entry, silent real-process integration, and the launcher's exact Omarchy command routing after extraction to a temporary directory. Removing that temporary directory leaves the system installation untouched. Dependency checks were skipped during the local makepkg run because the required build/runtime tools were already present; package metadata declares the runtime and build dependencies.

## Remaining environmental validation

- The actual collection location has not been supplied, so real-collection compatibility is not yet verified.
- Audible output and live Omarchy media-key/theme/launcher observation remain target-desktop checks. The tests validate MPRIS and launcher routing in isolation; they do not claim to have heard audio or operated the user's desktop.
- The package was built and verified through isolated extraction, not installed into the user's system with pacman. No system configuration was changed.
- GitHub Actions is configured but has not been run remotely; local checks passed. Commits and the prototype archive branch have not been pushed.
