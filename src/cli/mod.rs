use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Args;
use dialoguer::Confirm;

use crate::config::discovery::find_repo;
use crate::config::manifest::{Entry, Manifest, home_dir};
use crate::config::state::State;
use crate::error::Error;
use crate::sync::conflict::{self, Policy};
use crate::sync::executor;
use crate::sync::planner::{self, Direction, SyncPlan};

pub mod apply;
pub mod diff;
pub mod init;
pub mod list;
pub mod status;
pub mod sync;
pub mod track;
pub mod untrack;

/// Everything a command needs, loaded once at startup.
pub struct Ctx {
    pub home: PathBuf,
    pub repo: PathBuf,
    pub manifest: Manifest,
    pub state: State,
}

impl Ctx {
    pub fn load(explicit_repo: Option<&std::path::Path>) -> Result<Self> {
        let home = home_dir()?;
        let cwd = std::env::current_dir()?;
        let repo = find_repo(explicit_repo, &cwd, &home)?;
        let manifest = Manifest::load(&repo)?;
        let state = State::load(&repo)?;
        Ok(Ctx { home, repo, manifest, state })
    }
}

/// Flags shared by `sync` and `apply`.
#[derive(Args, Debug, Clone)]
pub struct RunOpts {
    /// Show what would happen and exit without changing anything
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Do not ask for confirmation
    #[arg(short, long)]
    pub yes: bool,

    /// How to settle conflicts without asking
    #[arg(long, value_enum)]
    pub on_conflict: Option<Policy>,
}

fn print_plan(plan: &SyncPlan, dir: Direction) {
    let flow = match dir {
        Direction::ToRepo => "filesystem -> repository",
        Direction::ToTarget => "repository -> filesystem",
    };
    println!("Plan ({flow}):");
    for action in plan.pending() {
        println!("  {} {}", action.kind.symbol(), action.display);
    }
}

fn confirm() -> Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!("not a terminal; pass --yes to proceed without confirmation");
    }
    if !Confirm::new().with_prompt("Proceed?").default(false).interact()? {
        return Err(Error::Aborted.into());
    }
    Ok(())
}

/// Plan, show, resolve, confirm, execute, remember. `entries` is the subset to operate on.
pub fn run_plan(ctx: &mut Ctx, entries: &[Entry], dir: Direction, opts: &RunOpts) -> Result<()> {
    let mut plan = planner::plan(&ctx.home, &ctx.repo, entries, &ctx.state, dir)?;

    if plan.pending().next().is_none() {
        println!("Nothing to do.");
        if !opts.dry_run {
            // identical files on both sides still deserve a recorded base
            executor::record_bases(&ctx.home, &ctx.repo, entries, &mut ctx.state)?;
            ctx.state.save(&ctx.repo)?;
        }
        return Ok(());
    }

    print_plan(&plan, dir);
    if opts.dry_run {
        return Ok(());
    }

    conflict::resolve(&mut plan, opts.on_conflict)?;
    if !opts.yes {
        confirm()?;
    }

    let changed = executor::execute(&plan, &ctx.home, &ctx.repo, entries, &mut ctx.state)?;
    ctx.state.save(&ctx.repo)?;
    println!("Done: {changed} file(s) changed.");
    Ok(())
}
