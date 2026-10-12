mod error;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Manages configuration files scattered throught a user's filesystem while keeping them synchronized
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new repo
    Init { path: Option<String> },

    /// Track the specific file and folder
    Track { path: PathBuf },

    /// Untrack the path or file
    Untrack { links: Vec<String> },

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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { path } => {
            if path.is_none() {
                // TODO: Pick the current location
            }

            println!("{:?} init location", path)
        }
        Command::Track { path } => {
            println!("{:?}", path)
        }
        Command::Untrack { links } => println!("Untrack"),
        Command::List => println!("List"),
        Command::Status => println!("Status"),
        Command::Diff => println!("Diff"),
        Command::Sync => println!("Sync"),
        Command::Apply => println!("Apply"),
    }
}
