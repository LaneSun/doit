use std::io::Read;
use std::process::Command as StdCommand;

use crate::backend::DeepSeekBackend;
use crate::block::Block;
use crate::context::RuntimeContext;
use crate::error::Result;
use crate::session::Session;

pub struct Agent {
    backend: DeepSeekBackend,
}

struct CommandResult {
    output: String,
    exit_code: i32,
}

impl Agent {
    pub fn new(backend: DeepSeekBackend) -> Self {
        Self { backend }
    }

    pub async fn run_interactive(
        &self,
        ctx: &RuntimeContext,
        session: &mut Session,
    ) -> Result<()> {
        // Build initial system prompt
        let system_content = self.generate_system_prompt(ctx, true).await?;
        let system_block = Block::System {
            seq: session.next_seq(),
            content: system_content,
        };
        session.append(system_block)?;

        self.run_loop(ctx, session).await
    }

    pub async fn run_task(
        &self,
        ctx: &RuntimeContext,
        session: &mut Session,
        task: &str,
    ) -> Result<()> {
        let system_content = self.generate_system_prompt(ctx, false).await?;
        let system_block = Block::System {
            seq: session.next_seq(),
            content: format!("{}\n\nTask: {task}\n\nExecute the assigned task.", system_content),
        };
        session.append(system_block)?;

        self.run_loop(ctx, session).await
    }

    async fn run_loop(&self, _ctx: &RuntimeContext, session: &mut Session) -> Result<()> {
        loop {
            let messages = session.build_messages();
            let response = self.backend.chat(&messages).await?;

            // Determine cmd
            let (cmd, reasoning, content, tool_call_id) = if response.is_prompt {
                let c = response.content.as_deref().unwrap_or("");
                let prompt_cmd = format!("doit prompt '{}'", c);
                (prompt_cmd, response.reasoning, response.content, None)
            } else {
                (
                    response.cmd.clone().unwrap_or_default(),
                    response.reasoning,
                    None,
                    response.tool_call_id,
                )
            };

            // Append assistant block
            let tc_id = tool_call_id.clone();
            let assistant_block = Block::Assistant {
                seq: session.next_seq(),
                reasoning: reasoning.unwrap_or_default(),
                cmd: cmd.clone(),
                tool_call_id,
                content,
            };
            session.append(assistant_block)?;

            // Execute command
            let result = self.execute_command(&cmd).await?;

            // Append tool block
            let tool_block = Block::Tool {
                seq: session.next_seq(),
                output: result.output,
                exit_code: result.exit_code,
                tool_call_id: tc_id.unwrap_or_else(|| "unknown".to_string()),
            };
            session.append(tool_block)?;

            // Check for exit
            if cmd.trim().starts_with("doit exit ") {
                break;
            }
        }

        Ok(())
    }

    async fn execute_command(&self, cmd: &str) -> Result<CommandResult> {
        let final_cmd = if cmd.trim().starts_with("doit ") {
            cmd.to_string()
        } else {
            format!("doit exec -- {}", cmd)
        };

        let output = tokio::task::spawn_blocking(move || pty_execute(&final_cmd))
            .await
            .map_err(|e| crate::error::DoitError::shell(format!("join error: {e}")))?;

        output
    }

    async fn generate_system_prompt(
        &self,
        ctx: &RuntimeContext,
        interactive: bool,
    ) -> Result<String> {
        let current_exe = std::env::current_exe()
            .map_err(|e| crate::error::DoitError::io(e, "cannot find current executable"))?;

        let flag = if interactive {
            "--interactive"
        } else {
            ""
        };

        let mut cmd = StdCommand::new(&current_exe);
        cmd.args(["template", "system"]);
        if interactive {
            cmd.arg(flag);
        }
        cmd.env("LANG", format!("{}.UTF-8", ctx.locale));

        let output = cmd.output().map_err(|e| {
            crate::error::DoitError::shell(format!("template generation failed: {e}"))
        })?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

fn pty_execute(cmd_str: &str) -> Result<CommandResult> {
    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system
        .openpty(portable_pty::PtySize::default())
        .map_err(|e| crate::error::DoitError::shell(format!("pty error: {e}")))?;

    let mut cmd = portable_pty::CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(cmd_str);

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| crate::error::DoitError::shell(format!("spawn error: {e}")))?;

    drop(pair.slave);

    let mut output = Vec::new();
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| crate::error::DoitError::shell(format!("read error: {e}")))?;
    reader
        .read_to_end(&mut output)
        .map_err(|e| crate::error::DoitError::shell(format!("read error: {e}")))?;

    let status = child
        .wait()
        .map_err(|e| crate::error::DoitError::shell(format!("wait error: {e}")))?;

    Ok(CommandResult {
        output: String::from_utf8_lossy(&output).to_string(),
        exit_code: status.exit_code() as i32,
    })
}
