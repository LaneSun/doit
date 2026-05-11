use std::process::Command as StdCommand;

use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Template type: system
    pub template_type: String,

    /// Generate system prompt for interactive mode
    #[arg(long, default_value_t = false)]
    pub interactive: bool,
}

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    let key = match (args.template_type.as_str(), args.interactive) {
        ("system", true) => "template.system_interactive",
        ("system", false) => "template.system",
        _ => {
            return Err(crate::error::DoitError::config(format!(
                "unknown template type: {}",
                args.template_type
            )));
        }
    };

    let script = rust_i18n::t!(key);
    let current_exe = std::env::current_exe()
        .map_err(|e| crate::error::DoitError::io(e, "cannot find current executable"))?;

    let output = StdCommand::new("sh")
        .arg("-c")
        .arg(&*script)
        .env("DOIT_BIN", &current_exe)
        .output()
        .map_err(|e| crate::error::DoitError::shell(format!("template execution failed: {e}")))?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
        Ok(())
    } else {
        Err(crate::error::DoitError::shell(format!(
            "template script failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}
