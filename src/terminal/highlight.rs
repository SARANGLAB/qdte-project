use once_cell::sync::Lazy;
use std::path::Path;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::highlighting::ThemeSet;

pub static PS: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
pub static TS: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

pub fn get_syntax_for_path(path: Option<&Path>) -> &'static SyntaxReference {
    if let Some(p) = path {
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if let Some(syn) = PS.find_syntax_by_extension(ext) {
                return syn;
            }
        }
    }
    PS.find_syntax_plain_text()
}