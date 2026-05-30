//! 常驻 Shell 会话:在 PTY 中运行一个长期存活的 bash,把 LLM 的命令喂给它,
//! 并通过 PS1 哨兵在连续的会话流里精确切出每条命令的输出与退出码。
//!
//! 终端模型遵循「洋葱直通」原则:
//! - 真实 tty 设为 raw,所有字节原样透传到 PTY,行规程只在 PTY 内部生效;
//! - 命令输出实时转发回真实 tty(过滤掉我们注入的 PS1 哨兵),用户看到的与真实 shell 一致;
//! - 用户输入(密码、TUI 操作)实时转发进 PTY,交互行为与常规终端无异;
//! - SIGWINCH 经自管道转发,实时同步窗口尺寸,保证 vim 等全屏程序正确重排。

use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};
use std::path::{Path, PathBuf};
use std::process::Child;

use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::termios::{self, LocalFlags, SetArg};

use crate::commands::prompt::REPLY_FILE;
use crate::error::{DoitError, Result};
use crate::pty::{self, RawModeGuard};

/// PS1 哨兵使用的分隔控制字符 (RS, Record Separator)。命令输出几乎不会包含它。
const RS: u8 = 0x1e;

/// 命令执行结果。
pub struct CommandOutput {
    /// 给 LLM / 存入 Block 的文本(已剥除命令回显与哨兵;全屏程序则为语义摘要)。
    pub output: String,
    pub exit_code: i32,
    /// 是否检测到全屏 TUI(alternate screen),此时 output 为语义摘要。
    pub fullscreen: bool,
}

/// PS1 哨兵增量匹配器:逐字节扫描输出流,过滤掉精确匹配的哨兵
/// `RS <nonce> RS <digits> RS`,并在匹配完成时返回退出码。
/// 不匹配的字节原样输出(支持跨 read 边界的部分哨兵)。
struct SentinelMatcher {
    nonce: Vec<u8>,
    phase: Phase,
    stash: Vec<u8>,
    digits: Vec<u8>,
}

#[derive(Clone, Copy)]
enum Phase {
    Idle,
    Nonce(usize),
    Sep2,
    Digits,
}

impl SentinelMatcher {
    fn new(nonce: &str) -> Self {
        Self {
            nonce: nonce.as_bytes().to_vec(),
            phase: Phase::Idle,
            stash: Vec::new(),
            digits: Vec::new(),
        }
    }

    /// 喂入一段原始字节,返回 (应转发/保留的字节, 命令结束时的退出码)。
    fn feed(&mut self, input: &[u8]) -> (Vec<u8>, Option<i32>) {
        let mut out = Vec::with_capacity(input.len());
        let mut found = None;
        let mut q: VecDeque<u8> = input.iter().copied().collect();

        while let Some(b) = q.pop_front() {
            match self.phase {
                Phase::Idle => {
                    if b == RS {
                        self.phase = Phase::Nonce(0);
                        self.stash.clear();
                        self.stash.push(RS);
                    } else {
                        out.push(b);
                    }
                }
                Phase::Nonce(k) => {
                    if b == self.nonce[k] {
                        self.stash.push(b);
                        self.phase = if k + 1 == self.nonce.len() {
                            Phase::Sep2
                        } else {
                            Phase::Nonce(k + 1)
                        };
                    } else {
                        self.mismatch(b, &mut out, &mut q);
                    }
                }
                Phase::Sep2 => {
                    if b == RS {
                        self.stash.push(b);
                        self.phase = Phase::Digits;
                        self.digits.clear();
                    } else {
                        self.mismatch(b, &mut out, &mut q);
                    }
                }
                Phase::Digits => {
                    if b.is_ascii_digit() {
                        self.stash.push(b);
                        self.digits.push(b);
                    } else if b == RS {
                        let code = if self.digits.is_empty() {
                            -1
                        } else {
                            String::from_utf8_lossy(&self.digits).parse().unwrap_or(-1)
                        };
                        found = Some(code);
                        self.phase = Phase::Idle;
                        self.stash.clear();
                        self.digits.clear();
                    } else {
                        self.mismatch(b, &mut out, &mut q);
                    }
                }
            }
        }
        (out, found)
    }

    /// 匹配失败:输出 stash 的首字节,把剩余字节 + 当前字节重新入队从头匹配。
    fn mismatch(&mut self, b: u8, out: &mut Vec<u8>, q: &mut VecDeque<u8>) {
        self.phase = Phase::Idle;
        let stash = std::mem::take(&mut self.stash);
        out.push(stash[0]);
        let mut reproc: Vec<u8> = stash[1..].to_vec();
        reproc.push(b);
        for &x in reproc.iter().rev() {
            q.push_front(x);
        }
    }
}

pub struct ShellSession {
    master: File,
    master_fd: RawFd,
    stdin_fd: RawFd,
    child: Child,
    sentinel: SentinelMatcher,
    sigwinch_read: RawFd,
    session_dir: PathBuf,
    _raw_guard: Option<RawModeGuard>,
}

impl ShellSession {
    /// 启动常驻 bash 会话。`session_dir` 通过 `DOIT_SESSION_DIR` 暴露给子命令。
    pub fn spawn(session_dir: &Path) -> Result<Self> {
        let stdin_fd = std::io::stdin().as_raw_fd();
        let ws = pty::get_winsize(stdin_fd);
        let term = std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string());
        let sdir = session_dir.to_string_lossy().to_string();

        // --noediting:关闭 readline 行编辑。这样命令回显由终端行规程负责,
        // 我们得以永久关闭回显、自行打印格式化的命令行,且不会打乱 readline 状态。
        let args = [
            "--noediting".to_string(),
            "--norc".to_string(),
            "--noprofile".to_string(),
            "-i".to_string(),
        ];
        let (master, master_fd, child) = pty::spawn_on_pty(
            "bash",
            &args,
            &[("TERM", term.as_str()), ("DOIT_SESSION_DIR", sdir.as_str())],
            &ws,
        )?;

        let sigwinch_read = pty::install_sigwinch_pipe()?;
        let raw_guard = RawModeGuard::new(stdin_fd)?;

        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let sentinel = SentinelMatcher::new(&nonce);

        let mut session = Self {
            master,
            master_fd,
            stdin_fd,
            child,
            sentinel,
            sigwinch_read,
            session_dir: session_dir.to_path_buf(),
            _raw_guard: raw_guard,
        };
        session.init_shell(&nonce)?;
        session.disable_echo(); // 永久关闭内层 PTY 回显,命令不再被行规程回显
        Ok(session)
    }

    /// 永久关闭内层 PTY 的本地回显(只设一次,绝不中途切换,避免竞态)。
    /// 命令显示改由 agent 格式化打印;需要回显的程序(如 doit prompt)自行临时开启。
    fn disable_echo(&self) {
        let bfd = unsafe { BorrowedFd::borrow_raw(self.master_fd) };
        if let Ok(mut t) = termios::tcgetattr(bfd) {
            t.local_flags &= !LocalFlags::ECHO;
            let _ = termios::tcsetattr(bfd, SetArg::TCSANOW, &t);
        }
    }

    /// 注入 PS1/PROMPT_COMMAND,并同步到第一个干净提示符(丢弃启动噪音)。
    fn init_shell(&mut self, nonce: &str) -> Result<()> {
        // PROMPT_COMMAND 仅设变量(零子进程开销);PS1 输出: RS nonce RS 上条退出码 RS
        let init = format!(
            "PROMPT_COMMAND='__doit_ec=$?'; PS1=$'\\x1e'\"{nonce}\"$'\\x1e''${{__doit_ec}}'$'\\x1e'\n"
        );
        self.master
            .write_all(init.as_bytes())
            .map_err(|e| DoitError::io(e, "write shell init"))?;
        self.master.flush().ok();
        self.pump(false)?; // 丢弃直到第一个哨兵,不向终端转发
        Ok(())
    }

    /// 执行一条命令:写入 bash,实时转发输出到终端,返回输出文本与退出码。
    /// 回显已永久关闭,捕获到的即命令的纯输出(无需再剥离命令回显)。
    pub fn run_command(&mut self, cmd: &str) -> Result<CommandOutput> {
        self.master
            .write_all(cmd.as_bytes())
            .map_err(|e| DoitError::io(e, "write command"))?;
        self.master.write_all(b"\n").ok();
        self.master.flush().ok();

        let (filtered, code) = self.pump(true)?;
        let text = String::from_utf8_lossy(&filtered).to_string();

        if text.contains("\x1b[?1049h") {
            Ok(CommandOutput {
                output: format!("<interactive program ran and exited, exit_code={code}>"),
                exit_code: code,
                fullscreen: true,
            })
        } else {
            Ok(CommandOutput {
                output: text,
                exit_code: code,
                fullscreen: false,
            })
        }
    }

    /// 运行裸 `doit prompt` 读取用户输入(content 转换路径用)。命令照常回显,
    /// 符合「可见即所执行」的 shell 语义;用户最终输入经 reply 文件确定性取回。
    pub fn prompt_input(&mut self) -> Result<String> {
        self.run_and_capture_reply("doit prompt")
    }

    /// 运行 LLM 手工发起的 doit prompt 工具调用,同样经 reply 文件取回输入。
    pub fn run_prompt_command(&mut self, cmd: &str) -> Result<String> {
        self.run_and_capture_reply(cmd)
    }

    /// 运行一条会写 reply 文件的命令(doit prompt 系列),返回其中的用户输入。
    fn run_and_capture_reply(&mut self, cmd: &str) -> Result<String> {
        let reply_path = self.session_dir.join(REPLY_FILE);
        let _ = std::fs::remove_file(&reply_path);
        self.run_command(cmd)?;
        let reply = std::fs::read_to_string(&reply_path).unwrap_or_default();
        Ok(reply.trim_end_matches(['\n', '\r']).to_string())
    }

    /// 代理循环:在 master / stdin / SIGWINCH 之间转发,直到命令结束哨兵出现。
    /// 返回 (过滤掉哨兵后的输出字节, 退出码)。`forward` 控制是否实时写回真实终端。
    fn pump(&mut self, forward: bool) -> Result<(Vec<u8>, i32)> {
        let mut collected = Vec::new();
        let mut buf = [0u8; 8192];
        let mut stdin_open = true;

        loop {
            // 在独立作用域内构建 PollFd,避免对 self.master 的借用与后续可变借用冲突
            let (master_ready, sig_ready, stdin_ready) = {
                let master_bfd = self.master.as_fd();
                let sig_bfd = unsafe { BorrowedFd::borrow_raw(self.sigwinch_read) };
                let stdin_bfd = unsafe { BorrowedFd::borrow_raw(self.stdin_fd) };
                let mut fds = Vec::with_capacity(3);
                fds.push(PollFd::new(master_bfd, PollFlags::POLLIN));
                fds.push(PollFd::new(sig_bfd, PollFlags::POLLIN));
                if stdin_open {
                    fds.push(PollFd::new(stdin_bfd, PollFlags::POLLIN));
                }
                poll(&mut fds, PollTimeout::NONE)
                    .map_err(|e| DoitError::shell(format!("poll: {e}")))?;
                let stdin_ready = stdin_open
                    && fds
                        .get(2)
                        .and_then(|f| f.revents())
                        .is_some_and(|r| r.contains(PollFlags::POLLIN));
                (
                    revents_in(&fds[0]),
                    fds[1]
                        .revents()
                        .is_some_and(|r| r.contains(PollFlags::POLLIN)),
                    stdin_ready,
                )
            };

            // SIGWINCH:同步窗口尺寸
            if sig_ready {
                let mut sbuf = [0u8; 64];
                unsafe {
                    libc::read(self.sigwinch_read, sbuf.as_mut_ptr() as *mut libc::c_void, 64);
                }
                let ws = pty::get_winsize(self.stdin_fd);
                pty::set_winsize(self.master_fd, &ws);
            }

            // 用户输入 → PTY
            if stdin_ready {
                let n = unsafe {
                    libc::read(self.stdin_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
                };
                if n > 0 {
                    let _ = self.master.write_all(&buf[..n as usize]);
                    let _ = self.master.flush();
                } else if n == 0 {
                    stdin_open = false; // stdin 关闭(EOF):停止轮询,避免忙等
                }
            }

            // PTY 输出 → 终端 + capture
            if master_ready {
                loop {
                    let n = unsafe {
                        libc::read(
                            self.master_fd,
                            buf.as_mut_ptr() as *mut libc::c_void,
                            buf.len(),
                        )
                    };
                    if n > 0 {
                        let (pass, code) = self.sentinel.feed(&buf[..n as usize]);
                        if forward {
                            pty::write_fd(1, &pass);
                        }
                        collected.extend_from_slice(&pass);
                        if let Some(c) = code {
                            return Ok((collected, c));
                        }
                    } else if n == 0 {
                        return Ok((collected, -1)); // bash 退出
                    } else {
                        let err = std::io::Error::last_os_error();
                        match err.raw_os_error() {
                            Some(libc::EAGAIN) | Some(libc::EINTR) => break,
                            // Linux 上 slave 关闭后读 master 返回 EIO,视为 bash 退出
                            Some(libc::EIO) => return Ok((collected, -1)),
                            _ => return Err(DoitError::io(err, "read pty")),
                        }
                    }
                }
            }
        }
    }
}

fn revents_in(fd: &PollFd) -> bool {
    fd.revents()
        .is_some_and(|r| r.intersects(PollFlags::POLLIN | PollFlags::POLLHUP))
}



impl Drop for ShellSession {
    fn drop(&mut self) {
        // 尝试让 bash 正常退出,然后回收子进程,避免僵尸
        let _ = self.master.write_all(b"exit\n");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::ShellSession;
    use std::path::PathBuf;

    fn tmp_session() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "doit-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 验证常驻 shell 的核心契约:哨兵切边界、退出码、以及 cd/env 状态跨命令持久。
    #[test]
    fn resident_shell_contract() {
        let dir = tmp_session();
        let mut sh = ShellSession::spawn(&dir).expect("spawn shell");

        let out = sh.run_command("echo hello123").unwrap();
        assert!(out.output.contains("hello123"), "echo 输出: {:?}", out.output);
        assert_eq!(out.exit_code, 0, "echo 退出码");

        let f = sh.run_command("false").unwrap();
        assert_eq!(f.exit_code, 1, "false 退出码应为 1");

        // cd 持久:常驻 shell 状态跨命令保留(本项目的关键设计目标)
        sh.run_command("cd /tmp").unwrap();
        let pwd = sh.run_command("pwd").unwrap();
        assert!(pwd.output.contains("/tmp"), "cd 后 pwd: {:?}", pwd.output);

        // 环境变量持久
        sh.run_command("export DOIT_TV=xyz789").unwrap();
        let v = sh.run_command("echo $DOIT_TV").unwrap();
        assert!(v.output.contains("xyz789"), "env 持久: {:?}", v.output);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
