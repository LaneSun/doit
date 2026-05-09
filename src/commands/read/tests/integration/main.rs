use assert_cmd::Command;
use insta::assert_snapshot;
use std::fs;

fn run_doit(args: &[&str], tmp: &tempfile::TempDir) -> String {
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.env("LANG", "en_US.UTF-8");
    cmd.current_dir(tmp.path());
    cmd.args(args);
    let output = cmd.output().unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn read_full_file() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.txt");
    fs::write(&file, "line one\nline two\nline three\n").unwrap();

    let stdout = run_doit(&["read", &file.to_string_lossy()], &tmp);
    assert_snapshot!(stdout);
}

#[test]
fn read_lines_range() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.txt");
    fs::write(&file, "a\nb\nc\nd\ne\n").unwrap();

    let stdout = run_doit(&["read", "--lines", "2:4", &file.to_string_lossy()], &tmp);
    assert_snapshot!(stdout);
}

#[test]
fn read_truncated() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("big.txt");
    let content: String = (1..=510)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&file, &content).unwrap();

    let stdout = run_doit(&["read", &file.to_string_lossy()], &tmp);
    assert_snapshot!(stdout);
}
