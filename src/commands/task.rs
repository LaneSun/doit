use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Task description
    pub task: Option<String>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("task.skill"));
        return Ok(());
    }

    let task = args
        .task
        .as_ref()
        .ok_or_else(|| crate::error::DoitError::config("missing required argument: task"))?;

    tracing::info!("task: {}", task);
    todo!("task mode")
}
