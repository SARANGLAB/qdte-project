use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeymapKind {
    Normal,
    Vim,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Number of visual columns per tab.
    pub tab_width: usize,

    /// If true, Tab inserts `tab_width` spaces instead of a literal '\t'.
    /// Note: fixed count, not tab-stop-aware.
    pub expand_tab: bool,

    /// If true, a trailing '\n' is appended on save when the buffer
    /// does not already end with one.
    ///
    /// Caveat: QDTE treats '\n' as a line separator, not terminator.
    /// With fixeol = true, saving "hello" writes "hello\n"; reopening
    /// shows an extra empty last line.
    pub fixeol: bool,

    /// Override the keymap selected by argv[0].
    /// None = infer from invocation name (vim/nvim/vi → Vim; else Normal).
    pub keymap: Option<KeymapKind>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tab_width: 4,
            expand_tab: false,
            fixeol: false,
            keymap: None,
        }
    }
}

impl Config {
    /// Load configuration from the platform default path.
    ///
    /// Returns (config, warning). If the file is missing, returns defaults
    /// with no warning. If the file exists but cannot be parsed, returns
    /// defaults with a warning message.
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

    /// Path where QDTE looks for its config file, if resolvable.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = Config::default();
        assert_eq!(c.tab_width, 4);
        assert!(!c.expand_tab);
        assert!(!c.fixeol);
        assert!(c.keymap.is_none());
    }

    #[test]
    fn parses_full_config() {
        let toml = r#"
            tab_width = 2
            expand_tab = true
            fixeol = true
            keymap = "vim"
        "#;
        let c: Config = toml::from_str(toml).unwrap();
        assert_eq!(c.tab_width, 2);
        assert!(c.expand_tab);
        assert!(c.fixeol);
        assert_eq!(c.keymap, Some(KeymapKind::Vim));
    }

    #[test]
    fn parses_partial_config() {
        let toml = r#"tab_width = 8"#;
        let c: Config = toml::from_str(toml).unwrap();
        assert_eq!(c.tab_width, 8);
        assert!(!c.expand_tab);
        assert!(!c.fixeol);
        assert!(c.keymap.is_none());
    }

    #[test]
    fn parses_empty_as_default() {
        let c: Config = toml::from_str("").unwrap();
        assert_eq!(c.tab_width, 4);
    }

    #[test]
    fn keymap_normal_parses() {
        let c: Config = toml::from_str(r#"keymap = "normal""#).unwrap();
        assert_eq!(c.keymap, Some(KeymapKind::Normal));
    }

    #[test]
    fn invalid_toml_fails() {
        let result = toml::from_str::<Config>("tab_width = \"not a number\"");
        assert!(result.is_err());
    }

    #[test]
    fn config_path_is_resolved_on_this_platform() {
        // Sanity check: on every supported platform, we expect some path.
        assert!(config_path().is_some());
    }
}