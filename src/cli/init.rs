use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::discovery::write_pointer;
use crate::config::manifest::{MANIFEST_FILE, Manifest, home_dir, resolve};
use crate::config::state::STATE_DIR;
use crate::git::repository;

pub fn run(path: Option<PathBuf>) -> Result<()> {
    let home = home_dir()?;
    let cwd = std::env::current_dir()?;
    let dir = resolve(&home, &cwd, path.as_deref().unwrap_or(Path::new(".")));

    fs::create_dir_all(&dir)?;
    if dir.join(MANIFEST_FILE).exists() {
        println!("Existing SmartRepo found in {}", dir.display());
    } else {
        repository::init(&dir)?;
        Manifest::new().save(&dir)?;
        println!("Initialized empty SmartRepo in {}", dir.display());
    }
    ensure_gitignore(&dir)?;
    write_pointer(&home, &dir)?;
    Ok(())
}

fn ensure_gitignore(dir: &Path) -> Result<()> {
    let path = dir.join(".gitignore");
    let wanted = format!("{STATE_DIR}/");
    let mut text = fs::read_to_string(&path).unwrap_or_default();
    if text.lines().any(|l| l.trim() == wanted) {
        return Ok(());
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&wanted);
    text.push('\n');
    fs::write(&path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gitignore_is_created_once_and_appended_safely() {
        let dir = tempfile::tempdir().unwrap();
        ensure_gitignore(dir.path()).unwrap();
        ensure_gitignore(dir.path()).unwrap();
        let text = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert_eq!(text, ".sysconfig/\n");

        fs::write(dir.path().join(".gitignore"), "target").unwrap();
        ensure_gitignore(dir.path()).unwrap();
        let text = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert_eq!(text, "target\n.sysconfig/\n");
    }
}
