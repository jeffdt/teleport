use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tunnel {
    pub repo: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub portals: BTreeMap<String, String>,
    #[serde(default)]
    pub tunnels: BTreeMap<String, Tunnel>,
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

    pub fn add_tunnel(&mut self, name: String, repo: String, path: String) {
        self.tunnels.insert(name, Tunnel { repo, path });
    }

    pub fn remove(&mut self, name: &str) -> bool {
        if self.portals.remove(name).is_some() {
            return true;
        }
        self.tunnels.remove(name).is_some()
    }
}
