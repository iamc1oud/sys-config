use anyhow::Result;

use super::Ctx;

pub fn run(ctx: &Ctx) -> Result<()> {
    if ctx.manifest.files.is_empty() {
        println!("No tracked paths.");
        return Ok(());
    }
    for e in &ctx.manifest.files {
        let sudo = if e.sudo { " [sudo]" } else { "" };
        println!("{:?} {} -> {}{sudo}", e.mode, e.target, e.repo);
    }
    Ok(())
}
