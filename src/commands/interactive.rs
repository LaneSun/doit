use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {}

pub async fn execute(_ctx: &RuntimeContext, _args: &Args) -> Result<()> {
    tracing::info!("starting interactive REPL");
    todo!("interactive REPL")
}
