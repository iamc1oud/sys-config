use std::{fs, io, path::Path};

use anyhow::{Context, Result};

///
fn atomic(to: &Path, fill: impl FnOnce(&Path) -> io::Result<()>) -> Result<()> {
    let parent = to
        .parent()
        .with_context(|| format!("{} has no parent directory", to.display()))?;

    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;

    let name = to
        .file_name()
        .with_context(|| format!("{} has no file name", to.display()))?
        .to_string_lossy();

    let tmp = parent.join(format!(".{name}.sys-config"));

    fill(&tmp).and_then(|()| fs::rename(&tmp, to)).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        anyhow::Error::new(e).context(format!("writing {}", to.display()))
    })
}

/// Copy `from` over `to`atomically, preserving permission bits.
pub fn copy_file(from: &Path, to: &Path) -> Result<()> {
    atomic(to, |tmp| fs::copy(from, tmp).map(|_| ()))
}

/// Writes bytes to `to` atomically
pub fn write_atomic(to: &Path, bytes: &[u8]) -> Result<()> {
    atomic(to, |tmp| fs::write(tmp, bytes))
}

/// Delete `path`, then delete empty ancestor directories up to (excluding) `stop`.
pub fn remove_file_prune(path: &Path, stop: &Path) -> Result<()> {
    fs::remove_file(path).with_context(|| format!("removing {}", path.display()))?;
    let mut cur = path;
    while let Some(dir) = cur.parent() {
        if dir == stop || !dir.starts_with(stop) || fs::remove_dir(dir).is_err() {
            break;
        }
        cur = dir;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_creates_parents_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        fs::write(&src, "new").unwrap();
        let dst = dir.path().join("a/b/c/dst");

        copy_file(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(&dst).unwrap(), "new");

        fs::write(&src, "newer").unwrap();
        copy_file(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(&dst).unwrap(), "newer");
        // no temp file left behind
        assert_eq!(fs::read_dir(dst.parent().unwrap()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn copy_preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("secret");
        fs::write(&src, "x").unwrap();
        fs::set_permissions(&src, fs::Permissions::from_mode(0o600)).unwrap();
        let dst = dir.path().join("out/secret");
        copy_file(&src, &dst).unwrap();
        let mode = fs::metadata(&dst).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn failed_copy_leaves_no_temp() {
        let dir = tempfile::tempdir().unwrap();
        let dst = dir.path().join("dst");
        assert!(copy_file(&dir.path().join("missing"), &dst).is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn remove_prunes_empty_dirs_but_not_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let file = root.join("x/y/f");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "1").unwrap();
        fs::write(root.join("keep"), "1").unwrap();

        remove_file_prune(&file, &root).unwrap();
        assert!(!root.join("x").exists());
        assert!(root.exists());
    }

    #[test]
    fn remove_single_file_entry_stops_at_itself() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sub/f");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "1").unwrap();
        // for single-file entries the root *is* the file
        remove_file_prune(&file, &file).unwrap();
        assert!(dir.path().join("sub").exists());
    }
}
