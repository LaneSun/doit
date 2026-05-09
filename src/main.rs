use doit::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "doit=info".into()),
        )
        .init();

    tracing::info!("doit started (skeleton)");

    Ok(())
}
