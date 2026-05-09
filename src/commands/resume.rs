use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Session ID
    pub id: String,
}

pub async fn execute(_ctx: &RuntimeContext, args: &Args) -> Result<()> {
    tracing::info!("resume: {}", args.id);
    todo!("resume mode")
}
