use std::io::IsTerminal;

use doit::context::RuntimeContext;
use doit::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = RuntimeContext {
        stderr_is_tty: std::io::stderr().is_terminal(),
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "doit=info".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(ctx.stderr_is_tty)
        .try_init()
        .expect("failed to initialize tracing subscriber");

    tracing::info!("doit started (skeleton)");

    Ok(())
}
