use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::manifest::{Entry, collapse};
use crate::config::state::State;
use crate::error::Error;
use crate::filesystem::scanner::{join_rel, scan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// sync: filesystem -> repository
    ToRepo,
    /// apply: repository -> filesystem
    ToTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Create,
    Update,
    Delete,
    Conflict,
    Unchanged,
}

impl Kind {
    pub fn symbol(self) -> char {
        match self {
            Kind::Create => '+',
            Kind::Update => 'M',
            Kind::Delete => '-',
            Kind::Conflict => '!',
            Kind::Unchanged => '=',
        }
    }
}

#[derive(Debug, Clone)]
pub struct Action {
    pub dir: Direction,
    pub kind: Kind,
    /// Target path with `~`, for printing.
    pub display: String,
    pub target: PathBuf,
    pub repo: PathBuf,
    /// Root of the entry on each side; pruning of empty dirs stops here.
    pub target_root: PathBuf,
    pub repo_root: PathBuf,
    pub target_hash: Option<String>,
    pub repo_hash: Option<String>,
}

#[derive(Debug)]
pub struct SyncPlan {
    pub actions: Vec<Action>,
}

impl SyncPlan {
    /// Actions that would change something (everything but `Unchanged`).
    pub fn pending(&self) -> impl Iterator<Item = &Action> {
        self.actions.iter().filter(|a| a.kind != Kind::Unchanged)
    }

    pub fn conflicts(&self) -> impl Iterator<Item = &Action> {
        self.actions.iter().filter(|a| a.kind == Kind::Conflict)
    }
}

/// Three-way decision for one file. See `specs/00-overview.md`.
pub fn decide(dir: Direction, target: Option<&str>, repo: Option<&str>, base: Option<&str>) -> Kind {
    if target == repo {
        return Kind::Unchanged;
    }
    let (src, dst) = match dir {
        Direction::ToRepo => (target, repo),
        Direction::ToTarget => (repo, target),
    };

    // On apply (ToTarget): if target is absent, base is absent, and repo exists: Create (fresh clone).
    // This handles: cloning a repo on a new machine where the config doesn't exist yet.
    if dir == Direction::ToTarget && dst.is_none() && src.is_some() && base.is_none() {
        return Kind::Create;
    }

    match (src != base, dst != base) {
        (true, false) => match (src, dst) {
            (None, _) => Kind::Delete,
            (Some(_), None) => Kind::Create,
            (Some(_), Some(_)) => Kind::Update,
        },
        (false, _) => Kind::Unchanged,
        (true, true) => Kind::Conflict,
    }
}

pub fn plan(
    home: &Path,
    repo_root: &Path,
    entries: &[Entry],
    state: &State,
    dir: Direction,
) -> Result<SyncPlan> {
    let mut actions = Vec::new();

    for entry in entries {
        let target_root = entry.target_path(home);
        let repo_dir = repo_root.join(&entry.repo);
        let target_snap = scan(&target_root)?;
        let repo_snap = scan(&repo_dir)?;

        let both_present = !target_snap.is_empty() && !repo_snap.is_empty();
        if both_present && target_snap.contains_key("") != repo_snap.contains_key("") {
            return Err(Error::TypeMismatch(entry.target.clone()).into());
        }

        let rels: BTreeSet<&String> = target_snap.keys().chain(repo_snap.keys()).collect();
        for rel in rels {
            let t = target_snap.get(rel).map(String::as_str);
            let r = repo_snap.get(rel).map(String::as_str);
            let target = join_rel(&target_root, rel);
            actions.push(Action {
                dir,
                kind: decide(dir, t, r, state.get(&entry.repo, rel)),
                display: collapse(home, &target),
                target,
                repo: join_rel(&repo_dir, rel),
                target_root: target_root.clone(),
                repo_root: repo_dir.clone(),
                target_hash: t.map(str::to_owned),
                repo_hash: r.map(str::to_owned),
            });
        }
    }
    Ok(SyncPlan { actions })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::manifest::Mode;
    use Direction::{ToRepo, ToTarget};
    use Kind::*;

    const A: Option<&str> = Some("a");
    const B: Option<&str> = Some("b");
    const C: Option<&str> = Some("c");
    const NONE: Option<&str> = None;

    #[test]
    fn truth_table() {
        // (target, repo, base, sync result, apply result)
        let cases = [
            (A, A, A, Unchanged, Unchanged),
            (NONE, NONE, A, Unchanged, Unchanged),
            (B, A, A, Update, Unchanged),
            (A, B, A, Unchanged, Update),
            (B, C, A, Conflict, Conflict),
            (NONE, A, A, Delete, Unchanged),
            (A, NONE, A, Unchanged, Delete),
            (A, NONE, NONE, Create, Unchanged),
            (NONE, A, NONE, Unchanged, Create),
            (A, B, NONE, Conflict, Conflict),
            (NONE, B, A, Conflict, Conflict),
            (B, NONE, A, Conflict, Conflict),
            (B, B, A, Unchanged, Unchanged),
        ];
        for (t, r, base, sync, apply) in cases {
            assert_eq!(decide(ToRepo, t, r, base), sync, "sync t={t:?} r={r:?} b={base:?}");
            assert_eq!(decide(ToTarget, t, r, base), apply, "apply t={t:?} r={r:?} b={base:?}");
        }
    }

    #[test]
    fn plan_over_real_files() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(home.join(".config/nvim/lua")).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(home.join(".config/nvim/init.lua"), "1").unwrap();
        std::fs::write(home.join(".config/nvim/lua/a.lua"), "2").unwrap();
        std::fs::write(home.join(".gitconfig"), "g").unwrap();

        let entries = vec![
            Entry { target: "~/.config/nvim".into(), repo: "nvim".into(), mode: Mode::Copy, sudo: false },
            Entry { target: "~/.gitconfig".into(), repo: "gitconfig".into(), mode: Mode::Copy, sudo: false },
        ];
        let state = State::default();

        let p = plan(&home, &repo, &entries, &state, ToRepo).unwrap();
        assert_eq!(p.actions.len(), 3);
        assert!(p.actions.iter().all(|a| a.kind == Create));
        assert_eq!(p.actions[0].display, "~/.config/nvim/init.lua");
        assert_eq!(p.actions[0].repo, repo.join("nvim/init.lua"));
        assert_eq!(p.actions[2].display, "~/.gitconfig");
        assert_eq!(p.actions[2].repo, repo.join("gitconfig"));

        // applying with nothing in the repo is a no-op: nothing to bring back
        let p = plan(&home, &repo, &entries, &state, ToTarget).unwrap();
        assert_eq!(p.pending().count(), 0);
    }

    #[test]
    fn file_vs_directory_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(repo.join("x")).unwrap();
        std::fs::write(home.join("x"), "file").unwrap();
        std::fs::write(repo.join("x/inner"), "dir").unwrap();
        let entries = vec![Entry { target: "~/x".into(), repo: "x".into(), mode: Mode::Copy, sudo: false }];
        let err = plan(&home, &repo, &entries, &State::default(), ToRepo).unwrap_err();
        assert!(err.to_string().contains("file on one side"), "{err}");
    }
}
