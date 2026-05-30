use crate::config::{self, Config, Scope};
use crate::context::RuntimeContext;
use crate::error::{DoitError, Result};

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    pub action: Option<Action>,

    /// Output skill reference for LLM
    #[arg(long, default_value_t = false)]
    pub skill: bool,
}

#[derive(clap::Subcommand)]
pub enum Action {
    /// Print effective config file paths
    Path,
    /// Print the effective (merged) configuration as TOML
    List,
    /// Get a single value by dotted key (e.g. model.name)
    Get { key: String },
    /// Set a value by dotted key, written to the user (default) or project layer
    Set {
        key: String,
        value: String,
        /// Write to the project-level file (./doit.toml) instead of the user file
        #[arg(long, default_value_t = false)]
        project: bool,
    },
}

pub async fn execute(ctx: &RuntimeContext, args: &Args) -> Result<()> {
    if args.skill {
        println!("{}", rust_i18n::t!("config.skill"));
        return Ok(());
    }

    match &args.action {
        // 无子命令时等同 list:打印当前生效配置
        None | Some(Action::List) => {
            print!("{}", ctx.config.to_toml()?);
        }
        Some(Action::Path) => {
            let user = Config::user_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<unavailable>".to_string());
            println!("{}: {}", rust_i18n::t!("config.user_label"), user);
            println!(
                "{}: {}",
                rust_i18n::t!("config.project_label"),
                Config::project_path().display()
            );
        }
        Some(Action::Get { key }) => {
            println!("{}", config::get_value(&ctx.config, key)?);
        }
        Some(Action::Set { key, value, project }) => {
            let scope = if *project { Scope::Project } else { Scope::User };
            let path = config::set_value(scope, key, value)?;
            // 校验写入结果可被重新加载
            Config::load(None).map_err(|e| {
                DoitError::config(format!("config written but failed to reload: {e}"))
            })?;
            println!(
                "{}",
                rust_i18n::t!("config.set_done", key => key, value => value, path => path.display())
            );
        }
    }
    Ok(())
}
