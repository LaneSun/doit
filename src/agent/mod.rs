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
        self.run_interactive_loop(session, &mut shell).await
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
        // 任务模式无用户回合:LLM 持续行动直到 doit exit 或停止行动
        loop {
            match self.llm_turn(session, &mut shell).await? {
                TurnOutcome::Continue => continue,
                TurnOutcome::AwaitUser | TurnOutcome::Exit => break,
            }
        }
        Ok(())
    }

    /// 交互循环:用户回合与 LLM 回合交替。用户单独输入 `$` 在「对话」与「命令」模式间切换。
    /// 命令模式下用户输入直接在常驻 shell 执行(状态保持),并记入会话上下文供 LLM 知晓。
    async fn run_interactive_loop(
        &self,
        session: &mut Session,
        shell: &mut ShellSession,
    ) -> Result<()> {
        let mut shell_mode = false;
        loop {
            // —— 用户回合 ——
            let input = shell.prompt_input(shell_mode)?;
            if input.trim() == "$" {
                shell_mode = !shell_mode; // $ 双向切换模式
                continue;
            }

            if shell_mode {
                // 命令模式:空行仅重新提示,否则直接执行并记入上下文
                if input.is_empty() {
                    continue;
                }
                render_command_line(&input);
                let result = shell.run_command(&input)?;
                session.append(Block::User {
                    seq: session.next_seq(),
                    content: format_manual_command(&input, &result.output, result.exit_code),
                })?;
                continue;
            }

            // 对话模式:空输入结束会话,否则交给 LLM
            if input.is_empty() {
                break;
            }
            session.append(Block::User {
                seq: session.next_seq(),
                content: input,
            })?;

            // —— LLM 回合:持续行动直到需要用户输入或退出 ——
            loop {
                match self.llm_turn(session, shell).await? {
                    TurnOutcome::Continue => continue,
                    TurnOutcome::AwaitUser => break,
                    TurnOutcome::Exit => return Ok(()),
                }
            }
        }
        Ok(())
    }

    /// 一个 LLM 回合:流式拿到响应并据类型处理。
    /// - 自由文本 → 记录 Assistant 内容,返回 AwaitUser(等待用户输入);
    /// - 工具调用 → 执行命令,结果配对为 Tool block;`doit exit` 返回 Exit,其余返回 Continue。
    async fn llm_turn(
        &self,
        session: &mut Session,
        shell: &mut ShellSession,
    ) -> Result<TurnOutcome> {
        let messages = session.build_messages();
        // 流式渲染:思维(灰)、内容(橘块)、命令($ 竖条)边收边显示
        let mut ui = StreamRender::new();
        let response = self
            .backend
            .chat_stream(&messages, |ev| ui.event(ev))
            .await?;
        ui.finish();
        let reasoning = response.reasoning.clone().unwrap_or_default();

        if response.is_prompt {
            let content = response.content.clone().unwrap_or_default();
            session.append(Block::Assistant {
                seq: session.next_seq(),
                reasoning,
                narration: String::new(),
                cmd: String::new(),
                tool_call_id: None,
                content: Some(content),
            })?;
            return Ok(TurnOutcome::AwaitUser);
        }

        let cmd = response.cmd.clone().unwrap_or_default();
        let narration = response.narration.clone().unwrap_or_default();
        let tool_call_id = response.tool_call_id.clone().unwrap_or_default();
        session.append(Block::Assistant {
            seq: session.next_seq(),
            reasoning,
            narration,
            cmd: cmd.clone(),
            tool_call_id: Some(tool_call_id.clone()),
            content: None,
        })?;

        let trimmed = cmd.trim_start();

        // LLM 手工调用 doit prompt:由 doit prompt 自己渲染输入框并取回输入
        if trimmed.starts_with("doit prompt") {
            let reply = shell.run_prompt_command(&cmd)?;
            session.append(Block::Tool {
                seq: session.next_seq(),
                output: reply,
                exit_code: 0,
                tool_call_id,
            })?;
            return Ok(TurnOutcome::Continue);
        }

        let is_exit = trimmed.starts_with("doit exit");
        let result = shell.run_command(&cmd)?;
        session.append(Block::Tool {
            seq: session.next_seq(),
            output: result.output,
            exit_code: result.exit_code,
            tool_call_id,
        })?;
        Ok(if is_exit {
            TurnOutcome::Exit
        } else {
            TurnOutcome::Continue
        })
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

/// 一个 LLM 回合的结果。
enum TurnOutcome {
    /// LLM 还在行动(执行了命令),继续下一回合
    Continue,
    /// LLM 停止行动,等待用户输入
    AwaitUser,
    /// 会话结束(doit exit)
    Exit,
}

/// 渲染一条完整命令行(用户手动执行的命令,非流式)。复用流式命令块的样式。
fn render_command_line(cmd: &str) {
    let mut block = StreamBlock::new(BlockKind::Command);
    block.push(cmd);
    block.finish();
}

/// 把用户手动执行的命令及其输出格式化为一条 User 消息,供 LLM 了解用户的操作。
fn format_manual_command(cmd: &str, output: &str, exit_code: i32) -> String {
    format!("（用户手动执行）$ {cmd}\n{output}\n[exit code: {exit_code}]")
}

const GRAY: &str = "\x1b[38;2;128;128;128m"; // 思维链灰
const CONTENT_BG: &str = "\x1b[48;2;55;41;25m"; // 对话内容浅橘着重底色
const CMD_BG: &str = "\x1b[48;2;38;43;59m"; // 命令行着重底色
const CMD_BAR: &str = "\x1b[38;2;29;112;233m"; // 命令行宝石蓝竖条
const DEFAULT_FG: &str = "\x1b[39m"; // 恢复默认前景(保留底色)
const EOL: &str = "\x1b[K"; // 用当前底色填充至行尾
const RESET: &str = "\x1b[0m";

#[derive(Clone, Copy, PartialEq)]
enum BlockKind {
    Reasoning,
    Content,
    Narration,
    Command,
}

/// 单个流式块的逐行渲染状态(真实终端处于 raw 模式,需手动 \n → \r\n)。
struct StreamBlock {
    kind: BlockKind,
    line_no: usize,
    line_open: bool,
    leading_done: bool, // Content 块的前导着色空行是否已输出
}

impl StreamBlock {
    fn new(kind: BlockKind) -> Self {
        Self {
            kind,
            line_no: 0,
            line_open: false,
            leading_done: false,
        }
    }

    fn prefix(&self) -> String {
        match self.kind {
            BlockKind::Reasoning => GRAY.to_string(),
            BlockKind::Content => CONTENT_BG.to_string(),
            // 概述与命令同样式(着重底色 + 宝石蓝竖条),首行符号区分:# / $
            BlockKind::Narration => {
                let p = if self.line_no == 0 { "# " } else { "  " };
                format!("{CMD_BG}{CMD_BAR}▌{DEFAULT_FG}{p}")
            }
            BlockKind::Command => {
                let p = if self.line_no == 0 { "$ " } else { "  " };
                format!("{CMD_BG}{CMD_BAR}▌{DEFAULT_FG}{p}")
            }
        }
    }

    fn suffix(&self) -> &'static str {
        match self.kind {
            BlockKind::Reasoning => RESET,
            BlockKind::Content | BlockKind::Narration | BlockKind::Command => "\x1b[K\x1b[0m",
        }
    }

    fn start_line(&mut self, out: &mut String) {
        if self.kind == BlockKind::Content && !self.leading_done {
            out.push_str(&format!("{CONTENT_BG}{EOL}{RESET}\r\n")); // 前导着色空行
            self.leading_done = true;
        }
        out.push_str(&self.prefix());
        self.line_open = true;
    }

    fn end_line(&mut self, out: &mut String) {
        out.push_str(self.suffix());
        out.push_str("\r\n");
        self.line_open = false;
        self.line_no += 1;
    }

    fn push(&mut self, text: &str) {
        use std::io::Write;
        let mut out = String::new();
        for ch in text.chars() {
            if ch == '\r' {
                continue;
            }
            if !self.line_open {
                self.start_line(&mut out);
            }
            if ch == '\n' {
                self.end_line(&mut out);
            } else {
                out.push(ch);
            }
        }
        print!("{out}");
        std::io::stdout().flush().ok();
    }

    fn finish(&mut self) {
        use std::io::Write;
        let mut out = String::new();
        if self.line_open {
            self.end_line(&mut out);
        }
        if self.kind == BlockKind::Content {
            out.push_str(&format!("{CONTENT_BG}{EOL}{RESET}\r\n")); // 尾随着色空行
        }
        print!("{out}");
        std::io::stdout().flush().ok();
    }
}

/// 命令流的过滤状态:用于在流式早期判断命令是否为 `doit prompt`。
/// `doit prompt` 不渲染成 `$ ...` 命令行(其消息由 doit prompt 自身渲染成内容块,
/// 与 content 路径外观统一);其余命令照常流式显示。
enum CommandFilter {
    Inactive,
    Buffering(String), // 累积中,尚未判定
    Streaming,         // 已判定为普通命令,逐字流式
    Suppressed,        // 已判定为 doit prompt,不渲染
}

/// 跨流式块的渲染器:按事件类型切换块,切换时收尾上一块。
struct StreamRender {
    block: Option<StreamBlock>,
    cmd: CommandFilter,
    narration: String, // 概述先到,缓冲至命令判定后再渲染或丢弃
}

impl StreamRender {
    fn new() -> Self {
        Self {
            block: None,
            cmd: CommandFilter::Inactive,
            narration: String::new(),
        }
    }

    fn event(&mut self, ev: crate::backend::StreamEvent) {
        use crate::backend::StreamEvent;
        match ev {
            StreamEvent::Reasoning(s) => self.text(BlockKind::Reasoning, s),
            StreamEvent::Content(s) => self.text(BlockKind::Content, s),
            StreamEvent::Narration(s) => self.narration(s),
            StreamEvent::Command(s) => self.command(s),
        }
    }

    /// reasoning / content:按 kind 切换块并推入文本。
    fn text(&mut self, kind: BlockKind, s: &str) {
        if self.block.as_ref().map(|b| b.kind) != Some(kind) {
            self.finish_block();
            self.block = Some(StreamBlock::new(kind));
        }
        if let Some(b) = self.block.as_mut() {
            b.push(s);
        }
    }

    /// 概述增量:先于命令到达。首次到达时收尾上一块(如 reasoning),仅缓冲不渲染,
    /// 待命令判定后再决定渲染(普通命令)或丢弃(doit prompt)。
    fn narration(&mut self, s: &str) {
        if self.narration.is_empty() {
            self.finish_block();
        }
        self.narration.push_str(s);
    }

    /// 把已缓冲的概述渲染为一个 # 块(与命令同样式),随后清空。
    fn flush_narration(&mut self) {
        if !self.narration.is_empty() {
            let mut nb = StreamBlock::new(BlockKind::Narration);
            nb.push(&self.narration);
            nb.finish();
            self.narration.clear();
        }
    }

    /// 命令增量:带前缀判定的缓冲,隐藏 doit prompt、流式其余命令。
    fn command(&mut self, s: &str) {
        match &mut self.cmd {
            CommandFilter::Suppressed => {}
            CommandFilter::Streaming => {
                if let Some(b) = self.block.as_mut() {
                    b.push(s);
                }
            }
            CommandFilter::Buffering(buf) => {
                buf.push_str(s);
                self.decide();
            }
            CommandFilter::Inactive => {
                self.finish_block(); // 收尾之前的 reasoning 块
                self.cmd = CommandFilter::Buffering(s.to_string());
                self.decide();
            }
        }
    }

    /// 依据已缓冲文本判定命令类型。
    fn decide(&mut self) {
        const TARGET: &str = "doit prompt";
        let buf = match &self.cmd {
            CommandFilter::Buffering(b) => b.clone(),
            _ => return,
        };
        let t = buf.trim_start();
        if t == TARGET || t.starts_with("doit prompt ") {
            // doit prompt:不渲染命令行,概述一并丢弃(消息由 doit prompt 渲染成橘块)
            self.narration.clear();
            self.cmd = CommandFilter::Suppressed;
        } else if TARGET.starts_with(t) {
            // 仍是 "doit prompt" 的前缀,继续缓冲等待
        } else {
            // 普通命令:先渲染概述行,再渲染已缓冲命令文本并转入流式
            self.flush_narration();
            let mut block = StreamBlock::new(BlockKind::Command);
            block.push(&buf);
            self.block = Some(block);
            self.cmd = CommandFilter::Streaming;
        }
    }

    fn finish_block(&mut self) {
        if let Some(mut b) = self.block.take() {
            b.finish();
        }
    }

    fn finish(&mut self) {
        // 未判定的缓冲(命令在判定前结束)按普通命令渲染
        if let CommandFilter::Buffering(buf) = &self.cmd {
            if !buf.is_empty() {
                let buf = buf.clone();
                self.flush_narration();
                let mut block = StreamBlock::new(BlockKind::Command);
                block.push(&buf);
                self.block = Some(block);
            }
        }
        self.narration.clear(); // 丢弃无对应命令的残留概述
        self.cmd = CommandFilter::Inactive;
        self.finish_block();
    }
}
