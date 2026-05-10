use std::io::IsTerminal;

use clap::Parser;
use doit::cli::{Cli, Command};
use doit::commands;
use doit::context::RuntimeContext;
use doit::error::Result;
use doit::i18n::detect_locale;
use rust_i18n::t;

rust_i18n::i18n!("locales");

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = RuntimeContext {
        stdin_is_tty: std::io::stdin().is_terminal(),
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

    let cli = Cli::parse();

    match cli.command {
        None => commands::interactive::execute(&ctx, &commands::interactive::Args {}).await,
        Some(Command::Edit(args)) => commands::edit::execute(&ctx, &args).await,
        Some(Command::Exec(args)) => commands::exec::execute(&ctx, &args).await,
        Some(Command::Exit(args)) => commands::exit::execute(&ctx, &args).await,
        Some(Command::Glob(args)) => commands::glob::execute(&ctx, &args).await,
        Some(Command::Prompt(args)) => commands::prompt::execute(&ctx, &args).await,
        Some(Command::Read(args)) => commands::read::execute(&ctx, &args).await,
        Some(Command::Search(args)) => commands::search::execute(&ctx, &args).await,
        Some(Command::Interactive(args)) => commands::interactive::execute(&ctx, &args).await,
        Some(Command::Run(args)) => commands::run::execute(&ctx, &args).await,
        Some(Command::Task(args)) => commands::task::execute(&ctx, &args).await,
        Some(Command::Write(args)) => commands::write::execute(&ctx, &args).await,
        Some(Command::Resume(args)) => commands::resume::execute(&ctx, &args).await,
    }
}
