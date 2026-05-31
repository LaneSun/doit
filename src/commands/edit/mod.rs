use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Target file to edit
    pub file: Option<PathBuf>,

    /// Line range to replace (e.g. "10:15"), 1-indexed, inclusive
    #[arg(long, value_name = "N:M", conflicts_with_all = ["regex", "replace"])]
    pub lines: Option<String>,

    /// Regular expression pattern for replacement
    #[arg(long)]
    pub regex: Option<String>,

    /// Replacement text for regex mode
    #[arg(long, requires = "regex")]
    pub replace: Option<String>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

const CONTEXT: usize = 3;

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("edit.skill"));
        return Ok(());
    }

    let file = args
        .file
        .as_ref()
        .ok_or_else(|| crate::error::DoitError::config("missing required argument: file"))?;

    let original = fs::read_to_string(file)
        .map_err(|e| crate::error::DoitError::io(e, format!("cannot read {}", file.display())))?;
    let original_lines: Vec<&str> = original.lines().collect();
    let file_name = file.display().to_string();

    if let Some(range) = &args.lines {
        let (start, end) = parse_range(range)?;
        let mut stdin = String::new();
        io::stdin()
            .read_to_string(&mut stdin)
            .map_err(|e| crate::error::DoitError::io(e, "failed to read stdin"))?;
        let new_content = stdin.trim_end().to_string();
        let (result_lines, changed_len) =
            do_line_replace(&original_lines, start, end, &new_content, file)?;
        let ctx_start = start.saturating_sub(CONTEXT);
        let ctx_end = (start + changed_len + CONTEXT).min(result_lines.len());
        for (i, line) in result_lines
            .iter()
            .enumerate()
            .take(ctx_end)
            .skip(ctx_start)
        {
            println!("{}:{}: {}", file_name, i + 1, line);
        }
    } else if let (Some(pattern), Some(replacement)) =
        (args.regex.as_deref(), args.replace.as_deref())
    {
        let re = regex::Regex::new(pattern)
            .map_err(|e| crate::error::DoitError::config(format!("invalid regex: {e}")))?;
        let result = do_regex_replace(&original_lines, &re, replacement, file)?;
        // Collect and merge changed ranges
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        for (i, (old, new)) in original_lines.iter().zip(result.iter()).enumerate() {
            if old != new {
                match ranges.last_mut() {
                    Some((_, end)) if *end + 1 + CONTEXT >= i => {
                        *end = i;
                    }
                    _ => ranges.push((i, i)),
                }
            }
        }
        // Print each merged region with context
        for (start, end) in &ranges {
            let ctx_start = (*start).saturating_sub(CONTEXT);
            let ctx_end = (end + 1 + CONTEXT).min(result.len());
            for i in ctx_start..ctx_end {
                if i >= *start && i <= *end {
                    println!("{}:{}: {}", file_name, i + 1, result[i]);
                } else {
                    println!("{}:{}: {}", file_name, i + 1, original_lines[i]);
                }
            }
        }
    } else {
        let mut stdin = String::new();
        io::stdin()
            .read_to_string(&mut stdin)
            .map_err(|e| crate::error::DoitError::io(e, "failed to read stdin"))?;
        let (result_lines, changed_start, changed_len) =
            do_diff_replace(&original_lines, &stdin, file)?;
        let ctx_start = changed_start.saturating_sub(CONTEXT);
        let ctx_end = (changed_start + changed_len + CONTEXT).min(result_lines.len());
        for (i, line) in result_lines
            .iter()
            .enumerate()
            .take(ctx_end)
            .skip(ctx_start)
        {
            println!("{}:{}: {}", file_name, i + 1, line);
        }
    }

    Ok(())
}

fn parse_range(s: &str) -> Result<(usize, usize)> {
    let (a, b) = s
        .split_once(':')
        .ok_or_else(|| crate::error::DoitError::config(format!("invalid range: {s}")))?;
    let start: usize = a
        .parse()
        .map_err(|_| crate::error::DoitError::config(format!("invalid start: {a}")))?;
    let end: usize = b
        .parse()
        .map_err(|_| crate::error::DoitError::config(format!("invalid end: {b}")))?;
    if start < 1 || end < 1 || start > end {
        return Err(crate::error::DoitError::config(format!(
            "invalid range: {s}"
        )));
    }
    Ok((start, end))
}

fn do_line_replace(
    lines: &[&str],
    start: usize,
    end: usize,
    new_content: &str,
    file: &PathBuf,
) -> Result<(Vec<String>, usize)> {
    let total = lines.len();
    if start > total {
        return Err(crate::error::DoitError::config(format!(
            "start line {} exceeds file length {}",
            start, total
        )));
    }
    let end = end.min(total);
    let new_lines: Vec<&str> = new_content.lines().collect();
    let changed_len = new_lines.len();

    let mut result: Vec<String> = lines
        .iter()
        .take(start - 1)
        .map(|s| s.to_string())
        .collect();
    for nl in &new_lines {
        result.push(nl.to_string());
    }
    for line in lines.iter().skip(end) {
        result.push(line.to_string());
    }
    fs::write(file, result.join("\n") + "\n")
        .map_err(|e| crate::error::DoitError::io(e, format!("cannot write {}", file.display())))?;

    Ok((result, changed_len))
}

fn do_regex_replace(
    lines: &[&str],
    re: &regex::Regex,
    replacement: &str,
    file: &PathBuf,
) -> Result<Vec<String>> {
    let new_lines: Vec<String> = lines
        .iter()
        .map(|line| re.replace_all(line, replacement).to_string())
        .collect();

    if new_lines.iter().zip(lines.iter()).all(|(n, o)| n == *o) {
        return Err(crate::error::DoitError::config(format!(
            "no matches found for pattern in {}",
            file.display()
        )));
    }

    fs::write(file, new_lines.join("\n") + "\n")
        .map_err(|e| crate::error::DoitError::io(e, format!("cannot write {}", file.display())))?;

    Ok(new_lines)
}

fn do_diff_replace(
    lines: &[&str],
    stdin: &str,
    file: &PathBuf,
) -> Result<(Vec<String>, usize, usize)> {
    let diff_lines: Vec<&str> = stdin.trim().lines().collect();

    let header_idx = diff_lines
        .iter()
        .position(|l| l.starts_with("@@"))
        .ok_or_else(|| crate::error::DoitError::config("invalid diff: missing @@ header"))?;

    let mut context_before: Vec<String> = Vec::new();
    let mut to_remove: Vec<String> = Vec::new();
    let mut to_add: Vec<String> = Vec::new();
    let mut changed_start: Option<usize> = None;

    // First pass: find the range of context + removed lines
    for line in diff_lines.iter().skip(header_idx + 1) {
        if let Some(c) = line.strip_prefix(' ') {
            if to_remove.is_empty() {
                context_before.push(c.to_string());
            }
        } else if let Some(c) = line.strip_prefix('-') {
            to_remove.push(c.to_string());
        } else if let Some(c) = line.strip_prefix('+') {
            to_add.push(c.to_string());
            if changed_start.is_none() {
                changed_start = Some(context_before.len());
            }
        }
    }

    if context_before.len() < CONTEXT {
        return Err(crate::error::DoitError::config(format!(
            "diff must have at least {} lines of context before changes",
            CONTEXT
        )));
    }

    // Search: match context_before + to_remove
    let search: Vec<&str> = context_before
        .iter()
        .chain(to_remove.iter())
        .map(|s| s.as_str())
        .collect();

    let search_len = search.len();
    let mut matches: Vec<usize> = Vec::new();
    for i in 0..(lines.len().saturating_sub(search_len - 1)) {
        if lines[i..i + search_len] == search[..] {
            matches.push(i);
        }
    }

    if matches.is_empty() {
        return Err(crate::error::DoitError::config(
            "diff context not found in file",
        ));
    }
    if matches.len() > 1 {
        return Err(crate::error::DoitError::config(format!(
            "found {} matching locations, diff must match exactly one",
            matches.len()
        )));
    }

    let ctx_start = matches[0];
    let remove_start = ctx_start + context_before.len();
    let remove_end = remove_start + to_remove.len();
    let changed_len = to_add.len();

    // Build new file
    let mut result: Vec<String> = lines
        .iter()
        .take(remove_start)
        .map(|s| s.to_string())
        .collect();
    for add in &to_add {
        result.push(add.clone());
    }
    for line in lines.iter().skip(remove_end) {
        result.push(line.to_string());
    }

    fs::write(file, result.join("\n") + "\n")
        .map_err(|e| crate::error::DoitError::io(e, format!("cannot write {}", file.display())))?;

    Ok((result, remove_start, changed_len))
}
