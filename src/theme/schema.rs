use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeSchema {
    pub theme: ThemeMeta,
    #[serde(default)]
    pub ui: UiSection,
    #[serde(default)]
    pub syntax: SyntaxSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeMeta {
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub variant: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UiSection {
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub cursor: Option<String>,
    pub selection: Option<String>,
    pub statusline_bg: Option<String>,
    pub statusline_fg: Option<String>,
    pub gutter: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SyntaxSection {
    pub keyword: Option<String>,
    pub function: Option<String>,
    pub string: Option<String>,
    pub number: Option<String>,
    pub comment: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub variable: Option<String>,
}

impl ThemeSchema {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("read {}: {}", path.display(), e))?;
        toml::from_str(&raw).map_err(|e| format!("parse {}: {}", path.display(), e))
    }

    pub fn from_str(raw: &str) -> Result<Self, String> {
        toml::from_str(raw).map_err(|e| format!("parse: {}", e))
    }
}