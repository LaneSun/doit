use doit::error::Result;
use std::io::IsTerminal;

#[tokio::main]
async fn main() -> Result<()> {
    let is_tty = std::io::stderr().is_terminal();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "doit=info".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(is_tty)
        .try_init()
        .expect("failed to initialize tracing subscriber");

    tracing::info!("doit started (skeleton)");

    Ok(())
}
