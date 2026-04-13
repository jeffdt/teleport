use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub portals: BTreeMap<String, String>,
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::home_dir()
            .expect("could not determine home directory")
            .join(".config")
            .join("tp")
            .join("portals.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() {
            return Self::default();
        }
        let contents = fs::read_to_string(&path).expect("could not read config file");
        toml::from_str(&contents).expect("could not parse config file")
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("could not create config directory");
        }
        let contents = toml::to_string_pretty(self).expect("could not serialize config");
        fs::write(&path, contents).expect("could not write config file");
    }

    pub fn add_portal(&mut self, name: String, path: String) {
        self.portals.insert(name, path);
    }

    pub fn remove(&mut self, name: &str) -> bool {
        self.portals.remove(name).is_some()
    }

    pub fn broken_portals(&self) -> Vec<(String, String)> {
        self.portals
            .iter()
            .filter(|(_, path)| !crate::resolve::expand_tilde(path).is_dir())
            .map(|(name, path)| (name.clone(), path.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn broken_portals_finds_missing_dirs() {
        let existing = tempfile::tempdir().unwrap();
        let missing_path = "/tmp/tp-test-nonexistent-dir-abc123";

        let mut config = Config::default();
        config.add_portal(
            "good".to_string(),
            format!("{}", existing.path().display()),
        );
        config.add_portal("bad".to_string(), missing_path.to_string());

        let broken = config.broken_portals();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].0, "bad");
        assert_eq!(broken[0].1, missing_path);
    }

    #[test]
    fn broken_portals_empty_when_all_valid() {
        let existing = tempfile::tempdir().unwrap();

        let mut config = Config::default();
        config.add_portal(
            "good".to_string(),
            format!("{}", existing.path().display()),
        );

        let broken = config.broken_portals();
        assert!(broken.is_empty());
    }

    #[test]
    fn broken_portals_detects_file_not_dir() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("not-a-dir");
        fs::write(&file_path, "i am a file").unwrap();

        let mut config = Config::default();
        config.add_portal(
            "file-portal".to_string(),
            format!("{}", file_path.display()),
        );

        let broken = config.broken_portals();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].0, "file-portal");
    }
}
