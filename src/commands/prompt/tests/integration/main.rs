use assert_cmd::Command;
use insta::assert_snapshot;

#[test]
fn prompt_non_tty() {
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.args(["prompt", "test message"]);
    cmd.env("RUST_LOG", "off");
    cmd.env("LANG", "en_US.UTF-8");
    let assert = cmd.assert().success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert_snapshot!(stderr);
}
