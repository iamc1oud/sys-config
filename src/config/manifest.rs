use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{error::Error, filesystem::copier::write_atomic};

pub const MANIFEST_FILE: &str = "sysconfig.toml";
pub const CURRENT_VERSION: u32 = 1;

const RESERVED: &[&str] = &[".git", ".gitignore", ".sysconfig", MANIFEST_FILE];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Independent copies on both sides (default and safest)
    #[default]
    Copy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Filesystem location, "~" allowed.
    pub target: String,

    /// Location inside the repository, relative.
    pub repo: String,

    #[serde(default)]
    pub mode: Mode,

    /// Recorded for later versions, no special behavior in v0.1.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sudo: bool,
}

impl Entry {
    pub fn target_path(&self, home: &Path) -> PathBuf {
        expand(home, &self.target)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub files: Vec<Entry>,
}

impl Manifest {
    pub fn new() -> Self {
        Manifest {
            version: CURRENT_VERSION,
            files: Vec::new(),
        }
    }

    pub fn load(repo: &Path) -> Result<Self> {
        let path = repo.join(MANIFEST_FILE);
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let manifest: Manifest =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        if manifest.version != CURRENT_VERSION {
            return Err(Error::ManifestVersion(manifest.version).into());
        }
        Ok(manifest)
    }

    pub fn save(&self, repo: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).context("serializing manifest")?;
        write_atomic(&repo.join(MANIFEST_FILE), text.as_bytes())
    }

    pub fn position(&self, home: &Path, target: &Path) -> Option<usize> {
        self.files
            .iter()
            .position(|e| e.target_path(home) == target)
    }
}

pub fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .context("$HOME is not set to an absolute path")
}

/// `~` and `~/x` become `home` and `home/x`; everything else is unchanged
pub fn expand(home: &Path, p: impl AsRef<Path>) -> PathBuf {
    let p = p.as_ref();
    match p.strip_prefix("~") {
        Ok(rest) if rest.as_os_str().is_empty() => home.to_path_buf(),
        Ok(rest) => home.join(rest),
        Err(_) => p.to_path_buf(),
    }
}

/// Inverse of `expand` for paths under `home`
pub fn collapse(home: &Path, p: &Path) -> String {
    match p.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => p.display().to_string(),
    }
}

/// Remove `.` and resolve `..` lexically
pub fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }

    out
}

/// User-typed path -> absolute normalized path
pub fn resolve(home: &Path, cwd: &Path, p: &Path) -> PathBuf {
    normalize(&cwd.join(expand(home, p)))
}

pub fn validate_repo_path(s: &str) -> Result<String> {
    let path = Path::new(s);
    let names: Vec<&str> = path
        .components()
        .map(|c| match c {
            Component::Normal(n) => n.to_str().ok_or(()),
            _ => Err(()),
        })
        .collect::<Result<_, _>>()
        .map_err(|()| Error::InvalidRepoPath(s.to_string()))?;

    match names.first() {
        Some(first) if !RESERVED.contains(first) => Ok(names.join("/")),
        _ => Err(Error::InvalidRepoPath(s.to_string()).into()),
    }
}

pub fn default_repo_path(home: &Path, target: &Path) -> Result<String> {
    let rel = match target.strip_prefix(home) {
        Ok(rest) => rest.strip_prefix(".config").unwrap_or(rest),
        Err(_) => target.strip_prefix("/").unwrap_or(target),
    };

    let mut names: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    if let Some(first) = names.first_mut() {
        if let Some(stripped) = first.strip_prefix(".") {
            *first = stripped.to_string();
        }
    }
    validate_repo_path(&names.join("/"))
}

// Test Cases
#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        PathBuf::from("/home/u")
    }

    #[test]
    fn expand_and_collapse_roundtrip() {
        let h = home();
        assert_eq!(expand(&h, "~/.gitconfig"), Path::new("/home/u/.gitconfig"));
        assert_eq!(expand(&h, "~"), h);
    }

    #[test]
    fn resolve_normalizes() {
        let h = home();
        let cwd = Path::new("/work/proj");
        assert_eq!(
            resolve(&h, cwd, Path::new("a/../b/./c")),
            Path::new("/work/proj/b/c")
        );
        assert_eq!(
            resolve(&h, cwd, Path::new("~/x/../y")),
            Path::new("/home/u/y")
        );
        assert_eq!(
            resolve(&h, cwd, Path::new("/etc/hosts")),
            Path::new("/etc/hosts")
        );
    }

    #[test]
    fn repo_path_validation() {
        assert_eq!(validate_repo_path("a/b/").unwrap(), "a/b");
        for bad in [
            "",
            "/abs",
            "../x",
            "a/../b",
            "./a",
            ".git/config",
            "sysconfig.toml",
            ".sysconfig/state.toml",
        ] {
            assert!(validate_repo_path(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn default_repo_paths() {
        let h = home();
        let f = |p: &str| default_repo_path(&h, Path::new(p)).unwrap();
        assert_eq!(f("/home/u/.config/helix/config.toml"), "helix/config.toml");
        assert_eq!(f("/home/u/.config/nvim"), "nvim");
        assert_eq!(f("/home/u/.gitconfig"), "gitconfig");
        assert_eq!(f("/home/u/Documents/notes.txt"), "Documents/notes.txt");
        assert_eq!(f("/etc/nginx/nginx.conf"), "etc/nginx/nginx.conf");
        assert!(default_repo_path(&h, Path::new("/home/u/.config")).is_err());
        assert!(default_repo_path(&h, &h).is_err());
    }

    #[test]
    fn manifest_roundtrip_matches_prd_shape() {
        let text = r#"
    version = 1

    [[files]]
    target = "~/.config/helix/config.toml"
    repo = "helix/config.toml"
    mode = "copy"

    [[files]]
    target = "/etc/nginx/nginx.conf"
    repo = "nginx/nginx.conf"
    sudo = true
    "#;
        let m: Manifest = toml::from_str(text).unwrap();
        assert_eq!(m.files.len(), 2);
        assert!(!m.files[0].sudo);
        assert!(m.files[1].sudo);
        assert_eq!(m.files[1].mode, Mode::Copy);

        let dir = tempfile::tempdir().unwrap();
        m.save(dir.path()).unwrap();
        let again = Manifest::load(dir.path()).unwrap();
        assert_eq!(again.files, m.files);
    }
}
