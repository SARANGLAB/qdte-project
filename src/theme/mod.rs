pub mod builtin;
pub mod schema;

use std::path::PathBuf;

use crossterm::style::Color;
use syntect::highlighting::{
    Color as SynColor, ScopeSelectors, StyleModifier, Theme as SynTheme, ThemeItem,
};

pub use schema::ThemeSchema;

#[derive(Debug, Clone)]
pub struct UiColors {
    pub fg: Color,
    pub bg: Color,
    pub cursor: Color,
    pub selection: Color,
    pub statusbar_bg: Color,
    pub statusbar_fg: Color,
    pub gutter: Color,
}

pub struct Theme {
    pub name: String,
    pub ui: UiColors,
    pub syntect: SynTheme,
}

impl Theme {
    pub fn from_schema(schema: &ThemeSchema) -> Self {
        let ui = UiColors {
            fg: parse_ct(schema.ui.foreground.as_deref()).unwrap_or(Color::White),
            bg: parse_ct(schema.ui.background.as_deref()).unwrap_or(Color::Black),
            cursor: parse_ct(schema.ui.cursor.as_deref()).unwrap_or(Color::White),
            selection: parse_ct(schema.ui.selection.as_deref()).unwrap_or(Color::DarkGrey),
            statusbar_bg: parse_ct(schema.ui.statusline_bg.as_deref())
                .unwrap_or(Color::DarkGrey),
            statusbar_fg: parse_ct(schema.ui.statusline_fg.as_deref()).unwrap_or(Color::White),
            gutter: parse_ct(schema.ui.gutter.as_deref()).unwrap_or(Color::DarkGrey),
        };

        let syntect = build_syntect_theme(schema);
        Self { name: schema.theme.name.clone(), ui, syntect }
    }

    pub fn builtin_default() -> Self {
        let schema = ThemeSchema::from_str(builtin::DEFAULT_THEME_TOML)
            .expect("builtin theme must parse");
        Self::from_schema(&schema)
    }
}

pub struct ThemeManager {
    themes: Vec<Theme>,
    active_idx: usize,
}

impl ThemeManager {
    pub fn load(active_name: Option<&str>) -> Self {
        let mut themes = vec![Theme::builtin_default()];
        let mut errors: Vec<String> = Vec::new();

        if let Some(dir) = themes_dir() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                        match ThemeSchema::from_file(&path) {
                            Ok(schema) => themes.push(Theme::from_schema(&schema)),
                            Err(e) => errors.push(e),
                        }
                    }
                }
            }
        }

        for e in &errors {
            eprintln!("qdte: skipped theme: {}", e);
        }

        let active_idx = active_name
            .and_then(|name| themes.iter().position(|t| t.name.eq_ignore_ascii_case(name)))
            .unwrap_or(0);

        Self { themes, active_idx }
    }

    pub fn active(&self) -> &Theme {
        &self.themes[self.active_idx]
    }

    pub fn list_names(&self) -> Vec<&str> {
        self.themes.iter().map(|t| t.name.as_str()).collect()
    }

    pub fn set_by_name(&mut self, name: &str) -> Result<(), String> {
        if let Some(idx) = self
            .themes
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(name))
        {
            self.active_idx = idx;
            Ok(())
        } else {
            Err(format!("theme not found: {}", name))
        }
    }
}

pub fn themes_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(|p| PathBuf::from(p).join("qdte").join("themes"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("qdte")
                .join("themes")
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|p| p.join("qdte").join("themes"))
    }
}

// ---- color helpers --------------------------------------------------------

fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

fn parse_ct(hex: Option<&str>) -> Option<Color> {
    let (r, g, b) = parse_hex(hex?)?;
    Some(Color::Rgb { r, g, b })
}

fn parse_syn(hex: &str) -> Option<SynColor> {
    let (r, g, b) = parse_hex(hex)?;
    Some(SynColor { r, g, b, a: 255 })
}

fn build_syntect_theme(schema: &ThemeSchema) -> SynTheme {
    let ts = syntect::highlighting::ThemeSet::load_defaults();
    let mut theme = ts.themes["base16-ocean.dark"].clone();

    theme.name = Some(schema.theme.name.clone());
    theme.author = Some(schema.theme.author.clone());

    if let Some(c) = parse_ct(schema.ui.foreground.as_deref()) {
        if let Color::Rgb { r, g, b } = c {
            theme.settings.foreground = Some(SynColor { r, g, b, a: 255 });
        }
    }
    if let Some(c) = parse_ct(schema.ui.background.as_deref()) {
        if let Color::Rgb { r, g, b } = c {
            theme.settings.background = Some(SynColor { r, g, b, a: 255 });
        }
    }

    let mut add = |sel: &str, hex: Option<&str>| {
        if let Some(color) = hex.and_then(parse_syn) {
            if let Ok(selector) = sel.parse::<ScopeSelectors>() {
                // PUSH ke urutan paling akhir agar menimpa warna bawaan base16
                theme.scopes.push(ThemeItem {
                    scope: selector,
                    style: StyleModifier {
                        foreground: Some(color),
                        background: None,
                        font_style: None,
                    },
                });
            }
        }
    };

    // Tambahkan cakupan yang lebih luas untuk semua bahasa
    add("keyword, storage, storage.type, storage.modifier, keyword.control, keyword.other", schema.syntax.keyword.as_deref());
    add("entity.name.function, support.function, variable.function, meta.function-call", schema.syntax.function.as_deref());
    add("string, string.quoted, string.unquoted, constant.character, constant.other.symbol", schema.syntax.string.as_deref());
    add("constant.numeric, constant.language, constant.other", schema.syntax.number.as_deref());
    add("comment, comment.line, comment.block, punctuation.definition.comment", schema.syntax.comment.as_deref());
    add("entity.name.type, support.type, entity.name.class, storage.type.class, entity.name.struct, entity.name.enum, entity.name.namespace, entity.name.type.alias", schema.syntax.type_.as_deref());
    add("variable, variable.other, variable.parameter, entity.name.tag", schema.syntax.variable.as_deref());

    theme
}