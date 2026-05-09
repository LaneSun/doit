use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Task description
    pub task: String,
}

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    tracing::info!("task: {}", args.task);
    todo!("task mode")
}
