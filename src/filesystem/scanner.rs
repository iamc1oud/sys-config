use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::Result;
use walkdir::WalkDir;

use super::hashing::hash_file;

/// Relative path (`/`-separated, `""`for a single file) -> contain hash
pub type Snapshot = BTreeMap<String, String>;

const IGNORED: &[&str] = &[".git", ".DS_Store"];

/// `root.join(rel)`, except that the empty key means "root itself".
pub fn join_rel(root: &Path, rel: &str) -> PathBuf {
    if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    }
}

fn rel_keys(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn scan(root: &Path) -> Result<Snapshot> {
    let mut snap = Snapshot::new();

    let meta = match fs::metadata(root) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(snap),
        Err(e) => {
            return Err(anyhow::Error::new(e).context(format!("reading {}", root.display())));
        }
        Ok(meta) => meta,
    };

    if meta.is_file() {
        snap.insert(String::new(), hash_file(root)?);
        return Ok(snap);
    }

    let walker = WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !IGNORED.iter().any(|i| e.file_name() == *i));

    for entry in walker {
        let entry = entry?;
        let file_type = entry.file_type();

        if file_type.is_file() {
            let rel = entry.path().strip_prefix(root)?;
            snap.insert(rel_keys(rel), hash_file(entry.path())?);
        } else if file_type.is_symlink() {
            eprintln!("Warning: skipping symlink {}", entry.path().display());
        }
    }

    Ok(snap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_is_empty() {
        assert!(scan(Path::new("/definitely/not/here")).unwrap().is_empty());
    }

    #[test]
    fn single_file_uses_empty_key() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        fs::write(&file, "x").unwrap();
        let snap = scan(&file).unwrap();
        assert_eq!(snap.keys().collect::<Vec<_>>(), [""]);
    }

    #[test]
    fn directory_is_nested_and_filtered() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("lua/plugins")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("init.lua"), "1").unwrap();
        fs::write(root.join("lua/plugins/a.lua"), "2").unwrap();
        fs::write(root.join(".git/HEAD"), "ref").unwrap();
        fs::write(root.join(".DS_Store"), "junk").unwrap();

        let snap = scan(root).unwrap();
        let keys: Vec<_> = snap.keys().map(String::as_str).collect();
        assert_eq!(keys, ["init.lua", "lua/plugins/a.lua"]);
    }

    #[cfg(unix)]
    #[test]
    fn root_symlink_is_followed_inner_symlink_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        fs::create_dir(&real).unwrap();
        fs::write(real.join("a"), "1").unwrap();
        std::os::unix::fs::symlink(real.join("a"), real.join("link")).unwrap();
        let link_root = dir.path().join("root");
        std::os::unix::fs::symlink(&real, &link_root).unwrap();

        let snap = scan(&link_root).unwrap();
        assert_eq!(snap.keys().collect::<Vec<_>>(), ["a"]);
    }

    #[test]
    fn join_rel_handles_empty_keys() {
        let root = Path::new("/r/file");
        assert_eq!(join_rel(root, ""), root);
        assert_eq!(join_rel(Path::new("/r"), "a/b"), Path::new("/r/a/b"));
    }
}
