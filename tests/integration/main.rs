use assert_cmd::Command;
use insta::assert_snapshot;

#[test]
fn doit_help_output() {
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.arg("--help");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_snapshot!(stdout);
}
