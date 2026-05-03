use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NavMode {
    #[default]
    Picker,
    Direct,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub default_nav_mode: NavMode,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub portals: BTreeMap<String, String>,
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::home_dir()
            .expect("could not determine home directory")
            .join(".config")
            .join("tp")
            .join("config.toml")
    }

    // TODO: remove once all users have migrated (added alongside rename portals.toml -> config.toml)
    fn legacy_path() -> PathBuf {
        Self::path().with_file_name("portals.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() {
            // TODO: remove once all users have migrated
            let legacy = Self::legacy_path();
            if legacy.exists() {
                if let Err(e) = fs::rename(&legacy, &path) {
                    eprintln!("tp: could not rename portals.toml to config.toml: {}", e);
                } else {
                    eprintln!("tp: renamed ~/.config/tp/portals.toml -> ~/.config/tp/config.toml");
                }
            }
        }
        if !path.exists() {
            return Self::default();
        }
        let contents = fs::read_to_string(&path).expect("could not read config file");
        match toml::from_str(&contents) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error in config: {}", e);
                std::process::exit(1);
            }
        }
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
    fn nav_mode_picker_deserializes() {
        let s: Settings = toml::from_str("default_nav_mode = \"picker\"").unwrap();
        assert_eq!(s.default_nav_mode, NavMode::Picker);
    }

    #[test]
    fn nav_mode_direct_deserializes() {
        let s: Settings = toml::from_str("default_nav_mode = \"direct\"").unwrap();
        assert_eq!(s.default_nav_mode, NavMode::Direct);
    }

    #[test]
    fn settings_defaults_to_picker_when_absent() {
        let s: Settings = toml::from_str("").unwrap();
        assert_eq!(s.default_nav_mode, NavMode::Picker);
    }

    #[test]
    fn nav_mode_invalid_value_includes_bad_value_in_error() {
        let result: Result<Settings, _> = toml::from_str("default_nav_mode = \"blorp\"");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("blorp"), "error should mention bad value, got: {}", msg);
    }

    #[test]
    fn config_with_settings_section_deserializes() {
        let toml = "[settings]\ndefault_nav_mode = \"direct\"\n[portals]\nfoo = \"~/foo\"";
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.settings.default_nav_mode, NavMode::Direct);
        assert_eq!(config.portals.get("foo").unwrap(), "~/foo");
    }

    #[test]
    fn config_without_settings_section_defaults_to_picker() {
        let toml = "[portals]\nfoo = \"~/foo\"";
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.settings.default_nav_mode, NavMode::Picker);
    }

    #[test]
    fn broken_portals_finds_missing_dirs() {
        let existing = tempfile::tempdir().unwrap();
        let gone = tempfile::tempdir().unwrap();
        let missing_path = gone.path().display().to_string();
        drop(gone);

        let mut config = Config::default();
        config.add_portal(
            "good".to_string(),
            format!("{}", existing.path().display()),
        );
        config.add_portal("bad".to_string(), missing_path.clone());

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
