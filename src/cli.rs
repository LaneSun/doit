use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands::{
    config, edit, exec, exit, glob, interactive, prompt, read, resume, run, search, task, template,
    write,
};

#[derive(Parser)]
#[command(name = "doit", about = "A shell-first AI agent", version)]
pub struct Cli {
    /// Path to an extra config file (highest-priority file layer)
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Override the model name (highest priority)
    #[arg(long, global = true, value_name = "NAME")]
    pub model: Option<String>,

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
    /// Run a task as a sub-agent (non-interactive); output controlled by --verbosity
    Task(task::Args),
    /// Generate prompt templates
    Template(template::Args),
    /// Write content to a file (reads from stdin)
    Write(write::Args),
    /// Resume a previous session
    Resume(resume::Args),
    /// View or edit configuration
    Config(config::Args),
}
