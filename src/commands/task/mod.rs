use crate::agent::{Agent, Verbosity};
use crate::context::RuntimeContext;
use crate::error::Result;
use crate::session::Session;

#[derive(clap::Args)]
pub struct Args {
    /// Task description
    pub task: Option<String>,

    /// Output verbosity: result | summary | full
    #[arg(long, short = 'v', value_enum, default_value_t = Verbosity::Summary)]
    pub verbosity: Verbosity,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

pub async fn execute(ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("task.skill"));
        return Ok(());
    }

    // 任务描述:优先位置参数,否则从 stdin 读(支持 heredoc / 管道)
    let task = match &args.task {
        Some(t) => t.clone(),
        None => {
            use std::io::Read;
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).ok();
            s.trim().to_string()
        }
    };
    if task.is_empty() {
        return Err(crate::error::DoitError::config(
            "missing required argument: task",
        ));
    }

    // 子 Agent 拥有独立会话(独立目录与上下文),与父级隔离;输出按 verbosity 落到 stdout
    let agent = Agent::from_config(&ctx.config);
    let mut session = Session::create(&std::env::current_dir().unwrap(), &ctx.config.model.name)?;
    agent
        .run_task(ctx, &mut session, &task, args.verbosity)
        .await
}
