# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **CLI & Setup**
  - Argument parsing with `clap` (`qdte <FILE>`, `--help`, `--version`).
  - First-run setup wizard: prompts the user to choose a default keymap
    (Normal or Vi) on the first launch and saves it to the config file.
  - Built-in tutorials accessible via `qdte tutor` and `qdte vi-tutor`.
  - `argv[0]` detection: invoking via `vim`, `nvim`, or `vi` selects the Vim
    keybinding profile; any other name uses the non-modal profile.

- **Filesystem I/O**
  - Byte-exact file reading and writing.
  - Line ending detection (LF or CRLF) and round-trip preservation.
  - UTF-8 BOM detection and round-trip preservation.
  - New files are not created until the first save.
  - Gracefully rejects directories passed as file arguments.

- **Text buffer & Cursor**
  - Ropey-backed buffer with char-indexed API.
  - Grapheme-cluster aware horizontal movement (CJK, emoji ZWJ, flags).
  - Vertical movement preserves grapheme column.
  - Word motions (`w`, `b`, `e`, `W`, `B`, `E`) with vim-compatible
    classification (whitespace, word, punctuation).

- **Editing & History**
  - Full undo/redo history with coalescing (contiguous typing merges into
    a single undo step).
  - Dirty state derived from content hash comparison.
  - Yank/Paste (`yy`, `p`, `P`) with linewise and charwise support.

- **Search**
  - Incremental (live) search as the user types in the prompt.
  - Case-insensitive matching (e.g., `Status` matches `status`).
  - Visual highlighting of all matches, with the active match highlighted
    in a brighter color.
  - Normal mode: `Ctrl+F` to search, `F3` / `Shift+F3` for next/prev.
  - Vim mode: `/` to search, `n` / `N` for next/prev.
  - `Esc` cancels the search prompt and restores the cursor to its original
    position, or clears the active highlight in normal mode.

- **Terminal & Renderer**
  - RAII terminal guard (raw mode + alternate screen, panic-safe).
  - Line number gutter with dynamic width based on buffer size.
  - Vertical and horizontal scrolling.
  - Tab expansion (configurable).
  - Unicode visual width aware.
  - Status bar with mode, filename, dirty flag, char/line/col position.
  - Prompt UI for Open, Save As, Search, and `:` command mode.
  - Visual selection highlight (char-wise and line-wise).
  - Block cursor in Normal/Visual modes, bar cursor in Insert mode.
  - Tilde (`~`) indicator for lines past EOF.

- **Keybinding: Normal profile**
  - `Ctrl+S` save, `Ctrl+Shift+S` / `F2` Save As, `Ctrl+O` open.
  - `Ctrl+X` / `Ctrl+Q` / `Ctrl+C` quit (with unsaved changes warning).
  - `Ctrl+Z` undo, `Ctrl+Y` redo.
  - `Ctrl+F` search, `F3` / `Shift+F3` next/prev.
  - `Esc` clears search highlights.
  - Arrows, Home, End, Enter, Tab, Backspace, Delete.

- **Keybinding: Vim profile**
  - Modal editing: Normal, Insert, Visual, Visual Line.
  - Normal mode: `h j k l`, `0`, `$`, `gg`, `G`, `w b e`, `W B E`, `i a I A`,
    `o O`, `x`, `dd`, `yy`, `p P`, `u`, `Ctrl+R`, `v`, `V`, `/`, `n`, `N`, `:`.
  - Insert mode: standard text input, `Esc` to return to normal.
  - Visual mode: extend selection with motions, `y` yank, `d`/`x` delete,
    `o` swap anchor, `v`/`V` toggle modes.
  - Command-line mode (`:`): `:w [path]`, `:wq`, `:x`, `:q`, `:q!`, `:e <path>`.
  - Pending-operator state machine (`d`, `g`, `y`).

- **Configuration**
  - TOML config loaded from platform default path:
    - Windows: `%APPDATA%\qdte\config.toml`
    - macOS: `~/Library/Application Support/qdte/config.toml`
    - Linux: `$XDG_CONFIG_HOME/qdte/config.toml`
  - Fields: `tab_width`, `expand_tab`, `fixeol`, `keymap`.
  - Parse errors fall back to defaults with a stderr warning.

### Changed

- Package, crate, and binary renamed from `sarang-qdte` to `qdte`.
- `Keymap` trait now takes `&mut self` and a `Mode` parameter.
- `Buffer::char_to_line` now returns the last (possibly empty) line for
  past-end indices, fixing cursor position after Enter at EOF.

### Fixed

- `DeleteBackward` at the start of a line now correctly joins with the
  previous line.
- `Tab` key is now mapped in the normal profile and respects `expand_tab`.
- Char-wise paste (`p`/`P`) no longer injects unexpected newlines when the
  register is not linewise.
- Line-wise visual selection no longer highlights the entire remaining
  screen when only one line is selected.
- Tilde (`~`) rendering is now aligned correctly with the line number gutter.

### Known Limitations

- No syntax highlighting.
- Visual block mode (`Ctrl+V`) is not implemented.
- No distribution binaries or CI release pipeline yet.

## [0.1.0] — TBD

First tagged release. Pending: Syntax highlighting, Distribution builds.