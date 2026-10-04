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
    Init,

    /// Track the specific file and folder
    Track,

    /// Untrack the path or file
    Untrack,

    /// List all paths in sync
    List,

    /// Get status
    Status,

    /// Get diff
    Diff,

    /// Sync changes with remote
    Sync,

    /// Pull changes from remote to local
    Apply,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Command::Init => println!("Initialize empty sync repo"),
        Command::Track => println!("Implement track"),
        Command::Untrack => println!("Untrack"),
        Command::List => println!("List"),
        Command::Status => println!("Status"),
        Command::Diff => println!("Diff"),
        Command::Sync => println!("Sync"),
        Command::Apply => println!("Apply"),
    }
}
