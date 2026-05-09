use std::io::IsTerminal;

use doit::context::RuntimeContext;
use doit::error::Result;
use doit::i18n::detect_locale;
use rust_i18n::t;

rust_i18n::i18n!("locales");

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = RuntimeContext {
        stderr_is_tty: std::io::stderr().is_terminal(),
        locale: detect_locale(),
    };
    rust_i18n::set_locale(ctx.locale);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "doit=info".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(ctx.stderr_is_tty)
        .try_init()
        .expect("failed to initialize tracing subscriber");

    tracing::info!("{}", t!("tracing.startup"));

    Ok(())
}
