use once_cell::sync::Lazy;
use std::path::Path;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub static PS: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
pub static TS: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

pub fn get_syntax_for_path(path: Option<&Path>) -> &'static SyntaxReference {
    if let Some(p) = path {
        // 1. Paksa file .lock menjadi TOML
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if ext == "lock" {
                if let Some(syn) = PS.find_syntax_by_extension("toml").or_else(|| PS.find_syntax_by_name("TOML")) {
                    return syn;
                }
            }
            // 2. Ekstensi biasa (.rs, .py, .toml, dll)
            if let Some(syn) = PS.find_syntax_by_extension(ext) {
                return syn;
            }
        }

        // 3. Deteksi nama file tanpa ekstensi
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            match name {
                "Cargo.lock" | "Cargo.toml" => {
                    if let Some(syn) = PS.find_syntax_by_extension("toml").or_else(|| PS.find_syntax_by_name("TOML")) {
                        return syn;
                    }
                }
                "Dockerfile" => {
                    if let Some(syn) = PS.find_syntax_by_name("Dockerfile") {
                        return syn;
                    }
                }
                "Makefile" => {
                    if let Some(syn) = PS.find_syntax_by_name("Makefile") {
                        return syn;
                    }
                }
                ".gitignore" => {
                    if let Some(syn) = PS.find_syntax_by_name("Git Ignore") {
                        return syn;
                    }
                }
                _ => {}
            }
        }
    }
    
    PS.find_syntax_plain_text()
}