use std::io::Write;

use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Prompt message (not echoed, already visible as part of the command)
    pub message: Option<String>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

pub async fn execute(ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("prompt.skill"));
        return Ok(());
    }

    if !ctx.stdin_is_tty {
        eprintln!("{}", rust_i18n::t!("prompt.not_available"));
        return Ok(());
    }

    let input = read_input()?;

    if input.is_empty() {
        // Move cursor up one line, clear it, print > <ENTER>
        print!("\x1b[F> <ENTER>\n");
        std::io::stdout().flush().ok();
    }

    Ok(())
}

fn read_input() -> Result<String> {
    let mut rl = rustyline::DefaultEditor::new()
        .map_err(|e| crate::error::DoitError::shell(format!("rustyline init: {e}")))?;
    match rl.readline("> ") {
        Ok(line) => Ok(line.trim().to_string()),
        Err(rustyline::error::ReadlineError::Interrupted) => Ok(String::new()),
        Err(e) => Err(crate::error::DoitError::shell(format!(
            "readline: {e}"
        ))),
    }
}
