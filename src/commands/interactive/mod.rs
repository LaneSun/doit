use crate::agent::Agent;
use crate::backend::DeepSeekBackend;
use crate::context::RuntimeContext;
use crate::error::Result;
use crate::session::Session;

#[derive(clap::Args)]
pub struct Args {}

pub async fn execute(ctx: &RuntimeContext, _args: &Args) -> Result<()> {
    tracing::debug!("starting interactive REPL");

    let api_key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default();
    let backend = DeepSeekBackend::new(
        "https://api.deepseek.com".to_string(),
        api_key,
        "deepseek-v4-pro".to_string(),
    );

    let agent = Agent::new(backend);

    let mut session = Session::create(&std::env::current_dir().unwrap(), "deepseek-v4-pro")?;

    agent.run_interactive(ctx, &mut session).await
}
