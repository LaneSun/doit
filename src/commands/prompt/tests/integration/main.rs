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
    let session_dir = std::env::temp_dir().join(format!("doit-prompt-test-{}", std::process::id()));
    std::fs::create_dir_all(&session_dir).unwrap();

    // 开一个 PTY,子进程的 stdio 接到 slave(即一个真实 tty)
    let pty = nix::pty::openpty(None, None).unwrap();
    let slave_fd = pty.slave.as_raw_fd();

    let mut cmd = std::process::Command::new(&bin);
    cmd.args(["prompt", "What is your name?"])
        .env("RUST_LOG", "off")
        .env("LANG", "en_US.UTF-8")
        .env("TERM", "xterm-256color")
        .env("DOIT_SESSION_DIR", &session_dir);
    unsafe {
        cmd.pre_exec(move || {
            // 新会话 + 令 pts 成为控制终端(reedline 经 /dev/tty 读输入),与真实运行一致
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::ioctl(slave_fd, libc::TIOCSCTTY as libc::c_ulong, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
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
    // master 设非阻塞,便于在 read 之间穿插写入(充当极简终端)
    unsafe {
        let mfd = master.as_raw_fd();
        let flags = libc::fcntl(mfd, libc::F_GETFL);
        libc::fcntl(mfd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }

    // 充当极简终端:回应 reedline 的光标位置查询(\x1b[6n),并在提示符出现后键入回答。
    // 真实终端会自动回应 DSR;测试环境需自行模拟,否则 reedline 会阻塞等待。
    let mut output = Vec::new();
    let mut buf = [0u8; 1024];
    let mut input_sent = false;
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        match master.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let chunk = &buf[..n];
                output.extend_from_slice(chunk);
                if chunk.windows(4).any(|w| w == b"\x1b[6n") {
                    let _ = master.write_all(b"\x1b[1;1R"); // 回应光标位置
                    let _ = master.flush();
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if !input_sent && output.windows(4).any(|w| w == b"\x1b[6n") {
                    std::thread::sleep(Duration::from_millis(50));
                    master.write_all(b"Alice\r").unwrap(); // 真实终端 Enter 发 \r
                    master.flush().unwrap();
                    input_sent = true;
                } else {
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
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
