use clap::{Parser, Subcommand};

use crate::commands::{exit, glob, interactive, prompt, read, run, task, resume};

#[derive(Parser)]
#[command(
    name = "doit",
    about = "A shell-first AI agent",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start interactive REPL mode
    Interactive(interactive::Args),
    /// Complete the current task and exit
    Exit(exit::Args),
    /// File pattern matching
    Glob(glob::Args),
    /// Block waiting for user input
    Prompt(prompt::Args),
    /// Read file contents with line numbers
    Read(read::Args),
    /// Execute a one-shot task (turn-by-turn display)
    Run(run::Args),
    /// Execute a task as sub-agent (non-interactive, result only)
    Task(task::Args),
    /// Resume a previous session
    Resume(resume::Args),
}
