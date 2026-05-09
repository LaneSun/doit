use assert_cmd::Command;

#[test]
fn exit_no_output() {
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.args(["exit", "task completed"]);
    cmd.assert().success();
}
