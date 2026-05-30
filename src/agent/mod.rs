pub mod shell;

use std::process::Command as StdCommand;

use crate::backend::DeepSeekBackend;
use crate::block::Block;
use crate::context::RuntimeContext;
use crate::error::Result;
use crate::session::Session;
use shell::ShellSession;

pub struct Agent {
    backend: DeepSeekBackend,
}

impl Agent {
    pub fn new(backend: DeepSeekBackend) -> Self {
        Self { backend }
    }

    /// 交互模式:先 prompt 取回用户的首个请求,再进入循环让 LLM 工作。
    pub async fn run_interactive(&self, ctx: &RuntimeContext, session: &mut Session) -> Result<()> {
        let system = self.generate_system_prompt(ctx, true).await?;
        session.append(Block::System {
            seq: session.next_seq(),
            content: system,
        })?;
        let mut shell = ShellSession::spawn(&session.dir)?;

        // 先向用户取回首个输入(空输入视为直接退出)
        let first = shell.prompt_input()?;
        if first.is_empty() {
            return Ok(());
        }
        session.append(Block::User {
            seq: session.next_seq(),
            content: first,
        })?;

        self.run_loop(session, &mut shell, true).await
    }

    /// 任务模式:系统提示尾部注入任务描述,LLM 直接执行直到 `doit exit`。
    pub async fn run_task(
        &self,
        ctx: &RuntimeContext,
        session: &mut Session,
        task: &str,
    ) -> Result<()> {
        let base = self.generate_system_prompt(ctx, false).await?;
        let content = format!("{base}\n\nTask: {task}\n\nExecute the assigned task.");
        session.append(Block::System {
            seq: session.next_seq(),
            content,
        })?;
        let mut shell = ShellSession::spawn(&session.dir)?;
        self.run_loop(session, &mut shell, false).await
    }

    /// 核心循环:send → parse → execute → repeat。
    ///
    /// LLM 的响应分两类:
    /// - 工具调用(sh)→ 直接命令执行,结果配对为 Tool block(满足 API 的 tool_call 配对约束);
    /// - 自由文本(无工具调用)→ 在交互模式下转换为一次 doit prompt 向用户取输入。
    async fn run_loop(
        &self,
        session: &mut Session,
        shell: &mut ShellSession,
        interactive: bool,
    ) -> Result<()> {
        loop {
            let messages = session.build_messages();
            let response = self.backend.chat(&messages).await?;
            let reasoning = response.reasoning.clone().unwrap_or_default();

            // —— 自由文本:LLM 在和用户对话 ——
            if response.is_prompt {
                let content = response.content.clone().unwrap_or_default();
                session.append(Block::Assistant {
                    seq: session.next_seq(),
                    reasoning,
                    cmd: String::new(),
                    tool_call_id: None,
                    content: Some(content.clone()),
                })?;

                if !interactive {
                    // 非交互模式无用户可问,LLM 未采取行动即结束
                    break;
                }

                // 直接把 LLM 的文本打印到终端(它在和用户说话),再裸 doit prompt 取输入
                print_to_terminal(&content);
                let reply = shell.prompt_input()?;
                if reply.is_empty() {
                    break; // 用户空输入 → 结束会话
                }
                session.append(Block::User {
                    seq: session.next_seq(),
                    content: reply,
                })?;
                continue;
            }

            // —— 工具调用:直接命令执行 ——
            let cmd = response.cmd.clone().unwrap_or_default();
            let tool_call_id = response.tool_call_id.clone().unwrap_or_default();
            session.append(Block::Assistant {
                seq: session.next_seq(),
                reasoning,
                cmd: cmd.clone(),
                tool_call_id: Some(tool_call_id.clone()),
                content: None,
            })?;

            let trimmed = cmd.trim_start();

            // LLM 手工调用 doit prompt:走对话路径(由 doit prompt 自己渲染输入框),不加 $ 前缀
            if trimmed.starts_with("doit prompt") {
                let reply = shell.run_prompt_command(&cmd)?;
                session.append(Block::Tool {
                    seq: session.next_seq(),
                    output: reply,
                    exit_code: 0,
                    tool_call_id,
                })?;
                continue;
            }

            // 其余命令:打印格式化的命令行($ + 宝石蓝竖条 + 着重底色),再执行
            print_command(&cmd);

            if trimmed.starts_with("doit exit") {
                let result = shell.run_command(&cmd)?;
                session.append(Block::Tool {
                    seq: session.next_seq(),
                    output: result.output,
                    exit_code: result.exit_code,
                    tool_call_id,
                })?;
                break;
            }

            let result = shell.run_command(&cmd)?;
            session.append(Block::Tool {
                seq: session.next_seq(),
                output: result.output,
                exit_code: result.exit_code,
                tool_call_id,
            })?;
        }
        Ok(())
    }

    /// 通过 `doit template system` 子进程生成系统提示(模式差异由 --interactive 控制)。
    async fn generate_system_prompt(
        &self,
        ctx: &RuntimeContext,
        interactive: bool,
    ) -> Result<String> {
        let exe = std::env::current_exe().map_err(|e| crate::error::DoitError::io(e, "exe"))?;
        let mut cmd = StdCommand::new(&exe);
        cmd.args(["template", "system"]);
        if interactive {
            cmd.arg("--interactive");
        }
        cmd.env("LANG", format!("{}.UTF-8", ctx.locale));
        let out = cmd
            .output()
            .map_err(|_| crate::error::DoitError::shell("template generation failed"))?;
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}

/// 把 LLM 的文本打印到真实终端(它在和用户说话)。终端处于 raw 模式(无 ONLCR),
/// 需手动 \n → \r\n。内容上下各空一行与上下文隔开,并铺浅橘色着重底色。
fn print_to_terminal(text: &str) {
    use std::io::Write;
    const BG: &str = "\x1b[48;2;55;41;25m"; // 浅橘色着重底色(略暗)
    const EOL: &str = "\x1b[K"; // 用当前底色填充至行尾
    const RESET: &str = "\x1b[0m";
    let blank = format!("{BG}{EOL}{RESET}\r\n"); // 着色的空行
    let mut out = blank.clone(); // 与上文隔一行(着色)
    for line in text.replace("\r\n", "\n").split('\n') {
        out.push_str(&format!("{BG}{line}{EOL}{RESET}\r\n"));
    }
    out.push_str(&blank); // 与下文隔一行(着色)
    print!("{out}");
    std::io::stdout().flush().ok();
}

/// 渲染将要执行的命令行:着重底色 + 左侧宝石蓝粗竖条 + `$ <命令>`。
/// 真实终端处于 raw 模式,使用 \r\n 换行;多行命令逐行加竖条。
fn print_command(cmd: &str) {
    use std::io::Write;
    const BAR: &str = "\x1b[38;2;29;112;233m"; // 宝石蓝竖条
    const BG: &str = "\x1b[48;2;38;43;59m"; // 着重底色
    const FG: &str = "\x1b[39m"; // 恢复默认前景(保留底色)
    const EOL: &str = "\x1b[K"; // 用当前底色填充至行尾
    const RESET: &str = "\x1b[0m";
    let mut out = String::new();
    for (i, line) in cmd.lines().enumerate() {
        let prefix = if i == 0 { "$ " } else { "  " };
        out.push_str(&format!("{BG}{BAR}▌{FG}{prefix}{line}{EOL}{RESET}\r\n"));
    }
    print!("{out}");
    std::io::stdout().flush().ok();
}
