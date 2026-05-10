use std::path::PathBuf;

use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Glob pattern (e.g. "src/**/*.rs")
    pub pattern: Option<String>,

    /// Base directory for matching (defaults to current directory)
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("glob.skill"));
        return Ok(());
    }

    let pattern_str = args
        .pattern
        .as_ref()
        .ok_or_else(|| crate::error::DoitError::config("missing required argument: pattern"))?;

    let default_cwd = PathBuf::from(".");
    let cwd = args.cwd.as_deref().unwrap_or(&default_cwd);
    let pattern = cwd.join(pattern_str);
    let pattern_str = pattern.to_string_lossy();

    for entry in glob::glob(&pattern_str)
        .map_err(|e| crate::error::DoitError::shell(format!("glob pattern error: {e}")))?
    {
        match entry {
            Ok(path) => {
                // Output relative path
                let relative = path
                    .strip_prefix(cwd)
                    .unwrap_or(&path);
                println!("{}", relative.display());
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }

    Ok(())
}
