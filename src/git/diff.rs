use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;

use crate::error::Error;

/// Unified diff between two files (or /dev/null for missing sides).
/// Exit code 0 = identical, 1 = differences, other = error.
pub fn unified(old: Option<&Path>, new: Option<&Path>) -> Result<String> {
    let null = Path::new("/dev/null");
    let out = Command::new("git")
        .args(["diff", "--no-index", "--no-color", "--no-ext-diff", "--"])
        .arg(old.unwrap_or(null))
        .arg(new.unwrap_or(null))
        .output()
        .context("running git (is it installed and on PATH?)")?;
    match out.status.code() {
        Some(0 | 1) => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => Err(Error::Git(String::from_utf8_lossy(&out.stderr).trim().to_string()).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn shows_changes_and_missing_sides() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::write(&a, "one\n").unwrap();
        fs::write(&b, "two\n").unwrap();

        let d = unified(Some(&a), Some(&b)).unwrap();
        assert!(d.contains("-one") && d.contains("+two"), "{d}");

        let added = unified(None, Some(&b)).unwrap();
        assert!(added.contains("+two"), "{added}");

        assert_eq!(unified(Some(&a), Some(&a)).unwrap(), "");
    }
}
