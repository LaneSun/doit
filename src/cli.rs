use clap::{Parser, Subcommand};

use crate::commands::{
    edit, exec, exit, glob, interactive, prompt, read, resume, run, search, task, template, write,
};

#[derive(Parser)]
#[command(name = "doit", about = "A shell-first AI agent", version)]
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
    /// Structured file editing
    Edit(edit::Args),
    /// Execute a shell command via PTY
    Exec(exec::Args),
    /// File pattern matching
    Glob(glob::Args),
    /// Block waiting for user input
    Prompt(prompt::Args),
    /// Read file contents with line numbers
    Read(read::Args),
    /// Search file contents with regex
    Search(search::Args),
    /// Execute a one-shot task (turn-by-turn display)
    Run(run::Args),
    /// Execute a task as sub-agent (non-interactive, result only)
    Task(task::Args),
    /// Generate prompt templates
    Template(template::Args),
    /// Write content to a file (reads from stdin)
    Write(write::Args),
    /// Resume a previous session
    Resume(resume::Args),
}
