use assert_cmd::Command;
use std::fs;

fn run_doit(args: &[&str], stdin: &str, tmp: &tempfile::TempDir) {
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.env("LANG", "en_US.UTF-8");
    cmd.current_dir(tmp.path());
    cmd.args(args);
    cmd.write_stdin(stdin);
    cmd.assert().success();
}

#[test]
fn write_and_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("out.txt");

    run_doit(&["write", &file.to_string_lossy()], "hello\n", &tmp);
    assert_eq!(fs::read_to_string(&file).unwrap(), "hello\n");

    run_doit(&["write", &file.to_string_lossy()], "world\n", &tmp);
    assert_eq!(fs::read_to_string(&file).unwrap(), "world\n");
}

#[test]
fn write_append() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("out.txt");

    run_doit(&["write", &file.to_string_lossy()], "line 1\n", &tmp);
    run_doit(&["write", "--append", &file.to_string_lossy()], "line 2\n", &tmp);
    assert_eq!(fs::read_to_string(&file).unwrap(), "line 1\nline 2\n");
}
