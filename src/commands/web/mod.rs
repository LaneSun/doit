use crate::context::RuntimeContext;
use crate::error::Result;

#[derive(clap::Args)]
pub struct Args {
    /// Host/IP to bind (use 0.0.0.0 to expose on LAN)
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to listen on (0 = random free port)
    #[arg(long, default_value_t = 0)]
    pub port: u16,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

pub async fn execute(ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("web.skill"));
        return Ok(());
    }
    // RuntimeContext 需移动进 Agent 线程,这里克隆其字段重建一份
    let owned = RuntimeContext {
        stdin_is_tty: ctx.stdin_is_tty,
        stderr_is_tty: ctx.stderr_is_tty,
        locale: ctx.locale,
        config: ctx.config.clone(),
    };
    crate::web::run(owned, &args.host, args.port).await
}
