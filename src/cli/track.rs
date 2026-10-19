use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::Args;
use dialoguer::Confirm;

use super::{Ctx, RunOpts, run_plan};
use crate::config::manifest::{Entry, Mode, collapse, default_repo_path, resolve, validate_repo_path};
use crate::error::Error;
use crate::filesystem::scanner::scan;
use crate::sync::planner::Direction;

#[derive(Args, Debug)]
pub struct TrackArgs {
    /// Files or directories to track
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Repository-relative location for the copy (single path only)
    #[arg(long = "as", value_name = "REPO_PATH")]
    pub repo_path: Option<String>,

    /// Record that the target needs elevated privileges (informational in v0.1)
    #[arg(long)]
    pub sudo: bool,

    /// Do not ask about sensitive-looking files
    #[arg(short, long)]
    pub yes: bool,
}

const SENSITIVE: &[&str] = &[
    ".ssh", ".aws", ".gnupg", ".kube", ".docker", ".netrc", ".npmrc", ".pgpass", "id_rsa",
    "id_ed25519", "credentials", ".env",
];

pub fn run(ctx: &mut Ctx, args: TrackArgs) -> Result<()> {
    if args.repo_path.is_some() && args.paths.len() > 1 {
        bail!("--as can only be used with a single path");
    }
    let cwd = std::env::current_dir()?;
    for path in &args.paths {
        track_one(ctx, &cwd, path, &args)?;
    }
    Ok(())
}

fn track_one(ctx: &mut Ctx, cwd: &Path, path: &Path, args: &TrackArgs) -> Result<()> {
    let target = resolve(&ctx.home, cwd, path);
    if !target.exists() {
        return Err(Error::MissingTarget(target).into());
    }
    if target.starts_with(&ctx.repo) || ctx.repo.starts_with(&target) {
        bail!("{} overlaps the SmartRepo repository itself", target.display());
    }
    let shown = collapse(&ctx.home, &target);

    for existing in &ctx.manifest.files {
        let other = existing.target_path(&ctx.home);
        if other == target {
            return Err(Error::AlreadyTracked(shown).into());
        }
        if other.starts_with(&target) || target.starts_with(&other) {
            return Err(Error::Overlap(shown, existing.target.clone()).into());
        }
    }

    let repo_path = match &args.repo_path {
        Some(given) => validate_repo_path(given)?,
        None => default_repo_path(&ctx.home, &target)?,
    };
    for existing in &ctx.manifest.files {
        let (a, b) = (Path::new(&existing.repo), Path::new(&repo_path));
        if a.starts_with(b) || b.starts_with(a) {
            return Err(Error::RepoPathTaken(repo_path, existing.target.clone()).into());
        }
    }

    if let Some(hit) = find_sensitive(&target)? {
        if !args.yes && !confirm_sensitive(&shown, &hit)? {
            println!("Skipped {shown}");
            return Ok(());
        }
    }

    let entry = Entry {
        target: shown.clone(),
        repo: repo_path.clone(),
        mode: Mode::Copy,
        sudo: args.sudo,
    };
    ctx.manifest.files.push(entry.clone());

    let opts = RunOpts { dry_run: false, yes: true, on_conflict: None };
    if let Err(e) = run_plan(ctx, &[entry], Direction::ToRepo, &opts) {
        ctx.manifest.files.pop();
        return Err(e);
    }
    ctx.manifest.save(&ctx.repo)?;
    println!("Tracking {shown} -> {repo_path}");
    Ok(())
}

/// First sensitive-looking name in the path itself or anywhere below it.
fn find_sensitive(target: &Path) -> Result<Option<String>> {
    let hit = |p: &Path| {
        p.components().find_map(|c| {
            c.as_os_str()
                .to_str()
                .filter(|n| SENSITIVE.contains(n))
                .map(str::to_owned)
        })
    };
    if let Some(h) = hit(target) {
        return Ok(Some(h));
    }
    for rel in scan(target)?.keys() {
        if let Some(h) = hit(Path::new(rel)) {
            return Ok(Some(h));
        }
    }
    Ok(None)
}

fn confirm_sensitive(shown: &str, hit: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        bail!("{shown} looks sensitive ({hit}); pass --yes to track it anyway");
    }
    Ok(Confirm::new()
        .with_prompt(format!(
            "{shown} looks sensitive ({hit}) and would be stored in Git. Track anyway?"
        ))
        .default(false)
        .interact()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn sensitive_by_path_and_by_child() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = dir.path().join(".ssh");
        fs::create_dir_all(&ssh).unwrap();
        fs::write(ssh.join("config"), "x").unwrap();
        assert_eq!(find_sensitive(&ssh.join("config")).unwrap().as_deref(), Some(".ssh"));

        let plain = dir.path().join("proj");
        fs::create_dir_all(&plain).unwrap();
        fs::write(plain.join("id_rsa"), "k").unwrap();
        assert_eq!(find_sensitive(&plain).unwrap().as_deref(), Some("id_rsa"));

        fs::remove_file(plain.join("id_rsa")).unwrap();
        fs::write(plain.join("ok.toml"), "k").unwrap();
        assert_eq!(find_sensitive(&plain).unwrap(), None);
    }
}
