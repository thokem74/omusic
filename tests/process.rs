#[test]
fn real_terminal_playback_and_desktop_controls_share_one_session() {
    let result = std::process::Command::new("dbus-run-session")
        .args([
            "--",
            "python3",
            "scripts/smoke.py",
            env!("CARGO_BIN_EXE_omusic"),
        ])
        .output()
        .expect("Install dbus and python to run the process integration check");
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn delayed_mpv_events_do_not_skip_a_replacement_track() {
    let result = std::process::Command::new("dbus-run-session")
        .env("OMUSIC_TEST_STALE_EVENTS", "1")
        .args([
            "--",
            "python3",
            "scripts/smoke.py",
            env!("CARGO_BIN_EXE_omusic"),
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
