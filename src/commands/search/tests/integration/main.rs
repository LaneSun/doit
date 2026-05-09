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
fn search_basic() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.rs"), "fn main() {\n    hello()\n}\nfn foo() {}\n").unwrap();
    fs::write(tmp.path().join("b.txt"), "fn main() {}\nother text\n").unwrap();

    let stdout = run_doit(&["search", "--include", "*.rs", "fn "], &tmp);
    let mut lines: Vec<&str> = stdout.trim().lines().collect();
    lines.sort();
    assert_snapshot!(lines.join("\n"));
}

#[test]
fn search_no_match() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.rs"), "hello world\n").unwrap();

    let stdout = run_doit(&["search", "nomatch", "--include", "*.rs"], &tmp);
    assert_snapshot!(stdout);
}
