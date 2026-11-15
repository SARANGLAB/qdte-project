use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use thiserror::Error;

use crate::config::Config;
use crate::editor::{Action, Editor, EditorError, Mode};
use crate::io::{self, IoError, LineEnding};
use crate::theme::ThemeManager;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Editor(#[from] EditorError),

    #[error(transparent)]
    Io(#[from] IoError),

    #[error("no file path set; open QDTE with a filename to enable save")]
    NoPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Open,
    SaveAs,
    Command,
    Search,
}

impl PromptKind {
    pub fn label(&self) -> &'static str {
        match self {
            PromptKind::Open => "Open",
            PromptKind::SaveAs => "Save As",
            PromptKind::Command => "Cmd",
            PromptKind::Search => "Search",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Prompt {
    pub kind: PromptKind,
    pub input: String,
    pub cursor: usize,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub current_idx: usize,
}

#[derive(Debug, Clone)]
pub struct ThemePickerState {
    pub themes: Vec<String>,
    pub selected_idx: usize,
}

impl Prompt {
    pub fn new(kind: PromptKind, initial: String) -> Self {
        let cursor = initial.chars().count();
        Self {
            kind,
            input: initial,
            cursor,
            note: None,
        }
    }

    pub fn input_len(&self) -> usize {
        self.input.chars().count()
    }

    pub fn insert_char(&mut self, c: char) {
        let byte_idx = self.char_to_byte(self.cursor);
        self.input.insert(byte_idx, c);
        self.cursor += 1;
        self.note = None;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let byte_idx = self.char_to_byte(self.cursor - 1);
        let next_byte = self.char_to_byte(self.cursor);
        self.input.replace_range(byte_idx..next_byte, "");
        self.cursor -= 1;
        self.note = None;
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.input_len() {
            return;
        }
        let byte_idx = self.char_to_byte(self.cursor);
        let next_byte = self.char_to_byte(self.cursor + 1);
        self.input.replace_range(byte_idx..next_byte, "");
        self.note = None;
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.input_len() {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.input_len();
    }

    fn char_to_byte(&self, char_idx: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.input.len())
    }
}

pub struct App {
    pub editor: Editor,
    pub path: Option<PathBuf>,
    pub line_ending: LineEnding,
    pub had_bom: bool,
    pub message: Option<String>,
    pub mode: Mode,
    pub config: Config,
    pub prompt: Option<Prompt>,
    pub selection_anchor: Option<usize>,
    pub should_quit: bool,
    pub search: Option<SearchState>,
    pub cursor_before_search: Option<usize>,
    pub theme_manager: ThemeManager,
    pub theme_picker: Option<ThemePickerState>,
    saved_hash: Option<u64>,
    pub pending_shell_command: Option<String>,
}

impl App {
    pub fn new_empty(config: Config) -> Self {
        let mut editor = Editor::new();
        editor.set_tab_config(config.tab_width, config.expand_tab);
        let saved_hash = Some(hash_str(&editor.buffer().as_string()));
        let theme_manager = ThemeManager::load(config.theme.as_deref());
        
        Self {
            editor,
            path: None,
            line_ending: LineEnding::default_for_platform(),
            had_bom: false,
            message: None,
            mode: Mode::Normal,
            config,
            prompt: None,
            selection_anchor: None,
            should_quit: false,
            search: None,
            cursor_before_search: None,
            theme_manager,
            theme_picker: None,
            saved_hash,
            pending_shell_command: None,
        }
    }

    pub fn from_file(path: PathBuf, config: Config) -> Result<Self, IoError> {
        let loaded = io::reader::read(&path)?;
        let mut editor = Editor::from_str(&loaded.content);
        editor.set_tab_config(config.tab_width, config.expand_tab);
        let saved_hash = Some(hash_str(&editor.buffer().as_string()));
        let theme_manager = ThemeManager::load(config.theme.as_deref());
        
        Ok(Self {
            editor,
            path: Some(path),
            line_ending: loaded.line_ending,
            had_bom: loaded.had_bom,
            message: None,
            mode: Mode::Normal,
            config,
            prompt: None,
            selection_anchor: None,
            should_quit: false,
            search: None,
            cursor_before_search: None,
            theme_manager,
            theme_picker: None,
            saved_hash,
            pending_shell_command: None,
        })
    }

    pub fn is_dirty(&self) -> bool {
        match self.saved_hash {
            Some(saved) => saved != self.editor.hash(),
            None => self.editor.buffer().len_chars() > 0,
        }
    }

    pub fn apply_editor(&mut self, action: Action) -> Result<(), AppError> {
        self.editor.apply(action)?;
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), AppError> {
        let path = self.path.clone().ok_or(AppError::NoPath)?;
        let mut content = self.editor.buffer().as_string();
        if self.config.fixeol && !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        io::writer::write(&path, &content, self.line_ending, self.had_bom)?;
        self.saved_hash = Some(hash_str(&self.editor.buffer().as_string()));
        self.message = Some(format!("saved: {}", path.display()));
        Ok(())
    }

    pub fn open_prompt(&mut self, kind: PromptKind) {
        self.message = None;
        let initial = match kind {
            PromptKind::SaveAs => self
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            PromptKind::Open | PromptKind::Command | PromptKind::Search => String::new(),
        };
        let mut prompt = Prompt::new(kind, initial);
        
        if kind == PromptKind::Open && self.is_dirty() {
            prompt.note = Some("unsaved changes will be lost".to_string());
        }
        
        if kind == PromptKind::Search {
            self.cursor_before_search = Some(self.editor.cursor().char_idx());
        }
        
        self.prompt = Some(prompt);
    }

    pub fn cancel_prompt(&mut self) {
        if let Some(prompt) = &self.prompt {
            if prompt.kind == PromptKind::Search {
                if let Some(c) = self.cursor_before_search {
                    self.editor.set_cursor(c);
                }
                self.search = None;
            }
        }
        self.prompt = None;
        self.cursor_before_search = None;
    }

    pub fn visual_range(&self) -> Option<(usize, usize, bool)> {
        let anchor = self.selection_anchor?;
        let cursor_idx = self.editor.cursor().char_idx();
        let buffer = self.editor.buffer();
        let linewise = matches!(self.mode, Mode::VisualLine);

        if linewise {
            let anchor_line = buffer.char_to_line(anchor);
            let cursor_line = buffer.char_to_line(cursor_idx);
            let (start_line, end_line) = if anchor_line <= cursor_line {
                (anchor_line, cursor_line)
            } else {
                (cursor_line, anchor_line)
            };
            let start = buffer.line_to_char(start_line);
            let end = if end_line + 1 < buffer.len_lines() {
                buffer.line_to_char(end_line + 1)
            } else {
                buffer.len_chars()
            };
            Some((start, end, true))
        } else {
            let (lo, hi) = if anchor <= cursor_idx {
                (anchor, cursor_idx)
            } else {
                (cursor_idx, anchor)
            };
            let hi_end = crate::editor::Cursor::at(hi)
                .next_grapheme_end(buffer)
                .unwrap_or(hi + 1);
            let end = hi_end.min(buffer.len_chars()).max(hi + 1).min(buffer.len_chars());
            Some((lo, end, false))
        }
    }

    pub fn confirm_prompt(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        let path_str = prompt.input.trim().to_string();
        if path_str.is_empty() {
            return;
        }
        let path = PathBuf::from(&path_str);

        match prompt.kind {
            PromptKind::Open => {
                let existed = path.exists();
                if !existed && self.is_dirty() {
                    let mut prompt = prompt;
                    prompt.note = Some("file does not exist; save current buffer first".to_string());
                    self.prompt = Some(prompt);
                    return;
                }
                match Self::from_file(path.clone(), self.config.clone()) {
                    Ok(new_app) => {
                        self.editor = new_app.editor;
                        self.path = new_app.path;
                        self.line_ending = new_app.line_ending;
                        self.had_bom = new_app.had_bom;
                        self.saved_hash = new_app.saved_hash;
                        self.mode = Mode::Normal;
                        self.selection_anchor = None;
                        self.message = Some(format!("opened: {}", path.display()));
                    }
                    Err(e) => {
                        let mut prompt = prompt;
                        prompt.note = Some(format!("open failed: {}", e));
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::SaveAs => {
                let content = self.editor.buffer().as_string();
                let content_to_write =
                    if self.config.fixeol && !content.is_empty() && !content.ends_with('\n') {
                        let mut s = content.clone();
                        s.push('\n');
                        s
                    } else {
                        content.clone()
                    };
                match io::writer::write(&path, &content_to_write, self.line_ending, self.had_bom) {
                    Ok(()) => {
                        self.path = Some(path.clone());
                        self.saved_hash = Some(hash_str(&content));
                        self.message = Some(format!("saved: {}", path.display()));
                    }
                    Err(e) => {
                        let mut prompt = prompt;
                        prompt.note = Some(format!("save failed: {}", e));
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::Command => {
                self.execute_command(&path_str);
            }
            PromptKind::Search => {
                self.cursor_before_search = None; 
                self.execute_search(&path_str);
                if self.search.is_none() {
                    self.message = Some(format!("pattern not found: {}", path_str));
                }
            }
        }
    }

    pub fn set_theme(&mut self, name: &str) -> Result<(), String> {
        self.theme_manager.set_by_name(name)?;
        self.config.theme = Some(self.theme_manager.active().name.clone());
        if let Err(e) = self.config.save_default() {
            self.message = Some(format!("warning: could not persist theme: {}", e));
        }
        self.message = Some(format!("theme: {}", self.theme_manager.active().name));
        Ok(())
    }

    pub fn list_themes(&self) -> Vec<&str> {
        self.theme_manager.list_names()
    }

    // --- THEME PICKER METHODS ---
    pub fn open_theme_picker(&mut self) {
        let themes: Vec<String> = self.theme_manager.list_names().iter().map(|s| s.to_string()).collect();
        let selected_idx = themes.iter().position(|t| t == self.theme_manager.active().name.as_str()).unwrap_or(0);
        self.theme_picker = Some(ThemePickerState { themes, selected_idx });
        self.message = None;
    }

    pub fn close_theme_picker(&mut self) {
        self.theme_picker = None;
    }

    pub fn theme_picker_up(&mut self) {
        if let Some(picker) = self.theme_picker.as_mut() {
            if picker.selected_idx > 0 {
                picker.selected_idx -= 1;
            } else {
                picker.selected_idx = picker.themes.len().saturating_sub(1);
            }
        }
    }

    pub fn theme_picker_down(&mut self) {
        if let Some(picker) = self.theme_picker.as_mut() {
            if picker.selected_idx + 1 < picker.themes.len() {
                picker.selected_idx += 1;
            } else {
                picker.selected_idx = 0;
            }
        }
    }

    pub fn theme_picker_select(&mut self) {
        if let Some(picker) = self.theme_picker.take() {
            if let Some(theme_name) = picker.themes.get(picker.selected_idx) {
                let _ = self.set_theme(theme_name);
            }
        }
    }
    // ----------------------------

    pub fn update_live_search(&mut self) {
        let query = if let Some(prompt) = &self.prompt {
            if prompt.kind == PromptKind::Search {
                Some(prompt.input.clone())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(q) = query {
            self.execute_search(&q);
        }
    }

    pub fn execute_search(&mut self, query: &str) {
        if query.is_empty() {
            self.search = None;
            return;
        }

        let buffer = self.editor.buffer();
        let content = buffer.as_string();
        let chars: Vec<char> = content.chars().collect();
        let q_chars: Vec<char> = query.chars().collect();
        let q_len = q_chars.len();

        let mut matches = Vec::new();

        if q_len > 0 && q_len <= chars.len() {
            for i in 0..=chars.len() - q_len {
                let mut is_match = true;
                for j in 0..q_len {
                    if !chars[i + j].to_lowercase().eq(q_chars[j].to_lowercase()) {
                        is_match = false;
                        break;
                    }
                }
                if is_match {
                    matches.push(SearchMatch { start: i, end: i + q_len });
                }
            }
        }

        if matches.is_empty() {
            self.search = None;
            return;
        }

        let cursor = if let Some(c) = self.cursor_before_search {
            c
        } else {
            self.editor.cursor().char_idx()
        };

        let mut current_idx = 0;
        for (i, m) in matches.iter().enumerate() {
            if m.start >= cursor {
                current_idx = i;
                break;
            }
        }

        self.search = Some(SearchState {
            query: query.to_string(),
            matches,
            current_idx,
        });

        self.jump_to_match();
    }

    pub fn search_next(&mut self) {
        if let Some(search) = self.search.as_mut() {
            search.current_idx = (search.current_idx + 1) % search.matches.len();
            self.jump_to_match();
        }
    }

    pub fn search_prev(&mut self) {
        if let Some(search) = self.search.as_mut() {
            if search.current_idx == 0 {
                search.current_idx = search.matches.len() - 1;
            } else {
                search.current_idx -= 1;
            }
            self.jump_to_match();
        }
    }

    fn jump_to_match(&mut self) {
        if let Some(search) = &self.search {
            let start = search.matches[search.current_idx].start;
            self.editor.set_cursor(start);
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        if let Some(shell_cmd) = cmd.strip_prefix('!') {
            let shell_cmd = shell_cmd.trim();
            if shell_cmd.is_empty() {
                self.message = Some("usage: :!<command>".into());
            } else {
                self.pending_shell_command = Some(shell_cmd.to_string());
                self.message = None;
            }
            return; // Keluar dari fungsi karena sudah ditangani
        }
        let mut parts = cmd.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("").trim();
        let arg = parts.next().map(|s| s.trim());

        match name {
            "w" => {
                if let Some(path) = arg {
                    self.path = Some(PathBuf::from(path));
                }
                match self.save() {
                    Ok(()) => {}
                    Err(e) => self.message = Some(format!("error: {}", e)),
                }
            }
            "wq" | "x" => {
                if let Some(path) = arg {
                    self.path = Some(PathBuf::from(path));
                }
                match self.save() {
                    Ok(()) => self.should_quit = true,
                    Err(e) => self.message = Some(format!("error: {}", e)),
                }
            }
            "q" => {
                if self.is_dirty() {
                    self.message = Some("unsaved changes (add ! to override)".into());
                } else {
                    self.should_quit = true;
                }
            }
            "q!" => {
                self.should_quit = true;
            }
            "e" => {
                if let Some(path) = arg {
                    match App::from_file(PathBuf::from(path), self.config.clone()) {
                        Ok(new_app) => {
                            self.editor = new_app.editor;
                            self.path = new_app.path;
                            self.line_ending = new_app.line_ending;
                            self.had_bom = new_app.had_bom;
                            self.saved_hash = new_app.saved_hash;
                            self.mode = Mode::Normal;
                            self.selection_anchor = None;
                            self.message = Some(format!("opened: {}", path));
                        }
                        Err(e) => self.message = Some(format!("error: {}", e)),
                    }
                } else {
                    self.message = Some("usage: :e <file>".into());
                }
            }
            "settheme" => {
                if let Some(theme_name) = arg {
                    if let Err(e) = self.set_theme(theme_name) {
                        self.message = Some(e);
                    }
                } else {
                    self.open_theme_picker();
                }
            }
            "pwd" => {
                match std::env::current_dir() {
                    Ok(path) => self.message = Some(format!("pwd: {}", path.display())),
                    Err(e) => self.message = Some(format!("pwd error: {}", e)),
                }
            }
            "cd" => {
                if let Some(arg_str) = arg {
                    let mut path_str = arg_str.to_string();
                    #[cfg(target_os = "windows")]
                    {
                        if path_str.len() == 2 && path_str.ends_with(':') {
                            path_str.push('\\');
                        }
                    }

                    let path = PathBuf::from(&path_str);

                    if !path.exists() {
                        self.message = Some(format!("cd error: path does not exist: {}", path.display()));
                    } else if !path.is_dir() {
                        self.message = Some(format!("cd error: not a directory: {}", path.display()));
                    } else if let Err(e) = std::env::set_current_dir(&path) {
                        self.message = Some(format!("cd error: {}", e));
                    } else {
                        self.message = Some(format!("cd: {}", path.display()));
                    }
                } else {
                    // Jika `:cd` tanpa argumen, kembali ke HOME
                    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
                    if let Some(h) = home {
                        if let Err(e) = std::env::set_current_dir(&h) {
                            self.message = Some(format!("cd error: {}", e));
                        } else {
                            self.message = Some(format!("cd: {}", PathBuf::from(h).display()));
                        }
                    } else {
                        self.message = Some("cd error: HOME not set".into());
                    }
                }
            }
            "!" => {
                if let Some(cmd) = arg {
                    self.pending_shell_command = Some(cmd.to_string());
                    self.message = None;
                } else {
                    self.message = Some("usage: :!<command>".into());
                }
            }
            _ => {
                self.message = Some(format!("unknown command: {}", name));
            }
        }
    }
}

fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_w_saves() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        let mut app = App::new_empty(Config::default());
        app.path = Some(path.clone());
        app.apply_editor(Action::InsertChar('x')).unwrap();
        app.execute_command("w");
        assert!(!app.is_dirty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    }

    #[test]
    fn command_q_refuses_when_dirty() {
        let mut app = App::new_empty(Config::default());
        app.apply_editor(Action::InsertChar('x')).unwrap();
        app.execute_command("q");
        assert!(!app.should_quit);
        assert!(app.message.is_some());
    }

    #[test]
    fn command_qbang_quits() {
        let mut app = App::new_empty(Config::default());
        app.apply_editor(Action::InsertChar('x')).unwrap();
        app.execute_command("q!");
        assert!(app.should_quit);
    }

    #[test]
    fn command_wq_saves_and_quits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        let mut app = App::new_empty(Config::default());
        app.path = Some(path.clone());
        app.apply_editor(Action::InsertChar('y')).unwrap();
        app.execute_command("wq");
        assert!(app.should_quit);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "y");
    }

    #[test]
    fn command_e_opens_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.txt");
        std::fs::write(&src, "loaded").unwrap();
        let mut app = App::new_empty(Config::default());
        app.execute_command(&format!("e {}", src.display()));
        assert_eq!(app.editor.buffer().as_string(), "loaded");
    }

    #[test]
    fn search_finds_matches_and_jumps() {
        let mut app = App::new_empty(Config::default());
        app.editor = Editor::from_str("hello world\nhello again");
        
        app.execute_search("hello");
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.matches.len(), 2);
        assert_eq!(search.current_idx, 0);
        assert_eq!(app.editor.cursor().char_idx(), 0);

        app.search_next();
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.current_idx, 1);
        assert_eq!(app.editor.cursor().char_idx(), 12);

        app.search_next();
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.current_idx, 0);
        assert_eq!(app.editor.cursor().char_idx(), 0);

        app.search_prev();
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.current_idx, 1);
        assert_eq!(app.editor.cursor().char_idx(), 12);
    }

    #[test]
    fn search_is_case_insensitive() {
        let mut app = App::new_empty(Config::default());
        app.editor = Editor::from_str("Hello World");
        
        app.execute_search("hello");
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.matches.len(), 1);
        assert_eq!(search.matches[0].start, 0);
        assert_eq!(search.matches[0].end, 5);
    }
}