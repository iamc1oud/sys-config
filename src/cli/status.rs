use anyhow::Result;

use super::Ctx;
use crate::sync::planner::{Direction, Kind, plan};

pub fn run(ctx: &Ctx) -> Result<()> {
    if ctx.manifest.files.is_empty() {
        println!("No tracked paths.");
        return Ok(());
    }
    let entries = &ctx.manifest.files;
    let local = plan(&ctx.home, &ctx.repo, entries, &ctx.state, Direction::ToRepo)?;
    let remote = plan(&ctx.home, &ctx.repo, entries, &ctx.state, Direction::ToTarget)?;

    let mut clean = true;
    let sections = [
        ("Modified", Kind::Update),
        ("Added", Kind::Create),
        ("Deleted", Kind::Delete),
        ("Conflicts", Kind::Conflict),
    ];
    for (title, kind) in sections {
        let rows: Vec<_> = local.actions.iter().filter(|a| a.kind == kind).collect();
        if rows.is_empty() {
            continue;
        }
        clean = false;
        println!("{title}");
        for a in rows {
            println!("  {} {}", kind.symbol(), a.display);
        }
    }

    let behind: Vec<_> = remote
        .actions
        .iter()
        .filter(|a| matches!(a.kind, Kind::Create | Kind::Update | Kind::Delete))
        .collect();
    if !behind.is_empty() {
        clean = false;
        println!("Repository changes (run `smartrepo apply`)");
        for a in behind {
            println!("  {} {}", a.kind.symbol(), a.display);
        }
    }

    if clean {
        println!("Everything in sync.");
    }
    Ok(())
}
