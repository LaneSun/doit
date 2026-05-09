use assert_cmd::Command;
use insta::assert_snapshot;
use std::fs;

#[test]
fn glob_matches_files() {
    // Create temp test files
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.rs"), "").unwrap();
    fs::write(tmp.path().join("b.rs"), "").unwrap();
    fs::create_dir(tmp.path().join("sub")).unwrap();
    fs::write(tmp.path().join("sub/c.rs"), "").unwrap();

    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.current_dir(tmp.path());
    cmd.args(["glob", "**/*.rs"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    // Sort for deterministic output
    let mut lines: Vec<&str> = stdout.trim().lines().collect();
    lines.sort();
    assert_snapshot!(lines.join("\n"));
}
