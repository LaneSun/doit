use std::fs;
use std::path::PathBuf;

use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Regular expression pattern to search for
    pub pattern: String,

    /// File glob to include (can be specified multiple times)
    #[arg(long, value_name = "GLOB")]
    pub include: Vec<String>,

    /// Base directory for search (defaults to current directory)
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

const MAX_MATCHES: usize = 500;

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("search.skill"));
        return Ok(());
    }

    let re = regex::Regex::new(&args.pattern)
        .map_err(|e| crate::error::DoitError::config(format!("invalid regex: {e}")))?;

    let default_cwd = PathBuf::from(".");
    let cwd = args.cwd.as_deref().unwrap_or(&default_cwd);
    let inc_patterns: Vec<&str> = if args.include.is_empty() {
        vec!["*"]
    } else {
        args.include.iter().map(|s| s.as_str()).collect()
    };

    let mut count = 0;
    for inc in &inc_patterns {
        if count > MAX_MATCHES {
            break;
        }
        let glob_pattern = cwd.join("**").join(inc);
        for entry in glob::glob(&glob_pattern.to_string_lossy())
            .map_err(|e| crate::error::DoitError::shell(format!("glob error: {e}")))?
        {
            if count > MAX_MATCHES {
                break;
            }
            let path = entry
                .map_err(|e| crate::error::DoitError::shell(format!("glob entry error: {e}")))?;
            if !path.is_file() {
                continue;
            }
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let file_display = path.strip_prefix(cwd).unwrap_or(&path);
            for (i, line) in content.lines().enumerate() {
                if count >= MAX_MATCHES {
                    break;
                }
                if re.is_match(line) {
                    println!("{}:{}: {}", file_display.display(), i + 1, line);
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        println!("{}", rust_i18n::t!("search.no_matches"));
    } else if count >= MAX_MATCHES {
        println!("{}", rust_i18n::t!("search.truncated"));
    }

    Ok(())
}
