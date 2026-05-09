use assert_cmd::Command;
use insta::{assert_snapshot, with_settings};

fn run_doit(lang: Option<&str>) -> String {
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.env("RUST_LOG", "doit=info");
    if let Some(l) = lang {
        cmd.env("LANG", l);
    } else {
        cmd.env_remove("LANG");
    }
    let assert = cmd.assert().success();
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

#[test]
fn doit_no_args_exits_successfully_en() {
    let stderr = run_doit(Some("en_US.UTF-8"));
    with_settings!({filters => vec![
        (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z", "[TIMESTAMP]"),
    ]}, {
        assert_snapshot!("en", stderr);
    });
}

#[test]
fn doit_no_args_exits_successfully_zh_cn() {
    let stderr = run_doit(Some("zh_CN.UTF-8"));
    with_settings!({filters => vec![
        (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z", "[TIMESTAMP]"),
    ]}, {
        assert_snapshot!("zh_cn", stderr);
    });
}

#[test]
fn doit_no_args_exits_successfully_default() {
    let stderr = run_doit(None);
    with_settings!({filters => vec![
        (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z", "[TIMESTAMP]"),
    ]}, {
        assert_snapshot!("default", stderr);
    });
}
