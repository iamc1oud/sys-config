use anyhow::Result;

use super::Ctx;
use crate::git::diff;
use crate::sync::planner::{Direction, plan};

pub fn run(ctx: &Ctx) -> Result<()> {
    let p = plan(&ctx.home, &ctx.repo, &ctx.manifest.files, &ctx.state, Direction::ToRepo)?;
    let mut any = false;
    for a in p.actions.iter().filter(|a| a.target_hash != a.repo_hash) {
        any = true;
        println!("=== {} (repository -> filesystem)", a.display);
        let old = a.repo_hash.is_some().then_some(a.repo.as_path());
        let new = a.target_hash.is_some().then_some(a.target.as_path());
        print!("{}", diff::unified(old, new)?);
    }
    if !any {
        println!("No differences.");
    }
    Ok(())
}
