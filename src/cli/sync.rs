use anyhow::Result;

use super::{Ctx, RunOpts, run_plan};
use crate::sync::planner::Direction;

pub fn run(ctx: &mut Ctx, opts: &RunOpts) -> Result<()> {
    let entries = ctx.manifest.files.clone();
    run_plan(ctx, &entries, Direction::ToRepo, opts)
}
