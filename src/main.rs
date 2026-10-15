use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};

mod error;

#[derive(Parser)]
#[command(name = "sys-config", version, about)]
struct Cli {
    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new repo
    Init { path: Option<PathBuf> },

    /// Track the specific file and folder
    Track { paths: Vec<PathBuf> },

    /// Untrack the path or file
    Untrack { paths: Vec<PathBuf> },

    /// List all paths in sync
    List,
    /// Get status
    Status,
    /// Get diff
    Diff,
    /// Sync changes with remote
    Sync,
    /// Pull changes from remote and sync with system
    Apply,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    println!("repo: {:?}", cli.repo);

    match cli.command {
        Command::Init { path } => println!("init {path:?}"),
        Command::Track { paths } => println!("track {paths:?}"),
        Command::Untrack { paths } => println!("untrack {paths:?}"),
        Command::List => println!("list"),
        Command::Status => println!("status"),
        Command::Diff => println!("diff"),
        Command::Sync => println!("sync"),
        Command::Apply => println!("apply"),
    }

    ExitCode::SUCCESS
}
