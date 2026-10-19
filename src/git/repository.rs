use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;

use crate::error::Error;

/// Run `git <args>` in `cwd`, returning stdout.
pub fn git(cwd: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .context("running git (is it installed and on PATH?)")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(Error::Git(stderr).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Initialize a new git repository with `git init --quiet`
pub fn init(dir: &Path) -> Result<()> {
    git(dir, &["init", "--quiet"]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_dot_git() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        assert!(dir.path().join(".git").exists());
    }

    #[test]
    fn failing_command_reports_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let err = git(dir.path(), &["status"]).unwrap_err().to_string();
        assert!(err.starts_with("git failed:"), "{err}");
    }
}
