use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Summary of the completed task
    pub summary: Option<String>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("exit.skill"));
        return Ok(());
    }

    if let Some(summary) = &args.summary {
        println!("{summary}");
    }
    Ok(())
}
