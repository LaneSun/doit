use assert_cmd::Command;
use insta::assert_snapshot;

fn run_doit(args: &[&str]) -> String {
    let mut cmd = Command::cargo_bin("doit").unwrap();
    cmd.env("LANG", "en_US.UTF-8");
    cmd.args(args);
    let output = cmd.output().unwrap();
    // Strip ANSI for stable snapshots
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let re = regex::Regex::new("\x1b\\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(&stdout, "").into_owned()
}

#[test]
fn template_system_interactive() {
    let stdout = run_doit(&["template", "system", "--interactive"]);
    assert!(stdout.contains("shell-first AI assistant"));
    assert_snapshot!(stdout);
}

#[test]
fn template_system_noninteractive() {
    let stdout = run_doit(&["template", "system"]);
    assert!(stdout.contains("shell-first AI assistant"));
    assert_snapshot!(stdout);
}
