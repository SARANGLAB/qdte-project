use std::path::PathBuf;

use serde::{Deserialize, Serialize}; // Tambahkan Serialize

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)] // Tambahkan Serialize
#[serde(rename_all = "lowercase")]
pub enum KeymapKind {
    Normal,
    Vim,
}

#[derive(Debug, Clone, Deserialize, Serialize)] // Tambahkan Serialize
#[serde(default)]
pub struct Config {
    pub tab_width: usize,
    pub expand_tab: bool,
    pub fixeol: bool,
    pub keymap: Option<KeymapKind>,
    pub theme: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tab_width: 4,
            expand_tab: false,
            fixeol: false,
            keymap: None,
            theme: None,
        }
    }
}

impl Config {
    pub fn load() -> (Self, Option<String>) {
        let Some(path) = config_path() else {
            return (Self::default(), None);
        };

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return (Self::default(), None);
            }
            Err(e) => {
                return (
                    Self::default(),
                    Some(format!("could not read {}: {}", path.display(), e)),
                );
            }
        };

        match toml::from_str::<Config>(&raw) {
            Ok(cfg) => (cfg, None),
            Err(e) => (
                Self::default(),
                Some(format!("config parse error in {}: {}", path.display(), e)),
            ),
        }
    }

    pub fn save_default(&self) -> std::io::Result<()> {
        let path = config_path().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "config path not found")
        })?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn path() -> Option<PathBuf> {
        config_path()
    }
}

fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(|p| PathBuf::from(p).join("qdte").join("config.toml"))
    }

    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("qdte")
                .join("config.toml")
        })
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|p| p.join("qdte").join("config.toml"))
    }
}