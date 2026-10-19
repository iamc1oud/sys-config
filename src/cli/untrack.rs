use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use super::Ctx;
use crate::config::manifest::{collapse, resolve};
use crate::error::Error;

#[derive(Args, Debug)]
pub struct UntrackArgs {
    /// Targets to stop tracking, as originally given to `track`
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Also delete the stored copy from the repository working tree
    #[arg(long)]
    pub purge: bool,
}

pub fn run(ctx: &mut Ctx, args: UntrackArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    for path in &args.paths {
        let target = resolve(&ctx.home, &cwd, path);
        let idx = ctx
            .manifest
            .position(&ctx.home, &target)
            .ok_or_else(|| Error::NotTracked(collapse(&ctx.home, &target)))?;

        let entry = ctx.manifest.files.remove(idx);
        ctx.state.forget_entry(&entry.repo);
        if args.purge {
            let stored = ctx.repo.join(&entry.repo);
            if stored.is_dir() {
                fs::remove_dir_all(&stored)?;
            } else if stored.exists() {
                fs::remove_file(&stored)?;
            }
        }
        ctx.manifest.save(&ctx.repo)?;
        ctx.state.save(&ctx.repo)?;

        println!("Untracked {}", entry.target);
        if !args.purge {
            println!("  (stored copy kept at {}; use --purge to delete it)", entry.repo);
        }
    }
    Ok(())
}
