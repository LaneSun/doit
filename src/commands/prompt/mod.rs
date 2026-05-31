//! `doit prompt`:向用户展示一段消息(可选)并读取输入,基于 reedline 提供完整行编辑。
//!
//! - 消息来自可选位置参数;给出时本命令打印它(像普通命令一样显示其「输出」)。
//! - 输入框:橘黄分隔线 + reedline 输入(提示符 ` > `,命令模式 `--shell` 时为 ` $ `)+ 橘黄分隔线。
//! - reedline 自带:全角字符宽度、左右光标移动、按词移动/删除(Alt+方向键、Alt+Backspace)、
//!   历史(落盘会话目录)。Alt+Enter 插入换行(续行无前缀),Enter 提交。
//! - stdin 非 tty(被重定向/测试环境)时优雅降级,绝不阻塞。
//! - 读到的输入写入 `$DOIT_SESSION_DIR/.prompt_reply`,供父 agent 确定性取回。

use std::borrow::Cow;
use std::fs;
use std::io::{self, IsTerminal};
use std::os::fd::AsRawFd;

use reedline::{
    EditCommand, Emacs, FileBackedHistory, KeyCode, KeyModifiers, Prompt, PromptEditMode,
    PromptHistorySearch, Reedline, ReedlineEvent, Signal, default_emacs_keybindings,
};

use crate::context::RuntimeContext;
use crate::error::Result;

/// 橘黄色分隔线前景色。
const ORANGE: &str = "\x1b[38;2;235;145;30m";
const RESET: &str = "\x1b[0m";
/// 内容块浅橘着重底色(与 agent 渲染 LLM content 时一致),用于显示 prompt 消息。
const CONTENT_BG: &str = "\x1b[48;2;55;41;25m";
const EOL: &str = "\x1b[K";

/// 父 agent 取回用户输入的文件名(位于会话目录下)。
pub const REPLY_FILE: &str = ".prompt_reply";

#[derive(clap::Args)]
pub struct Args {
    /// Prompt message to display before reading input
    pub message: Option<String>,

    /// Use the command-mode prompt indicator (` $ ` instead of ` > `)
    #[arg(long, default_value_t = false)]
    pub shell: bool,

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

    // 可选消息(LLM 手工 prompt 的提问)渲染为橘色内容块,与 content 路径外观一致
    if let Some(message) = &args.message
        && !message.is_empty()
    {
        render_content_block(message);
    }

    print_divider();
    let input = read_line(args.shell);
    print_divider();

    write_reply(&input);
    Ok(())
}

/// 用 reedline 读取一行(可多行)输入。Ctrl-C/Ctrl-D/出错均返回空串。
fn read_line(shell_mode: bool) -> String {
    let prompt = DoitPrompt {
        indicator: if shell_mode { " $ " } else { " > " },
    };

    // Alt+Enter 插入换行(其余 Emacs 键位:按词移动/删除、行首尾等均为默认)
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
    let edit_mode = Box::new(Emacs::new(keybindings));

    let mut editor = Reedline::create().with_edit_mode(edit_mode);

    // 跨提示符的历史(对话/命令各自落盘到会话目录)
    let hist_file = if shell_mode {
        ".shell_history"
    } else {
        ".prompt_history"
    };
    let hist_path = crate::session::resolve_session_dir().join(hist_file);
    if let Ok(history) = FileBackedHistory::with_file(1000, hist_path) {
        editor = editor.with_history(Box::new(history));
    }

    match editor.read_line(&prompt) {
        Ok(Signal::Success(buffer)) => buffer,
        _ => String::new(), // Ctrl-C / Ctrl-D / 错误
    }
}

/// 极简提示符:仅左侧指示符(` > ` / ` $ `),续行与右侧无任何内容。
struct DoitPrompt {
    indicator: &'static str,
}

impl Prompt for DoitPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed(self.indicator)
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("") // 续行无前缀
    }
    fn render_prompt_history_search_indicator(
        &self,
        _history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
}

/// 把消息渲染成橘色内容块:上下各一行着色空行,逐行铺底色填满整行。
/// stdout 是 cooked PTY(ONLCR 开),用 `\n` 即可,无需手动 \r\n。
fn render_content_block(text: &str) {
    let blank = format!("{CONTENT_BG}{EOL}{RESET}");
    println!("{blank}");
    for line in text.replace("\r\n", "\n").split('\n') {
        println!("{CONTENT_BG}{line}{EOL}{RESET}");
    }
    println!("{blank}");
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

/// 将用户输入写入会话目录的 reply 文件,供父 agent 取回。
fn write_reply(input: &str) {
    let path = crate::session::resolve_session_dir().join(REPLY_FILE);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, input);
}
