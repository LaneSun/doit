use assert_cmd::Command;
use insta::assert_snapshot;
use std::fs;

fn run_doit(args: &[&str], stdin: &str, tmp: &tempfile::TempDir) -> String {
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.env("LANG", "en_US.UTF-8");
    cmd.current_dir(tmp.path());
    cmd.args(args);
    if !stdin.is_empty() {
        cmd.write_stdin(stdin);
    }
    let output = cmd.output().unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn edit_lines() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("test.txt"),
        "line 1\nline 2\nline 3\nline 4\nline 5\n",
    )
    .unwrap();

    let stdout = run_doit(
        &["edit", "--lines", "2:3", "test.txt"],
        "NEW A\nNEW B\n",
        &tmp,
    );
    assert_snapshot!(stdout);
}

#[test]
fn edit_regex() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("test.txt"),
        "hello world\nfoo bar\nhello again\n",
    )
    .unwrap();

    let stdout = run_doit(
        &["edit", "--regex", "hello", "--replace", "hi", "test.txt"],
        "",
        &tmp,
    );
    assert_snapshot!(stdout);
    assert_eq!(
        fs::read_to_string(tmp.path().join("test.txt")).unwrap(),
        "hi world\nfoo bar\nhi again\n"
    );
}

#[test]
fn edit_diff() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "a\nb\nc\nd\ne\nf\ng\nh\n").unwrap();

    let diff = "@@ -2,7 +2,7 @@
 a
 b
 c
-d
+D-new
 e
 f
 g
";

    let stdout = run_doit(&["edit", "test.txt"], diff, &tmp);
    assert_snapshot!(stdout);
    let content = fs::read_to_string(tmp.path().join("test.txt")).unwrap();
    assert!(content.contains("D-new"));
}
