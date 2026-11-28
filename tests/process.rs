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
