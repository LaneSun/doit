//! 底层 PTY / 终端原语,供常驻 shell 会话(`agent::shell`)与 `doit exec` 共用。
//!
//! 这里只放与具体业务无关的 Linux 终端机制:窗口尺寸 ioctl、非阻塞 fd、
//! SIGWINCH 自管道、原始模式守卫,以及「在新 PTY 中 spawn 一个进程」的通用流程。

use std::fs::File;
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicI32, Ordering};

use nix::sys::termios::{self, SetArg, Termios};

use crate::error::{DoitError, Result};

/// 读取 fd 对应终端的窗口尺寸;非 tty 或失败时回退到 80x24。
pub fn get_winsize(fd: RawFd) -> libc::winsize {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    unsafe {
        libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws as *mut libc::winsize);
    }
    if ws.ws_row == 0 {
        ws.ws_row = 24;
    }
    if ws.ws_col == 0 {
        ws.ws_col = 80;
    }
    ws
}

/// 设置 fd 对应终端(通常是 PTY master)的窗口尺寸。
pub fn set_winsize(fd: RawFd, ws: &libc::winsize) {
    unsafe {
        libc::ioctl(fd, libc::TIOCSWINSZ, ws as *const libc::winsize);
    }
}

pub fn set_nonblocking(fd: RawFd) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags >= 0 {
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }
}

/// 直接向 fd 写入全部字节(阻塞语义,用于把过滤后的输出写回真实终端)。
pub fn write_fd(fd: RawFd, buf: &[u8]) {
    let mut off = 0;
    while off < buf.len() {
        let n = unsafe {
            libc::write(
                fd,
                buf[off..].as_ptr() as *const libc::c_void,
                buf.len() - off,
            )
        };
        if n <= 0 {
            break;
        }
        off += n as usize;
    }
}

/// 真实终端 raw 模式守卫:构造时进入 raw,析构时恢复原始 termios。
pub struct RawModeGuard {
    fd: RawFd,
    orig: Termios,
}

impl RawModeGuard {
    /// 对 fd 进入 raw 模式;若不是 tty 返回 `Ok(None)`(无需恢复)。
    pub fn new(fd: RawFd) -> Result<Option<Self>> {
        let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
        let orig = match termios::tcgetattr(borrowed) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        let mut raw = orig.clone();
        termios::cfmakeraw(&mut raw);
        termios::tcsetattr(borrowed, SetArg::TCSANOW, &raw)
            .map_err(|e| DoitError::shell(format!("tcsetattr raw: {e}")))?;
        Ok(Some(Self { fd, orig }))
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let borrowed = unsafe { BorrowedFd::borrow_raw(self.fd) };
        let _ = termios::tcsetattr(borrowed, SetArg::TCSANOW, &self.orig);
    }
}

// ---- SIGWINCH 自管道(线程安全,不依赖信号在特定线程递送)----

static SIGWINCH_WRITE_FD: AtomicI32 = AtomicI32::new(-1);

extern "C" fn handle_sigwinch(_: libc::c_int) {
    let fd = SIGWINCH_WRITE_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        let byte = [1u8];
        unsafe {
            libc::write(fd, byte.as_ptr() as *const libc::c_void, 1);
        }
    }
}

/// 安装 SIGWINCH 自管道:返回可读端 fd。处理器把 1 字节写入管道,供 poll 感知。
pub fn install_sigwinch_pipe() -> Result<RawFd> {
    let mut fds = [0i32; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(DoitError::shell("pipe for sigwinch"));
    }
    let (read_fd, write_fd) = (fds[0], fds[1]);
    set_nonblocking(read_fd);
    set_nonblocking(write_fd);
    SIGWINCH_WRITE_FD.store(write_fd, Ordering::Relaxed);

    let mut sa: libc::sigaction = unsafe { std::mem::zeroed() };
    sa.sa_sigaction = handle_sigwinch as extern "C" fn(libc::c_int) as usize;
    sa.sa_flags = libc::SA_RESTART;
    unsafe {
        libc::sigemptyset(&mut sa.sa_mask);
        if libc::sigaction(libc::SIGWINCH, &sa, std::ptr::null_mut()) != 0 {
            return Err(DoitError::shell("sigaction SIGWINCH"));
        }
    }
    Ok(read_fd)
}

/// 在新开的 PTY 中 spawn 一个进程,令 PTY slave 成为其控制终端。
/// 返回 (master 文件, master 原始 fd, 子进程句柄);父进程已关闭 slave。
pub fn spawn_on_pty(
    program: &str,
    args: &[String],
    envs: &[(&str, &str)],
    ws: &libc::winsize,
) -> Result<(File, RawFd, Child)> {
    let pty = nix::pty::openpty(ws, None).map_err(|e| DoitError::shell(format!("openpty: {e}")))?;
    let master_fd = pty.master.as_raw_fd();
    let slave_fd = pty.slave.as_raw_fd();

    let mut cmd = Command::new(program);
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }

    unsafe {
        cmd.pre_exec(move || {
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
            if slave_fd > 2 {
                libc::close(slave_fd);
            }
            libc::close(master_fd);
            Ok(())
        });
    }

    let child = cmd.spawn().map_err(|e| DoitError::io(e, "spawn on pty"))?;
    drop(pty.slave);
    set_nonblocking(master_fd);
    Ok((File::from(pty.master), master_fd, child))
}
