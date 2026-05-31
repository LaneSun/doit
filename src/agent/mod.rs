pub mod shell;

use std::process::Command as StdCommand;

use crate::backend::DeepSeekBackend;
use crate::block::Block;
use crate::config::{Config, DisplayConfig};
use crate::context::RuntimeContext;
use crate::error::Result;
use crate::session::Session;
use shell::{CommandOutput, ShellSession};

pub struct Agent {
    backend: DeepSeekBackend,
    config: Config,
}

impl Agent {
    /// 从配置构造 Agent(后端、显示、提示覆盖等均取自 config)。
    pub fn from_config(config: &Config) -> Self {
        Self {
            backend: DeepSeekBackend::from_config(config),
            config: config.clone(),
        }
    }

    /// 交互模式:先 prompt 取回用户的首个请求,再进入循环让 LLM 工作。
    pub async fn run_interactive(&self, ctx: &RuntimeContext, session: &mut Session) -> Result<()> {
        let system = self.generate_system_prompt(ctx, true).await?;
        session.append(Block::System {
            seq: session.next_seq(),
            content: system,
        })?;
        let forward = self.config.display.show_command_output;
        let mut shell = ShellSession::spawn(&session.dir, true, forward)?;
        self.run_interactive_loop(session, &mut shell).await
    }

    /// 任务模式:系统提示尾部注入任务描述,LLM 直接执行直到 `doit exit`。
    pub async fn run_task(
        &self,
        ctx: &RuntimeContext,
        session: &mut Session,
        task: &str,
        verbosity: Verbosity,
    ) -> Result<()> {
        let base = self.generate_system_prompt(ctx, false).await?;
        let content = format!("{base}\n\nTask: {task}\n\nExecute the assigned task.");
        session.append(Block::System {
            seq: session.next_seq(),
            content,
        })?;
        let mut shell = ShellSession::spawn(&session.dir, false, false)?;
        // 任务模式无用户回合:LLM 持续行动直到 doit exit 或停止行动
        loop {
            let mut view = TaskView::new(verbosity);
            match self.llm_turn(session, &mut shell, &mut view).await? {
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
                let mut view = InteractiveView::new(&self.config.display);
                match self.llm_turn(session, shell, &mut view).await? {
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
    pub(crate) async fn llm_turn(
        &self,
        session: &mut Session,
        shell: &mut ShellSession,
        view: &mut dyn TurnView,
    ) -> Result<TurnOutcome> {
        let messages = session.build_messages();
        // 流式事件交给视图:交互式 raw 渲染 / 子 Agent 按档输出
        let response = self
            .backend
            .chat_stream(&messages, |ev| view.on_stream(ev))
            .await?;
        view.on_stream_end();
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

        // LLM 手工调用 doit prompt:视图可接管(web),否则由 doit prompt 子命令在 shell 中取输入(终端)
        if trimmed.starts_with("doit prompt") {
            let reply = match view.handle_prompt(&cmd) {
                Some(r) => r?,
                None => shell.run_prompt_command(&cmd)?,
            };
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
        view.on_command(&cmd, &result, is_exit);
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
    pub(crate) async fn generate_system_prompt(
        &self,
        ctx: &RuntimeContext,
        interactive: bool,
    ) -> Result<String> {
        let prompt = &self.config.prompt;
        // 完全覆盖(逃生舱):配置了 system_* 则直接使用,跳过 template 生成
        let (override_text, append_text) = if interactive {
            (&prompt.system_interactive, &prompt.append_interactive)
        } else {
            (&prompt.system_task, &prompt.append_task)
        };

        let mut base = match override_text {
            Some(text) => text.clone(),
            None => {
                let exe =
                    std::env::current_exe().map_err(|e| crate::error::DoitError::io(e, "exe"))?;
                let mut cmd = StdCommand::new(&exe);
                cmd.args(["template", "system"]);
                if interactive {
                    cmd.arg("--interactive");
                }
                cmd.env("LANG", format!("{}.UTF-8", ctx.locale));
                let out = cmd
                    .output()
                    .map_err(|_| crate::error::DoitError::shell("template generation failed"))?;
                String::from_utf8_lossy(&out.stdout).to_string()
            }
        };

        // 追加:在生成结果尾部加上项目自定义指令(注册表照常工作)
        if let Some(extra) = append_text
            && !extra.trim().is_empty()
        {
            base.push_str("\n\n");
            base.push_str(extra);
        }
        Ok(base)
    }
}

/// 子 Agent(doit task)的输出详细档位。
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Verbosity {
    /// 仅最终 exit 总结
    Result,
    /// 每条命令的 narration 行 + 最终总结
    Summary,
    /// 完整 transcript(narration + 命令 + 输出) + 最终总结
    Full,
}

/// 一个 LLM 回合的结果。
pub(crate) enum TurnOutcome {
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
    narration: String,    // 概述先到,缓冲至命令判定后再渲染或丢弃
    show_reasoning: bool, // 显隐:思维链
    show_narration: bool, // 显隐:命令概述
}

impl StreamRender {
    fn new(show_reasoning: bool, show_narration: bool) -> Self {
        Self {
            block: None,
            cmd: CommandFilter::Inactive,
            narration: String::new(),
            show_reasoning,
            show_narration,
        }
    }

    fn event(&mut self, ev: crate::backend::StreamEvent) {
        use crate::backend::StreamEvent;
        match ev {
            StreamEvent::Reasoning(s) => {
                if self.show_reasoning {
                    self.text(BlockKind::Reasoning, s)
                }
            }
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

    /// 把已缓冲的概述渲染为一个 # 块(与命令同样式),随后清空。显隐关闭时仅清空不渲染。
    fn flush_narration(&mut self) {
        if !self.narration.is_empty() {
            if self.show_narration {
                let mut nb = StreamBlock::new(BlockKind::Narration);
                nb.push(&self.narration);
                nb.finish();
            }
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
        if let CommandFilter::Buffering(buf) = &self.cmd
            && !buf.is_empty()
        {
            let buf = buf.clone();
            self.flush_narration();
            let mut block = StreamBlock::new(BlockKind::Command);
            block.push(&buf);
            self.block = Some(block);
        }
        self.narration.clear(); // 丢弃无对应命令的残留概述
        self.cmd = CommandFilter::Inactive;
        self.finish_block();
    }
}

/// 一个 LLM 回合的「输出视图」:把流式增量与命令结果导向不同呈现方式
/// —— 交互式 raw 终端渲染,或子 Agent(doit task)按 verbosity 的纯文本输出。
pub(crate) trait TurnView {
    /// LLM 流式增量(reasoning / narration / command)。
    fn on_stream(&mut self, ev: crate::backend::StreamEvent);
    /// 流式结束(收尾)。
    fn on_stream_end(&mut self);
    /// 一条命令执行完成。`is_exit` 表示该命令是 doit exit(其输出即最终总结)。
    fn on_command(&mut self, cmd: &str, out: &CommandOutput, is_exit: bool);
    /// LLM 发起 doit prompt 时由视图接管获取用户回复。
    /// 返回 `Some(reply)` 表示视图已处理(如 web:推送提示并等待 WS 回复);
    /// 返回 `None` 表示回退到在常驻 shell 中运行 doit prompt 子命令(终端默认)。
    fn handle_prompt(&mut self, _cmd: &str) -> Option<Result<String>> {
        None
    }
}

/// 交互式视图:复用流式 raw 渲染。命令输出已由常驻 shell 实时转发到终端,此处不再打印。
struct InteractiveView {
    render: StreamRender,
}

impl InteractiveView {
    fn new(display: &DisplayConfig) -> Self {
        Self {
            render: StreamRender::new(display.show_reasoning, display.show_narration),
        }
    }
}

impl TurnView for InteractiveView {
    fn on_stream(&mut self, ev: crate::backend::StreamEvent) {
        self.render.event(ev);
    }
    fn on_stream_end(&mut self) {
        self.render.finish();
    }
    fn on_command(&mut self, _cmd: &str, _out: &CommandOutput, _is_exit: bool) {}
}

/// 子 Agent 视图:按 verbosity 向 stdout 输出纯文本(无 ANSI,因输出会进入父 Agent 上下文)。
/// - Result:仅最终 exit 总结
/// - Summary:每回合 narration 行 + 最终总结
/// - Full:完整 narration + 命令 + 输出 + 最终总结
struct TaskView {
    verbosity: Verbosity,
    narration: String,
    content: String,
}

impl TaskView {
    fn new(verbosity: Verbosity) -> Self {
        Self {
            verbosity,
            narration: String::new(),
            content: String::new(),
        }
    }
}

impl TurnView for TaskView {
    fn on_stream(&mut self, ev: crate::backend::StreamEvent) {
        // narration(命令回合)与 content(自由文本收尾)进入输出;reasoning/command 增量忽略
        match ev {
            crate::backend::StreamEvent::Narration(s) => self.narration.push_str(s),
            crate::backend::StreamEvent::Content(s) => self.content.push_str(s),
            _ => {}
        }
    }

    fn on_stream_end(&mut self) {
        // 命令回合:summary/full 打印 narration 行
        if !self.narration.is_empty()
            && matches!(self.verbosity, Verbosity::Summary | Verbosity::Full)
        {
            println!("# {}", self.narration.trim());
        }
        // 文本回合:LLM 以自由文本收尾(未走 doit exit),该文本即最终输出,所有档都打印
        if !self.content.is_empty() {
            println!("{}", self.content.trim());
        }
        self.narration.clear();
        self.content.clear();
    }

    fn on_command(&mut self, cmd: &str, out: &CommandOutput, is_exit: bool) {
        match self.verbosity {
            Verbosity::Full => {
                println!("$ {cmd}");
                let o = out.output.trim_end();
                if !o.is_empty() {
                    println!("{o}");
                }
            }
            // result / summary:仅把最终 exit 的总结作为结果输出
            Verbosity::Summary | Verbosity::Result => {
                if is_exit {
                    let s = out.output.trim();
                    if !s.is_empty() {
                        println!("{s}");
                    }
                }
            }
        }
    }
}
