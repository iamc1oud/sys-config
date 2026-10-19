use std::io::IsTerminal;

use anyhow::Result;
use clap::ValueEnum;
use dialoguer::Select;

use super::planner::{Action, Direction, Kind, SyncPlan};
use crate::error::Error;
use crate::git::diff;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Policy {
    /// Local filesystem content wins; the repository copy is overwritten.
    KeepLocal,
    /// Repository content wins; the local file is overwritten.
    UseRepo,
    /// Stop without changing anything.
    Abort,
}

/// Turn every `Conflict` in `plan` into a concrete action, asking if needed.
pub fn resolve(plan: &mut SyncPlan, policy: Option<Policy>) -> Result<()> {
    let count = plan.conflicts().count();
    if count == 0 {
        return Ok(());
    }
    if policy.is_none() && !std::io::stdin().is_terminal() {
        for a in plan.conflicts() {
            eprintln!("conflict: {}", a.display);
        }
        return Err(Error::Conflicts(count).into());
    }
    for action in plan.actions.iter_mut().filter(|a| a.kind == Kind::Conflict) {
        let choice = match policy {
            Some(p) => p,
            None => prompt(action)?,
        };
        apply_choice(action, choice)?;
    }
    Ok(())
}

fn apply_choice(a: &mut Action, choice: Policy) -> Result<()> {
    match choice {
        Policy::KeepLocal => {
            a.dir = Direction::ToRepo;
            a.kind = if a.target_hash.is_some() { Kind::Update } else { Kind::Delete };
        }
        Policy::UseRepo => {
            a.dir = Direction::ToTarget;
            a.kind = if a.repo_hash.is_some() { Kind::Update } else { Kind::Delete };
        }
        Policy::Abort => return Err(Error::Aborted.into()),
    }
    Ok(())
}

fn prompt(a: &Action) -> Result<Policy> {
    loop {
        eprintln!("\nConflict detected:\n  {}", a.display);
        let pick = Select::new()
            .with_prompt("Options")
            .items(["Keep local", "Use repository", "Show diff", "Abort"])
            .default(3)
            .interact()?;
        match pick {
            0 => return Ok(Policy::KeepLocal),
            1 => return Ok(Policy::UseRepo),
            2 => {
                let old = a.repo_hash.is_some().then_some(a.repo.as_path());
                let new = a.target_hash.is_some().then_some(a.target.as_path());
                println!("{}", diff::unified(old, new)?);
            }
            _ => return Ok(Policy::Abort),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn conflict(target: Option<&str>, repo: Option<&str>) -> Action {
        Action {
            dir: Direction::ToRepo,
            kind: Kind::Conflict,
            display: "~/f".into(),
            target: PathBuf::from("/t/f"),
            repo: PathBuf::from("/r/f"),
            target_root: PathBuf::from("/t/f"),
            repo_root: PathBuf::from("/r/f"),
            target_hash: target.map(Into::into),
            repo_hash: repo.map(Into::into),
        }
    }

    #[test]
    fn keep_local_pushes_to_repo() {
        let mut a = conflict(Some("x"), Some("y"));
        apply_choice(&mut a, Policy::KeepLocal).unwrap();
        assert_eq!((a.dir, a.kind), (Direction::ToRepo, Kind::Update));

        let mut a = conflict(None, Some("y"));
        apply_choice(&mut a, Policy::KeepLocal).unwrap();
        assert_eq!((a.dir, a.kind), (Direction::ToRepo, Kind::Delete));
    }

    #[test]
    fn use_repo_pulls_to_target() {
        let mut a = conflict(Some("x"), Some("y"));
        apply_choice(&mut a, Policy::UseRepo).unwrap();
        assert_eq!((a.dir, a.kind), (Direction::ToTarget, Kind::Update));

        let mut a = conflict(Some("x"), None);
        apply_choice(&mut a, Policy::UseRepo).unwrap();
        assert_eq!((a.dir, a.kind), (Direction::ToTarget, Kind::Delete));
    }

    #[test]
    fn policy_resolves_whole_plan_and_abort_fails() {
        let mut plan = SyncPlan { actions: vec![conflict(Some("x"), Some("y"))] };
        resolve(&mut plan, Some(Policy::UseRepo)).unwrap();
        assert_eq!(plan.conflicts().count(), 0);

        let mut plan = SyncPlan { actions: vec![conflict(Some("x"), Some("y"))] };
        assert!(resolve(&mut plan, Some(Policy::Abort)).is_err());
    }
}
