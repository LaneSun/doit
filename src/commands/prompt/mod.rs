//! `doit prompt`:向用户展示一段消息(可选)并阻塞读取一行输入。
//!
//! - 消息来自可选位置参数;给出时本命令打印它(像普通命令一样显示其「输出」)。
//! - 用户输入从 stdin 读取(常驻 bash 中 stdin 即 PTY tty,默认回显)。
//!   stdin 非 tty(被重定向/测试环境)时优雅降级,绝不阻塞。
//! - 读到的输入写入 `$DOIT_SESSION_DIR/.prompt_reply`,供父 agent 确定性取回。

use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::os::fd::{AsRawFd, BorrowedFd};

use nix::sys::termios::{self, LocalFlags, SetArg, Termios};

use crate::context::RuntimeContext;
use crate::error::{DoitError, Result};

/// 橘黄色分隔线前景色。
const ORANGE: &str = "\x1b[38;2;235;145;30m";
const RESET: &str = "\x1b[0m";

/// 父 agent 取回用户输入的文件名(位于会话目录下)。
pub const REPLY_FILE: &str = ".prompt_reply";

#[derive(clap::Args)]
pub struct Args {
    /// Prompt message to display before reading input
    pub message: Option<String>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("prompt.skill"));
        return Ok(());
    }

    // stdin 非 tty(重定向/测试)→ 无法交互,优雅降级,绝不阻塞。
    if !io::stdin().is_terminal() {
        eprintln!("{}", rust_i18n::t!("prompt.not_available"));
        return Ok(());
    }

    // 可选消息(LLM 手工 prompt 的提问)显示在输入框上方
    if let Some(message) = &args.message {
        if !message.is_empty() {
            print!("{message}");
            if !message.ends_with('\n') {
                println!();
            }
        }
    }

    // 输入框:橘黄分隔线 + 橘黄竖条 + `> <输入>` + 橘黄分隔线
    print_divider();
    print!("{ORANGE}▌{RESET}> ");
    io::stdout().flush().ok();

    // 常驻 PTY 回显已永久关闭,临时开启回显使用户输入可见;读完即恢复。
    let echo_guard = EchoGuard::enable();

    let mut line = String::new();
    let n = io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| DoitError::io(e, "read stdin"))?;
    if n == 0 {
        println!(); // EOF(Ctrl-D):换行让光标落到下一行
    }
    drop(echo_guard);

    print_divider();

    let input = line.trim_end_matches(['\n', '\r']).to_string();
    write_reply(&input);
    Ok(())
}

/// 打印一条占满终端宽度的橘黄色分隔线。
fn print_divider() {
    let width = term_width();
    let line = "─".repeat(width);
    println!("{ORANGE}{line}{RESET}");
}

/// 通过 TIOCGWINSZ 获取终端列数,失败回退 80。
fn term_width() -> usize {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::ioctl(io::stdout().as_raw_fd(), libc::TIOCGWINSZ, &mut ws) };
    if rc == 0 && ws.ws_col > 0 {
        ws.ws_col as usize
    } else {
        80
    }
}

/// 临时在 stdin 上开启本地回显(ECHO),析构时恢复原状。
struct EchoGuard {
    orig: Option<Termios>,
}

impl EchoGuard {
    fn enable() -> Self {
        let bfd = unsafe { BorrowedFd::borrow_raw(io::stdin().as_raw_fd()) };
        if let Ok(orig) = termios::tcgetattr(bfd) {
            let mut t = orig.clone();
            t.local_flags |= LocalFlags::ECHO | LocalFlags::ICANON | LocalFlags::ECHOE;
            if termios::tcsetattr(bfd, SetArg::TCSANOW, &t).is_ok() {
                return Self { orig: Some(orig) };
            }
        }
        Self { orig: None }
    }
}

impl Drop for EchoGuard {
    fn drop(&mut self) {
        if let Some(orig) = &self.orig {
            let bfd = unsafe { BorrowedFd::borrow_raw(io::stdin().as_raw_fd()) };
            let _ = termios::tcsetattr(bfd, SetArg::TCSANOW, orig);
        }
    }
}

/// 将用户输入写入会话目录的 reply 文件,供父 agent 取回。
fn write_reply(input: &str) {
    let path = crate::session::resolve_session_dir().join(REPLY_FILE);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, input);
}
