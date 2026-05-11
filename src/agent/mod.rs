use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::process::Command as StdCommand;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::backend::DeepSeekBackend;
use crate::block::Block;
use crate::context::RuntimeContext;
use crate::error::Result;
use crate::session::Session;

pub struct Agent {
    backend: DeepSeekBackend,
}

struct CommandResult {
    output: String,
    exit_code: i32,
}

impl Agent {
    pub fn new(backend: DeepSeekBackend) -> Self {
        Self { backend }
    }

    pub async fn run_interactive(&self, ctx: &RuntimeContext, session: &mut Session) -> Result<()> {
        let system_content = self.generate_system_prompt(ctx, true).await?;
        session.append(Block::System { seq: session.next_seq(), content: system_content })?;
        self.run_loop(ctx, session).await
    }

    pub async fn run_task(&self, ctx: &RuntimeContext, session: &mut Session, task: &str) -> Result<()> {
        let base = self.generate_system_prompt(ctx, false).await?;
        let content = format!("{}\n\nTask: {task}\n\nExecute the assigned task.", base);
        session.append(Block::System { seq: session.next_seq(), content })?;
        self.run_loop(ctx, session).await
    }

    async fn run_loop(&self, _ctx: &RuntimeContext, session: &mut Session) -> Result<()> {
        loop {
            let messages = session.build_messages();
            let response = self.backend.chat(&messages).await?;

            // Parse response
            let (cmd, reasoning, raw_content, tool_call_id, is_prompt) = if response.is_prompt {
                let c = response.content.as_deref().unwrap_or("").to_string();
                ("doit prompt".to_string(), response.reasoning, Some(c), None, true)
            } else {
                let c = response.cmd.clone().unwrap_or_default();
                (c, response.reasoning, None, response.tool_call_id, false)
            };

            let tc_id = tool_call_id.clone();

            // Display command to terminal
            if is_prompt {
                let c = raw_content.as_deref().unwrap_or("");
                println!("$ doit prompt << 'EOF'");
                println!("{c}");
                println!("EOF");
            } else {
                println!("$ {cmd}");
            }
            io::stdout().flush().ok();

            // Append assistant block
            let assistant_block = Block::Assistant {
                seq: session.next_seq(),
                reasoning: reasoning.unwrap_or_default(),
                cmd: cmd.clone(),
                tool_call_id,
                content: raw_content.clone(),
            };
            session.append(assistant_block)?;

            // Exit check
            if cmd.starts_with("doit exit") {
                session.append(Block::Tool {
                    seq: session.next_seq(),
                    output: String::new(),
                    exit_code: 0,
                    tool_call_id: tc_id.unwrap_or_default(),
                })?;
                break;
            }

            // Execute via PTY
            let result = if is_prompt {
                let c = raw_content.unwrap_or_default();
                tokio::task::spawn_blocking(move || {
                    pty_exec(&["doit", "prompt", c.as_str()])
                }).await.map_err(|_| crate::error::DoitError::shell("join"))??
            } else if cmd.starts_with("doit ") {
                let args: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
                let (prog, rest) = (args[0].clone(), args[1..].to_vec());
                tokio::task::spawn_blocking(move || {
                    let a: Vec<&str> = rest.iter().map(|s| s.as_str()).collect();
                    pty_exec_direct(&prog, &a)
                }).await.map_err(|e| crate::error::DoitError::shell("join"))??
            } else {
                let s = format!("sh -c '{}'", cmd.replace('\'', "'\\''"));
                tokio::task::spawn_blocking(move || {
                    pty_exec_shell(&s)
                }).await.map_err(|e| crate::error::DoitError::shell("join"))??
            };

            // Append result block
            if is_prompt {
                session.append(Block::User {
                    seq: session.next_seq(),
                    content: result.output.trim().to_string(),
                })?;
            } else {
                session.append(Block::Tool {
                    seq: session.next_seq(),
                    output: result.output,
                    exit_code: result.exit_code,
                    tool_call_id: tc_id.unwrap_or_default(),
                })?;
            }
        }
        Ok(())
    }

    async fn generate_system_prompt(&self, ctx: &RuntimeContext, interactive: bool) -> Result<String> {
        let exe = std::env::current_exe()
            .map_err(|e| crate::error::DoitError::io(e, "exe"))?;
        let mut cmd = StdCommand::new(&exe);
        cmd.args(["template", "system"]);
        if interactive { cmd.arg("--interactive"); }
        cmd.env("LANG", format!("{}.UTF-8", ctx.locale));
        let out = cmd.output()
            .map_err(|e| crate::error::DoitError::shell("template failed"))?;
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}

// -- PTY execution --

fn pty_exec(args: &[&str]) -> Result<CommandResult> {
    pty_raw_mode(|| {
        let mut cmd = portable_pty::CommandBuilder::new(args[0]);
        for a in &args[1..] { cmd.arg(a); }
        cmd.cwd(std::env::current_dir().unwrap_or_default());
        pty_run(cmd)
    })
}

fn pty_exec_direct(prog: &str, args: &[&str]) -> Result<CommandResult> {
    pty_raw_mode(|| {
        let mut cmd = portable_pty::CommandBuilder::new(prog);
        for a in args { cmd.arg(a); }
        cmd.cwd(std::env::current_dir().unwrap_or_default());
        pty_run(cmd)
    })
}

fn pty_exec_shell(cmd_str: &str) -> Result<CommandResult> {
    pty_raw_mode(|| {
        let mut cmd = portable_pty::CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg(cmd_str);
        cmd.cwd(std::env::current_dir().unwrap_or_default());
        pty_run(cmd)
    })
}

fn pty_raw_mode<F: FnOnce() -> Result<CommandResult>>(f: F) -> Result<CommandResult> {
    let stdin_fd = io::stdin().as_raw_fd();
    let mut orig = std::mem::MaybeUninit::uninit();
    let ok = unsafe { libc::tcgetattr(stdin_fd, orig.as_mut_ptr()) == 0 };
    if ok {
        let mut raw = unsafe { orig.assume_init() };
        raw.c_lflag &= !(libc::ECHO | libc::ICANON);
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
        unsafe { libc::tcsetattr(stdin_fd, libc::TCSANOW, &raw); }
    }
    let r = f();
    if ok {
        let o = unsafe { orig.assume_init() };
        unsafe { libc::tcsetattr(stdin_fd, libc::TCSANOW, &o); }
    }
    r
}

fn pty_run(cmd: portable_pty::CommandBuilder) -> Result<CommandResult> {
    let sys = portable_pty::native_pty_system();
    let pair = sys.openpty(portable_pty::PtySize::default())
        .map_err(|e| crate::error::DoitError::shell("pty"))?;

    let mut child = pair.slave.spawn_command(cmd)
        .map_err(|e| crate::error::DoitError::shell("spawn"))?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()
        .map_err(|e| crate::error::DoitError::shell("reader"))?;
    let mut writer = pair.master.take_writer()
        .map_err(|e| crate::error::DoitError::shell("writer"))?;

    let capture = Arc::new(Mutex::new(Vec::new()));
    let cap2 = capture.clone();
    let mut out = io::stdout();

    let mut sig = [0i32; 2];
    unsafe { libc::pipe(sig.as_mut_ptr()); }
    let (sr, sw) = (sig[0], sig[1]);

    let h = thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => { unsafe { libc::write(sw, [1].as_ptr() as *const _, 1); } break; }
                Ok(n) => { out.write_all(&buf[..n]).ok(); out.flush().ok(); cap2.lock().unwrap().extend_from_slice(&buf[..n]); }
                Err(_) => break,
            }
        }
    });

    let stdin_fd = io::stdin().as_raw_fd();
    let mut buf = [0u8; 4096];
    loop {
        let mut fds = [
            libc::pollfd { fd: sr, events: libc::POLLIN, revents: 0 },
            libc::pollfd { fd: stdin_fd, events: libc::POLLIN, revents: 0 },
        ];
        unsafe { libc::poll(fds.as_mut_ptr(), 2, -1); }
        if fds[0].revents & libc::POLLIN != 0 { break; }
        if fds[1].revents & libc::POLLIN != 0 {
            match io::stdin().read(&mut buf) {
                Ok(0) => break,
                Ok(n) => { writer.write_all(&buf[..n]).ok(); writer.flush().ok(); }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(e) => { tracing::error!("stdin: {e}"); break; }
            }
        }
    }

    drop(writer);
    unsafe { libc::close(sw); libc::close(sr); }
    h.join().ok();

    let code = child.wait().map(|s| s.exit_code() as i32).unwrap_or(-1);
    let out = String::from_utf8_lossy(&capture.lock().unwrap()).to_string();
    Ok(CommandResult { output: out, exit_code: code })
}
