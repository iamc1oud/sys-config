use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, bail};

use super::planner::{Action, Direction, Kind, SyncPlan};
use crate::config::manifest::Entry;
use crate::config::state::State;
use crate::filesystem::copier::{copy_file, remove_file_prune};
use crate::filesystem::scanner::scan;

fn run_action(a: &Action) -> Result<()> {
    match (a.kind, a.dir) {
        (Kind::Create | Kind::Update, Direction::ToRepo) => copy_file(&a.target, &a.repo),
        (Kind::Create | Kind::Update, Direction::ToTarget) => copy_file(&a.repo, &a.target),
        (Kind::Delete, Direction::ToRepo) => remove_file_prune(&a.repo, &a.repo_root),
        (Kind::Delete, Direction::ToTarget) => remove_file_prune(&a.target, &a.target_root),
        (Kind::Conflict, _) => bail!("internal error: unresolved conflict for {}", a.display),
        (Kind::Unchanged, _) => Ok(()),
    }
}

/// Perform the plan. Returns the number of files changed.
pub fn execute(
    plan: &SyncPlan,
    home: &Path,
    repo_root: &Path,
    entries: &[Entry],
    state: &mut State,
) -> Result<usize> {
    let mut changed = 0;
    for action in plan.pending() {
        run_action(action)?;
        changed += 1;
    }
    record_bases(home, repo_root, entries, state)?;
    Ok(changed)
}

/// Remember, for every file that is now identical on both sides, its hash as the new base.
pub fn record_bases(home: &Path, repo_root: &Path, entries: &[Entry], state: &mut State) -> Result<()> {
    for entry in entries {
        let target_snap = scan(&entry.target_path(home))?;
        let repo_snap = scan(&repo_root.join(&entry.repo))?;

        let mut rels: BTreeSet<String> = target_snap.keys().chain(repo_snap.keys()).cloned().collect();
        rels.extend(state.rels(&entry.repo));

        for rel in rels {
            match (target_snap.get(&rel), repo_snap.get(&rel)) {
                (Some(t), Some(r)) if t == r => state.set(&entry.repo, &rel, t.clone()),
                (None, None) => state.remove(&entry.repo, &rel),
                _ => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::manifest::Mode;
    use crate::sync::planner::plan;
    use std::fs;

    struct Fixture {
        _tmp: tempfile::TempDir,
        home: std::path::PathBuf,
        repo: std::path::PathBuf,
        entries: Vec<Entry>,
        state: State,
    }

    fn fixture() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let repo = tmp.path().join("repo");
        fs::create_dir_all(home.join(".config/app")).unwrap();
        fs::create_dir_all(&repo).unwrap();
        fs::write(home.join(".config/app/a.conf"), "a1").unwrap();
        fs::write(home.join(".config/app/b.conf"), "b1").unwrap();
        let entries = vec![Entry {
            target: "~/.config/app".into(),
            repo: "app".into(),
            mode: Mode::Copy,
            sudo: false,
        }];
        Fixture { _tmp: tmp, home, repo, entries, state: State::default() }
    }

    fn run(f: &mut Fixture, dir: Direction) -> usize {
        let p = plan(&f.home, &f.repo, &f.entries, &f.state, dir).unwrap();
        execute(&p, &f.home, &f.repo, &f.entries, &mut f.state).unwrap()
    }

    #[test]
    fn sync_then_edit_then_apply_cycle() {
        let mut f = fixture();

        // first sync creates both files in the repo and records bases
        assert_eq!(run(&mut f, Direction::ToRepo), 2);
        assert_eq!(fs::read_to_string(f.repo.join("app/a.conf")).unwrap(), "a1");
        assert_eq!(run(&mut f, Direction::ToRepo), 0);

        // edit locally, delete another, sync again
        fs::write(f.home.join(".config/app/a.conf"), "a2").unwrap();
        fs::remove_file(f.home.join(".config/app/b.conf")).unwrap();
        assert_eq!(run(&mut f, Direction::ToRepo), 2);
        assert_eq!(fs::read_to_string(f.repo.join("app/a.conf")).unwrap(), "a2");
        assert!(!f.repo.join("app/b.conf").exists());

        // repo moves ahead (as after a `git pull`), apply brings it down
        fs::write(f.repo.join("app/a.conf"), "a3").unwrap();
        fs::write(f.repo.join("app/c.conf"), "c1").unwrap();
        assert_eq!(run(&mut f, Direction::ToRepo), 0, "sync must not clobber repo-ahead files");
        assert_eq!(run(&mut f, Direction::ToTarget), 2);
        assert_eq!(fs::read_to_string(f.home.join(".config/app/a.conf")).unwrap(), "a3");
        assert_eq!(fs::read_to_string(f.home.join(".config/app/c.conf")).unwrap(), "c1");
    }

    #[test]
    fn fresh_machine_with_matching_files_gets_bases_without_changes() {
        let mut f = fixture();
        run(&mut f, Direction::ToRepo);
        f.state = State::default(); // pretend: clone on a machine that already has identical files
        assert_eq!(run(&mut f, Direction::ToTarget), 0);
        assert!(f.state.get("app", "a.conf").is_some());
    }

    #[test]
    fn unresolved_conflict_is_refused() {
        let mut f = fixture();
        run(&mut f, Direction::ToRepo);
        fs::write(f.home.join(".config/app/a.conf"), "local").unwrap();
        fs::write(f.repo.join("app/a.conf"), "remote").unwrap();
        let p = plan(&f.home, &f.repo, &f.entries, &f.state, Direction::ToTarget).unwrap();
        assert_eq!(p.conflicts().count(), 1);
        assert!(execute(&p, &f.home, &f.repo, &f.entries, &mut f.state).is_err());
        // nothing was overwritten
        assert_eq!(fs::read_to_string(f.home.join(".config/app/a.conf")).unwrap(), "local");
    }
}
