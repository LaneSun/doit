use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Target file path
    pub file: Option<PathBuf>,

    /// Append to file instead of overwriting
    #[arg(long, default_value_t = false)]
    pub append: bool,

    /// File permission mode in octal (e.g. 0o644)
    #[arg(long, value_name = "MODE")]
    pub mode: Option<String>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("write.skill"));
        return Ok(());
    }

    let file = args
        .file
        .as_ref()
        .ok_or_else(|| crate::error::DoitError::config("missing required argument: file"))?;

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| crate::error::DoitError::io(e, "failed to read stdin"))?;

    // Atomic write: write to temp file, then rename
    let tmp_path = file.with_extension("tmp");
    if args.append && file.exists() {
        let existing = fs::read_to_string(file).unwrap_or_default();
        let content = existing + &input;
        fs::write(&tmp_path, &content)
            .map_err(|e| crate::error::DoitError::io(e, format!("cannot write to {}", tmp_path.display())))?;
    } else {
        fs::write(&tmp_path, &input)
            .map_err(|e| crate::error::DoitError::io(e, format!("cannot write to {}", tmp_path.display())))?;
    }

    if let Some(mode_str) = &args.mode {
        set_mode(&tmp_path, mode_str)?;
    }

    fs::rename(&tmp_path, file)
        .map_err(|e| crate::error::DoitError::io(e, format!("cannot rename to {}", file.display())))?;

    Ok(())
}

fn set_mode(path: &std::path::Path, mode_str: &str) -> Result<()> {
    // Support both 0o644 and 644 formats
    let mode_str = mode_str.strip_prefix("0o").unwrap_or(mode_str);
    let mode = u32::from_str_radix(mode_str, 8)
        .map_err(|_| crate::error::DoitError::config(format!("invalid mode: {mode_str}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|e| crate::error::DoitError::io(e, "failed to set permissions"))?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
    Ok(())
}
