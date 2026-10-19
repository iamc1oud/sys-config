use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::manifest::{MANIFEST_FILE, expand};
use crate::error::Error;
use crate::filesystem::copier::write_atomic;

fn is_repo(dir: &Path) -> bool {
    dir.join(MANIFEST_FILE).is_file()
}

pub fn pointer_path(home: &Path) -> PathBuf {
    home.join(".config/smartrepo/repo")
}

pub fn write_pointer(home: &Path, repo: &Path) -> Result<()> {
    let path = pointer_path(home);
    write_atomic(&path, repo.display().to_string().as_bytes())
        .with_context(|| format!("recording repository location in {}", path.display()))
}

/// Find the SmartRepo repository in this order:
/// 1. Explicit `--repo` / `$SMARTREPO_DIR`
/// 2. Walk up from current directory
/// 3. Pointer file at ~/.config/smartrepo/repo
pub fn find_repo(explicit: Option<&Path>, cwd: &Path, home: &Path) -> Result<PathBuf> {
    // Explicit repo given, must be valid
    if let Some(p) = explicit {
        let dir = cwd.join(expand(home, p));
        return if is_repo(&dir) {
            Ok(dir)
        } else {
            Err(Error::RepoNotFound.into())
        };
    }

    // Walk up from current directory
    if let Some(dir) = cwd.ancestors().find(|d| is_repo(d)) {
        return Ok(dir.to_path_buf());
    }

    // Check pointer file
    if let Ok(text) = fs::read_to_string(pointer_path(home)) {
        let dir = PathBuf::from(text.trim());
        if is_repo(&dir) {
            return Ok(dir);
        }
    }

    Err(Error::RepoNotFound.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_then_ancestor_then_pointer() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let repo = tmp.path().join("repo");
        let nested = repo.join("a/b");
        let elsewhere = tmp.path().join("elsewhere");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(&elsewhere).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(repo.join(MANIFEST_FILE), "version = 1\n").unwrap();

        // nothing known yet
        assert!(find_repo(None, &elsewhere, &home).is_err());
        // ancestor walk
        assert_eq!(find_repo(None, &nested, &home).unwrap(), repo);
        // explicit wins, and must be a repo
        assert_eq!(find_repo(Some(&repo), &elsewhere, &home).unwrap(), repo);
        assert!(find_repo(Some(&elsewhere), &nested, &home).is_err());
        // pointer fallback
        write_pointer(&home, &repo).unwrap();
        assert_eq!(find_repo(None, &elsewhere, &home).unwrap(), repo);
    }
}
