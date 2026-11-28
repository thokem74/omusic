# Rust development policy

Status: agreed by the user. Apply this policy to implementation, review, tooling, and CI work.

Follow the [official Rust Style Guide](https://doc.rust-lang.org/style-guide/) for source style and the [Rust API Guidelines naming conventions](https://rust-lang.github.io/api-guidelines/naming.html) for names. These are adopted project conventions; the API Guidelines are recommendations rather than language requirements.

Use `snake_case` for modules, functions, methods, and local variables; `UpperCamelCase` for types, traits, and enum variants; and `SCREAMING_SNAKE_CASE` for constants and statics. Apply the remaining naming guidance, including Rust's established acronym and conversion conventions, where relevant.

Use the standard Rust tooling:

- [Cargo](https://doc.rust-lang.org/cargo/) for building, checking, testing, and documentation generation.
- [rustfmt](https://github.com/rust-lang/rustfmt) through `cargo fmt` for formatting.
- [Clippy](https://doc.rust-lang.org/clippy/) through `cargo clippy` for linting.
- [rustdoc](https://doc.rust-lang.org/rustdoc/) through `cargo doc` for documentation.
- [rustup](https://rust-lang.github.io/rustup/overrides.html) for toolchain selection and formatter/linter components.

## Formatting and naming

Use rustfmt's defaults without project-specific style overrides. Follow the API Guidelines' naming conventions: snake_case for modules, functions, methods, and local variables; UpperCamelCase for types, traits, and enum variants; SCREAMING_SNAKE_CASE for constants and statics. These recommendations are adopted as project policy; they are not all language requirements.

## Required checks

Run the same required checks locally and in CI:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Compiler warnings and default Clippy warnings fail the required checks. Keep documentation tests in the test run. Enable additional lint groups selectively rather than enabling all optional groups. Keep any necessary lint exception narrow and explain its reason.

Integration tests also need Python 3, ffmpeg, mpv, D-Bus tools (`dbus-run-session` and `busctl`), and a working pseudo-terminal. They use generated fixtures, an isolated session bus, temporary state, and an audio-free playback target. Run process integration checks outside a sandbox that prohibits sockets or pseudo-terminals.

## Toolchain

Use the stable version pinned in the root toolchain file with the rustfmt and Clippy components, locally and in CI. Update the pin deliberately; using nightly features requires a separate design decision.
