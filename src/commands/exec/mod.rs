use std::process::Command as StdCommand;

use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Disable output truncation
    #[arg(long, default_value_t = false)]
    pub no_truncate: bool,

    /// Override max chars for head/tail truncation (default: 2000)
    #[arg(long, value_name = "N")]
    pub truncate_chars: Option<usize>,

    /// Override max lines for head/tail truncation (default: 50)
    #[arg(long, value_name = "N")]
    pub truncate_lines: Option<usize>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,

    /// Shell command to execute (everything after --)
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

const DEFAULT_HEAD: usize = 2000;
const DEFAULT_LINES: usize = 50;

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("exec.skill"));
        return Ok(());
    }

    if args.command.is_empty() {
        return Err(crate::error::DoitError::config("no command specified"));
    }
    let (prog, cmd_args) = args.command.split_first().unwrap();

    let output = StdCommand::new(prog)
        .args(cmd_args)
        .output()
        .map_err(|e| crate::error::DoitError::shell(format!("spawn error: {e}")))?;

    let combined: Vec<u8> = [output.stdout, output.stderr].concat();

    let max_chars = args.truncate_chars.unwrap_or(DEFAULT_HEAD);
    let max_lines = args.truncate_lines.unwrap_or(DEFAULT_LINES);

    let text = String::from_utf8_lossy(&combined);

    if args.no_truncate || combined.is_empty() {
        print!("{}", text);
    } else {
        let lines: Vec<&str> = text.lines().collect();
        let total_lines = lines.len();
        let visible_chars = visible_len(&text);

        if total_lines <= max_lines * 2 && visible_chars <= max_chars * 2 {
            print!("{}", text);
        } else {
            // Head
            let head: String = build_head(&lines, max_lines, max_chars);
            // Tail
            let tail = build_tail(&lines, max_lines, max_chars);

            println!("{}", head);
            println!("\x1b[0m... [{}] [{}]",
                rust_i18n::t!("exec.truncated"),
                rust_i18n::t!("exec.truncated_hint")
            );
            if !tail.is_empty() {
                println!("\x1b[0m{}", tail);
            }
        }
    }

    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }

    Ok(())
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

fn visible_len(s: &str) -> usize {
    let re = regex::Regex::new("\x1b\\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").len()
}
