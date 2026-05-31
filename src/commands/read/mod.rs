use std::fs;
use std::path::PathBuf;

use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// File to read
    pub file: Option<PathBuf>,

    /// Line range (e.g. "10:15"), 1-indexed, inclusive. Disables default truncation.
    #[arg(long, value_name = "N:M")]
    pub lines: Option<String>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("read.skill"));
        return Ok(());
    }

    let file = args
        .file
        .as_ref()
        .ok_or_else(|| crate::error::DoitError::config("missing required argument: file"))?;

    let content = fs::read_to_string(file)
        .map_err(|e| crate::error::DoitError::io(e, format!("cannot open {}", file.display())))?;
    let all_lines: Vec<&str> = content.lines().collect();
    let total = all_lines.len();

    let (start, end, limit) = if let Some(range) = &args.lines {
        let (s, e) = parse_range(range)?;
        (s, e, None)
    } else {
        (1, total, Some(500))
    };

    let start = start.min(total.max(1));
    let end = end.min(total);

    let mut count = 0;
    for (i, line) in all_lines.iter().enumerate() {
        let line_num = i + 1;
        if line_num < start {
            continue;
        }
        if line_num > end {
            break;
        }
        if let Some(limit) = limit
            && count >= limit
        {
            let hidden = (end - start + 1).saturating_sub(limit);
            if hidden > 0 {
                let msg = rust_i18n::t!("read.truncated");
                println!("{}", msg.replace("%{hidden}", &hidden.to_string()));
            }
            break;
        }
        println!("{}: {}", line_num, line);
        count += 1;
    }

    Ok(())
}

fn parse_range(s: &str) -> Result<(usize, usize)> {
    let (a, b) = s
        .split_once(':')
        .ok_or_else(|| crate::error::DoitError::config(format!("invalid range: {s}")))
        .and_then(|(a, b)| {
            let start: usize = a
                .parse()
                .map_err(|_| crate::error::DoitError::config(format!("invalid start: {a}")))?;
            let end: usize = b
                .parse()
                .map_err(|_| crate::error::DoitError::config(format!("invalid end: {b}")))?;
            if start < 1 || end < 1 {
                return Err(crate::error::DoitError::config(
                    "line numbers must be >= 1".to_string(),
                ));
            }
            if start > end {
                return Err(crate::error::DoitError::config(
                    "start must be <= end".to_string(),
                ));
            }
            Ok((start, end))
        })?;
    Ok((a, b))
}
