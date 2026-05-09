use assert_cmd::Command;
use insta::{assert_snapshot, with_settings};

#[test]
fn doit_no_args_exits_successfully() {
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.env("RUST_LOG", "doit=info");
    let assert = cmd.assert().success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);

    with_settings!({filters => vec![
        (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z", "[TIMESTAMP]"),
    ]}, {
        assert_snapshot!(stderr);
    });
}
