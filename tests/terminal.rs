#[test]
#[cfg(unix)]
fn keyboard_resize_and_exit_restore_terminal_and_preserve_confirmed_save() {
    let dir = tempfile::tempdir().unwrap();
    let output = std::process::Command::new("python3")
        .args([
            "-c",
            include_str!("terminal_smoke.py"),
            env!("CARGO_BIN_EXE_precision"),
        ])
        .arg(dir.path().join("terminal.db"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
