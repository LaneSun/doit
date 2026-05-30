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

/// 在真实 PTY 中驱动 `doit prompt`:验证消息显示、从 tty 读输入、reply 落盘。
#[test]
fn prompt_reads_tty_and_writes_reply() {
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;
    use std::time::Duration;

    let bin = assert_cmd::cargo::cargo_bin("doit");
    let session_dir = std::env::temp_dir().join(format!(
        "doit-prompt-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&session_dir).unwrap();

    // 开一个 PTY,子进程的 stdio 接到 slave(即一个真实 tty)
    let pty = nix::pty::openpty(None, None).unwrap();
    let slave_fd = pty.slave.as_raw_fd();

    let mut cmd = std::process::Command::new(&bin);
    cmd.args(["prompt", "What is your name?"])
        .env("RUST_LOG", "off")
        .env("LANG", "en_US.UTF-8")
        .env("DOIT_SESSION_DIR", &session_dir);
    unsafe {
        cmd.pre_exec(move || {
            for target in 0..=2 {
                if libc::dup2(slave_fd, target) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().unwrap();
    drop(pty.slave);

    let mut master: std::fs::File = std::fs::File::from(pty.master);

    // 模拟用户键入回答
    std::thread::sleep(Duration::from_millis(300));
    master.write_all(b"Alice\n").unwrap();
    master.flush().unwrap();

    // 读取 PTY 输出(子进程退出后 master 收到 EIO,视为结束)
    let mut output = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        match master.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => output.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    let _ = child.wait();

    let display = String::from_utf8_lossy(&output);
    assert!(
        display.contains("What is your name?"),
        "消息应显示在终端: {display:?}"
    );

    let reply = std::fs::read_to_string(session_dir.join(".prompt_reply")).unwrap();
    assert_eq!(reply, "Alice", "reply 文件应记录用户输入");

    let _ = std::fs::remove_dir_all(&session_dir);
}
