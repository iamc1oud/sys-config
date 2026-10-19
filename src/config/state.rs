use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::filesystem::copier::write_atomic;

pub const STATE_DIR: &str = ".smartrepo";
const STATE_FILE: &str = "state.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    /// entry repo path -> (relative file key -> base hash)
    #[serde(default)]
    bases: BTreeMap<String, BTreeMap<String, String>>,
}

impl State {
    pub fn load(repo: &Path) -> Result<Self> {
        let path = repo.join(STATE_DIR).join(STATE_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => {
                toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(State::default()),
            Err(e) => Err(anyhow::Error::new(e).context(format!("reading {}", path.display()))),
        }
    }

    pub fn save(&self, repo: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).context("serializing state")?;
        write_atomic(&repo.join(STATE_DIR).join(STATE_FILE), text.as_bytes())
    }

    pub fn get(&self, entry: &str, rel: &str) -> Option<&str> {
        self.bases.get(entry)?.get(rel).map(String::as_str)
    }

    pub fn set(&mut self, entry: &str, rel: &str, hash: String) {
        self.bases
            .entry(entry.to_string())
            .or_default()
            .insert(rel.to_string(), hash);
    }

    pub fn remove(&mut self, entry: &str, rel: &str) {
        if let Some(files) = self.bases.get_mut(entry) {
            files.remove(rel);
            if files.is_empty() {
                self.bases.remove(entry);
            }
        }
    }

    /// Relative keys currently recorded for an entry.
    pub fn rels(&self, entry: &str) -> Vec<String> {
        self.bases
            .get(entry)
            .map(|files| files.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn forget_entry(&mut self, entry: &str) {
        self.bases.remove(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(State::load(dir.path()).unwrap().get("a", ""), None);
    }

    #[test]
    fn roundtrip_including_empty_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = State::default();
        s.set("helix/config.toml", "", "h1".into());
        s.set("nvim", "lua/plugins/a.lua", "h2".into());
        s.save(dir.path()).unwrap();

        let loaded = State::load(dir.path()).unwrap();
        assert_eq!(loaded.get("helix/config.toml", ""), Some("h1"));
        assert_eq!(loaded.get("nvim", "lua/plugins/a.lua"), Some("h2"));
        assert_eq!(loaded.get("nvim", "nope"), None);
    }

    #[test]
    fn remove_and_forget() {
        let mut s = State::default();
        s.set("e", "a", "1".into());
        s.set("e", "b", "2".into());
        s.remove("e", "a");
        assert_eq!(s.rels("e"), ["b"]);
        s.forget_entry("e");
        assert!(s.rels("e").is_empty());
    }
}
