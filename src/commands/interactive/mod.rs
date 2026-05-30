use crate::agent::Agent;
use crate::context::RuntimeContext;
use crate::error::Result;
use crate::session::Session;

#[derive(clap::Args)]
pub struct Args {}

pub async fn execute(ctx: &RuntimeContext, _args: &Args) -> Result<()> {
    tracing::debug!("starting interactive REPL");

    let agent = Agent::from_config(&ctx.config);
    let mut session = Session::create(&std::env::current_dir().unwrap(), &ctx.config.model.name)?;

    agent.run_interactive(ctx, &mut session).await
}
