use assert_cmd::Command;
use insta::assert_snapshot;

fn run_doit(args: &[&str]) -> String {
    let session = std::env::temp_dir().join("doit-exec-it");
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.env("LANG", "en_US.UTF-8");
    cmd.env("RUST_LOG", "off");
    cmd.env("DOIT_SESSION_DIR", &session);
    cmd.args(args);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    strip_ansi(&stdout)
}

fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new("\x1b\\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").into_owned()
}

#[test]
fn exec_simple() {
    let stdout = run_doit(&["exec", "--", "echo", "hello"]);
    assert!(stdout.contains("hello"));
}

#[test]
fn exec_truncated() {
    let cmd = r#"i=1; while [ $i -le 200 ]; do echo "line $i"; i=$((i+1)); done"#;
    let stdout = run_doit(&["exec", "--truncate-lines", "10", "--", "sh", "-c", cmd]);
    // 日志路径含随机 uuid,redact 以保证快照确定性
    insta::with_settings!({filters => vec![(r"full output: [^\]]+", "full output: <LOG>")]}, {
        assert_snapshot!(stdout);
    });
}
