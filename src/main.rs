mod cli;
mod config;
mod error;
mod filesystem;
mod git;
mod sync;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use cli::track::TrackArgs;
use cli::untrack::UntrackArgs;
use cli::{Ctx, RunOpts};

/// Manages configuration files scattered throughout a user's filesystem while keeping them synchronized
#[derive(Parser)]
#[command(name = "smartrepo", version, about)]
struct Cli {
    /// SmartRepo repository to operate on (default: search upward from the current directory)
    #[arg(long, global = true, env = "SMARTREPO_DIR")]
    repo: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new repo
    Init { path: Option<PathBuf> },

    /// Track the specific file and folder
    Track(TrackArgs),

    /// Untrack the path or file
    Untrack(UntrackArgs),

    /// List all paths in sync
    List,

    /// Get status
    Status,

    /// Get diff
    Diff,

    /// Copy filesystem changes into the repository working tree
    Sync(RunOpts),

    /// Make the filesystem match the repository
    Apply(RunOpts),
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Command::Init { path } = cli.command {
        return cli::init::run(path);
    }

    let mut ctx = Ctx::load(cli.repo.as_deref())?;
    match cli.command {
        Command::Init { .. } => unreachable!("handled above"),
        Command::Track(args) => cli::track::run(&mut ctx, args),
        Command::Untrack(args) => cli::untrack::run(&mut ctx, args),
        Command::List => cli::list::run(&ctx),
        Command::Status => cli::status::run(&ctx),
        Command::Diff => cli::diff::run(&ctx),
        Command::Sync(opts) => cli::sync::run(&mut ctx, &opts),
        Command::Apply(opts) => cli::apply::run(&mut ctx, &opts),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
