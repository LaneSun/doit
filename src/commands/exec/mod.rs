//! `doit exec`:面向所有 Agent 的命令智能化包装器。
//!
//! - 常规模式:在内层 PTY 中运行命令(洋葱直通,format 码/尺寸与上层终端一致),
//!   缓冲输出后按行/字符上限截断,完整输出落盘到会话 logs/,把截断结果打到 stdout
//!   ——因此「终端所见 == LLM 所见」。
//! - raw 模式(`--raw`):直接继承上层 tty 运行(vim/top 等交互/全屏程序完全一致),
//!   结束后输出一行语义摘要供 LLM 感知。
//! - 全屏自动降级:常规模式若探测到 alternate screen,按交互式处理(给摘要)。
//!
//! 通过 `DOIT_SESSION_DIR` 定位日志目录;读不到则降级到临时目录,可脱离 doit 独立使用。

use std::fs;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::process::Command as StdCommand;

use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

use crate::context::RuntimeContext;
use crate::error::{DoitError, Result};
use crate::pty;

#[derive(clap::Args)]
pub struct Args {
    /// Disable output truncation
    #[arg(long, default_value_t = false)]
    pub no_truncate: bool,

    /// Raw mode: inherit the terminal directly (for vim/top and other interactive/TUI programs)
    #[arg(long, default_value_t = false)]
    pub raw: bool,

    /// Override max chars for head/tail truncation (default: config output.truncate_chars)
    #[arg(long, value_name = "N")]
    pub truncate_chars: Option<usize>,

    /// Override max lines for head/tail truncation (default: config output.truncate_lines)
    #[arg(long, value_name = "N")]
    pub truncate_lines: Option<usize>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,

    /// Shell command to execute (everything after --)
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

pub async fn execute(ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("exec.skill"));
        return Ok(());
    }
    if args.command.is_empty() {
        return Err(DoitError::config("no command specified"));
    }

    let (prog, cmd_args) = args.command.split_first().unwrap();
    let display = args.command.join(" ");

    if args.raw {
        return run_raw(prog, cmd_args, &display);
    }
    run_normal(prog, cmd_args, &display, args, &ctx.config.output)
}

/// raw 模式:继承上层 tty,交互/全屏程序与真实终端完全一致;结束后给 LLM 一行摘要。
fn run_raw(prog: &str, cmd_args: &[String], display: &str) -> Result<()> {
    let status = StdCommand::new(prog)
        .args(cmd_args)
        .status()
        .map_err(|e| DoitError::shell(format!("spawn error: {e}")))?;
    let code = status.code().unwrap_or(-1);
    println!(
        "<{}: \"{display}\", exit_code={code}>",
        rust_i18n::t!("exec.raw_summary")
    );
    std::process::exit(code);
}

/// 常规模式:内层 PTY 捕获 → 截断 → 完整落盘 → 打印截断结果(终端 == LLM)。
fn run_normal(
    prog: &str,
    cmd_args: &[String],
    display: &str,
    args: &Args,
    out_cfg: &crate::config::OutputConfig,
) -> Result<()> {
    let ws = pty::get_winsize(1); // 以本进程 stdout(上层 PTY)的尺寸作为内层 PTY 尺寸
    let (mut master, master_fd, mut child) =
        pty::spawn_on_pty(prog, cmd_args, &[], &ws)?;
    let sigwinch_read = pty::install_sigwinch_pipe()?;
    let stdin_fd = std::io::stdin().as_raw_fd();

    let mut captured: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];
    let mut stdin_open = true;

    'pump: loop {
        let (master_ready, sig_ready, stdin_ready) = {
            let master_bfd = master.as_fd();
            let sig_bfd = unsafe { BorrowedFd::borrow_raw(sigwinch_read) };
            let stdin_bfd = unsafe { BorrowedFd::borrow_raw(stdin_fd) };
            let mut fds = Vec::with_capacity(3);
            fds.push(PollFd::new(master_bfd, PollFlags::POLLIN));
            fds.push(PollFd::new(sig_bfd, PollFlags::POLLIN));
            if stdin_open {
                fds.push(PollFd::new(stdin_bfd, PollFlags::POLLIN));
            }
            poll(&mut fds, PollTimeout::NONE).map_err(|e| DoitError::shell(format!("poll: {e}")))?;
            let stdin_ready = stdin_open
                && fds
                    .get(2)
                    .and_then(|f| f.revents())
                    .is_some_and(|r| r.contains(PollFlags::POLLIN));
            (
                fds[0]
                    .revents()
                    .is_some_and(|r| r.intersects(PollFlags::POLLIN | PollFlags::POLLHUP)),
                fds[1]
                    .revents()
                    .is_some_and(|r| r.contains(PollFlags::POLLIN)),
                stdin_ready,
            )
        };

        if sig_ready {
            let mut sbuf = [0u8; 64];
            unsafe {
                libc::read(sigwinch_read, sbuf.as_mut_ptr() as *mut libc::c_void, 64);
            }
            let ws = pty::get_winsize(1);
            pty::set_winsize(master_fd, &ws);
        }

        if stdin_ready {
            let n =
                unsafe { libc::read(stdin_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n > 0 {
                let _ = master.write_all(&buf[..n as usize]);
                let _ = master.flush();
            } else if n == 0 {
                stdin_open = false;
            }
        }

        if master_ready {
            loop {
                let n = unsafe {
                    libc::read(master_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
                };
                if n > 0 {
                    captured.extend_from_slice(&buf[..n as usize]);
                } else if n == 0 {
                    break 'pump; // 子进程退出,PTY EOF
                } else {
                    let err = std::io::Error::last_os_error();
                    match err.raw_os_error() {
                        Some(libc::EAGAIN) | Some(libc::EINTR) => break,
                        // Linux 上 slave 关闭(子进程退出)后读 master 返回 EIO,视为结束
                        Some(libc::EIO) => break 'pump,
                        _ => return Err(DoitError::io(err, "read pty")),
                    }
                }
            }
        }
    }

    let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
    // 内层 PTY 的 ONLCR 会把 \n 变成 \r\n;归一化为 \n,避免再经上层 PTY 二次加 \r,
    // 同时让落盘/LLM 文本更干净(裸 \r 如进度条仍保留)。
    let text = String::from_utf8_lossy(&captured).replace("\r\n", "\n");

    // 全屏程序在常规模式下产生的是控制序列,降级为语义摘要
    if text.contains("\x1b[?1049h") {
        println!(
            "<{}: \"{display}\", exit_code={code}>",
            rust_i18n::t!("exec.fullscreen_summary")
        );
        std::process::exit(code);
    }

    if args.no_truncate {
        print!("{text}");
        std::io::stdout().flush().ok();
        std::process::exit(code);
    }

    // CLI 参数优先,否则取配置的截断阈值
    let max_chars = args.truncate_chars.unwrap_or(out_cfg.truncate_chars);
    let max_lines = args.truncate_lines.unwrap_or(out_cfg.truncate_lines);
    emit_truncated(&text, max_chars, max_lines);
    std::process::exit(code);
}

/// 按行/字符上限做头尾截断;若发生截断,完整输出落盘并附带路径提示。
fn emit_truncated(text: &str, max_chars: usize, max_lines: usize) {
    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len();
    let total_chars = visible_len(text);

    if total_lines <= max_lines * 2 && total_chars <= max_chars * 2 {
        print!("{text}");
        std::io::stdout().flush().ok();
        return;
    }

    let head = build_head(&lines, max_lines, max_chars);
    let tail = build_tail(&lines, max_lines, max_chars);
    let omitted = total_lines.saturating_sub(count_lines(&head) + count_lines(&tail));
    let log_path = write_full_log(text).unwrap_or_else(|| "(unavailable)".to_string());

    println!("{head}");
    println!(
        "\x1b[0m... [{}]",
        rust_i18n::t!("exec.truncated_notice", omitted => omitted, path => log_path)
    );
    if !tail.is_empty() {
        println!("{tail}");
    }
    std::io::stdout().flush().ok();
}

fn write_full_log(text: &str) -> Option<String> {
    let dir = crate::session::logs_dir();
    let id = &uuid::Uuid::new_v4().simple().to_string()[..8];
    let path = dir.join(format!("{id}.log"));
    match fs::write(&path, text) {
        Ok(_) => Some(path.display().to_string()),
        Err(_) => None,
    }
}

fn build_head(lines: &[&str], max_lines: usize, max_chars: usize) -> String {
    let mut head = String::new();
    let mut count = 0;
    for line in lines.iter().take(max_lines) {
        let vl = visible_len(line);
        if count + vl > max_chars {
            break;
        }
        if !head.is_empty() {
            head.push('\n');
        }
        head.push_str(line);
        count += vl;
    }
    head
}

fn build_tail(lines: &[&str], max_lines: usize, max_chars: usize) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let mut count = 0;
    for line in lines.iter().rev().take(max_lines) {
        let vl = visible_len(line);
        if count + vl > max_chars {
            break;
        }
        parts.push(line);
        count += vl;
    }
    parts.reverse();
    parts.join("\n")
}

fn count_lines(s: &str) -> usize {
    if s.is_empty() {
        0
    } else {
        s.lines().count()
    }
}

fn visible_len(s: &str) -> usize {
    let re = regex::Regex::new("\x1b\\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").len()
}
