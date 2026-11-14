# Sarang QDTE

**Quick & Dirty Text Editor** — a terminal-based text editor built with Rust.

A [Sarang Community](https://github.com/SARANGLAB) project.

> QDTE takes its name from QDOS (Quick and Dirty Operating System). The spirit
> is the same: fast, minimal, native. Not that the implementation should be
> sloppy.

## Philosophy

- **Quick** — fast startup, responsive input.
- **Dirty** — simple, unassuming. No unnecessary complexity.
- **Text Editor** — focused on editing text.

Development principles:

- Lightweight, native executable
- Minimal dependencies
- No background processes, no telemetry
- Cross-platform (Linux, Windows, macOS)
- Maintainable, tested codebase

## Status

**v0.1.0 — early development.** Core editing is functional and stable. The
codebase is small and changes frequently. Not yet distributed publicly.

## Features

Current as of v0.1.0:

- **Two keybinding profiles**
  - *Normal* — non-modal, traditional shortcuts (`Ctrl+S`, `Ctrl+X`, arrows).
  - *Vim* — modal editing (Normal, Insert, Visual, and Command modes).
  - Selected automatically via first-run setup wizard, `argv[0]`, or config.
- **Interactive Setup & Tutorials**
  - First-run setup wizard prompts you to choose a default keymap.
  - Built-in tutorials: `qdte tutor` and `qdte vi-tutor`.
- **Search**
  - Incremental (live) and case-insensitive.
  - Regex-free exact substring matching (e.g., `Status` matches `status`).
  - Highlights all matches with a brighter color for the active match.
- **File I/O** with round-trip preservation of:
  - Line endings (LF or CRLF — detected per file, preserved on write).
  - UTF-8 BOM (stripped from buffer, restored on write).
- **Unicode-correct cursor & rendering**
  - Grapheme-cluster aware horizontal movement (CJK, emoji ZWJ).
  - Visual column tracking with Unicode width.
  - Block cursor in Normal/Visual modes, bar cursor in Insert mode.
- **Editing**
  - Full undo/redo history with coalescing.
  - Save, Save As, Open, and Quit with unsaved-changes protection.
- **Vim Advanced**
  - Word motions (`w`, `b`, `e`, `W`, `B`, `E`).
  - Yank/Paste (`yy`, `p`, `P`) with linewise and charwise support.
  - Visual mode (`v`, `V`) with movement, `y`, `d`/`x`, and `o` swap anchor.
  - Command-line mode (`:w`, `:wq`, `:q`, `:q!`, `:e`).
- **Full-screen renderer**
  - Line number gutter.
  - Vertical and horizontal scrolling.
  - Tab expansion, `~` for lines past EOF.
  - Status bar with mode, filename, dirty flag, cursor position, and messages.
- **Configuration** via TOML.

## Building

Requires Rust 1.75 or later.

```sh
git clone https://github.com/sarang-community/sarang-qdte
cd sarang-qdte
cargo build --release
```

The binary is `target/release/qdte` (`qdte.exe` on Windows).

### Installing to PATH

```sh
cargo install --path .
```

After installing, `qdte` will be available from any directory.

## Running

```sh
qdte                    # empty buffer
qdte file.txt           # open existing file, or start new buffer
qdte tutor              # view normal keymap tutorial
qdte vi-tutor           # view vim keymap tutorial
qdte --help             # usage
qdte --version          # version
```

### First Run

On the first launch, QDTE will prompt you to choose a default keymap:

```text
Welcome to QDTE!

Please choose your default keymap:
  1. Normal (for average users) [Default]
  2. Vi (for advanced users)
Choice (1/2):
```

This choice is saved to your platform-specific configuration file.

### Choosing a keybinding profile

The keymap can be selected in three ways (in order of precedence):
1. Overridden in the `config.toml` file.
2. Invoked via `vim`, `nvim`, or `vi` (selects Vim keybindings).
3. The default saved from the first-run setup wizard.

**Linux / macOS:**

```sh
ln -sf /path/to/qdte ~/.local/bin/vim
ln -sf /path/to/qdte ~/.local/bin/nano
```

**Windows (PowerShell):**

```powershell
Copy-Item "$env:USERPROFILE\.cargo\bin\qdte.exe" "$env:USERPROFILE\.cargo\bin\vim.exe"
Copy-Item "$env:USERPROFILE\.cargo\bin\qdte.exe" "$env:USERPROFILE\.cargo\bin\nano.exe"
```

## Keybindings

### Normal profile (default)

Non-modal editing.

| Key         | Action                    |
|-------------|---------------------------|
| `Ctrl+S`    | Save                      |
| `Ctrl+Shift+S` / `F2` | Save As         |
| `Ctrl+O`    | Open file                 |
| `Ctrl+X` / `Ctrl+Q` / `Ctrl+C` | Quit (warns if unsaved) |
| `Ctrl+Z`    | Undo                      |
| `Ctrl+Y`    | Redo                      |
| `Ctrl+F`    | Search                    |
| `F3` / `Shift+F3` | Next / Prev search match |
| `Esc`       | Clear search highlight    |
| `←` `→` `↑` `↓` | Move cursor           |
| `Home` / `End` | Start / end of line    |
| `Enter`     | Insert newline            |
| `Tab`       | Insert tab                |
| `Backspace` | Delete character before   |
| `Delete`    | Delete character after    |

### Vim profile

Modal editing. Invoke via `vim`, `nvim`, or `vi`.

**Normal mode:**

| Key       | Action                              |
|-----------|-------------------------------------|
| `h j k l` | Left, down, up, right               |
| `0` / `$` | Start / end of line                 |
| `gg` / `G`| Start / end of buffer               |
| `w b e`   | Next word, previous word, end of word |
| `W B E`   | Same, but WORDs (ignore punctuation) |
| `i a I A` | Insert / append (variants)          |
| `o O`     | Open line below / above             |
| `x`       | Delete character under cursor       |
| `dd`      | Delete current line                 |
| `yy`      | Yank current line                   |
| `p P`     | Paste below / above                 |
| `v V`     | Enter Visual / Visual Line mode     |
| `u` / `Ctrl+R` | Undo / Redo                    |
| `/`       | Search                              |
| `n` / `N` | Next / Prev search match            |
| `Esc`     | Clear search highlight / cancel     |
| `:`       | Enter command-line mode             |

**Insert mode:**

| Key         | Action                    |
|-------------|---------------------------|
| Printable   | Insert character          |
| `Enter`     | Insert newline            |
| `Tab`       | Insert tab                |
| `Backspace` / `Delete` | Delete before / after |
| Arrows      | Move cursor               |
| `Esc`       | Return to normal mode     |
| `Ctrl+S` / `Ctrl+O` | Save / Open       |
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo       |

**Visual mode (`v`) / Visual Line mode (`V`):**

| Key       | Action                              |
|-----------|-------------------------------------|
| `h j k l` | Extend selection                    |
| `w b e`   | Extend by word                      |
| `d x`     | Delete selection                    |
| `y`       | Yank selection                      |
| `o`       | Swap anchor and cursor              |
| `v` / `V` | Toggle visual / visual-line mode    |
| `Esc`     | Exit to normal mode                 |

**Command-line mode (`:`):**

| Command     | Action                              |
|-------------|-------------------------------------|
| `:w [path]` | Save (optionally to path)           |
| `:wq` / `:x`| Save and quit                       |
| `:q`        | Quit (refused if unsaved changes)   |
| `:q!`       | Force quit                          |
| `:e <path>` | Open file                           |

## Configuration

QDTE looks for a TOML configuration file at:

- **Windows:** `%APPDATA%\qdte\config.toml`
- **macOS:** `~/Library/Application Support/qdte/config.toml`
- **Linux:** `$XDG_CONFIG_HOME/qdte/config.toml` (or `~/.config/qdte/config.toml`)

If the file is missing, the first-run setup wizard will create it. Parse errors
print a warning to stderr and fall back to defaults.

Example `config.toml`:

```toml
# Visual tab stop for rendering and expand_tab.
tab_width = 4

# Tab inserts spaces instead of a literal '\t'.
expand_tab = false

# Append trailing newline on save if missing.
fixeol = false

# Force keymap. Omit to infer from setup wizard or argv[0].
# keymap = "normal" # or "vim"
```

## Roadmap

Ordered roughly by priority:

- [x] CLI argument parsing
- [x] Filesystem I/O with line-ending and BOM preservation
- [x] Ropey-backed text buffer
- [x] Grapheme-aware cursor with Unicode width
- [x] Editor action abstraction
- [x] Terminal raw mode and input loop
- [x] Normal keybinding profile
- [x] Vim keybinding profile (subset)
- [x] Full-screen renderer with status bar
- [x] Undo / redo
- [x] Open, Save As, and prompt UI
- [x] Configuration file & setup wizard
- [x] Visual mode, yank, paste
- [x] `:` command-line mode
- [x] Word motions (`w`, `b`, `e`)
- [x] Block cursor in Normal/Visual modes
- [x] Search (`/` or `Ctrl+F`)
- [x] Line number gutter
- [x] Built-in tutorials (`tutor`, `vi-tutor`)
- [ ] Syntax highlighting
- [ ] Distribution and packaging (Linux, Windows, macOS)
- [ ] CI release pipeline

## Architecture

```
src/
├── main.rs              entry point, event loop, command dispatch, setup
├── app.rs               application state, prompt UI, command parsing
├── config.rs            TOML configuration loading and saving
├── cli/                 argument parsing and argv[0] detection
├── editor/
│   ├── buffer.rs        Ropey-backed text buffer (char-indexed)
│   ├── cursor.rs        grapheme-aware cursor, word motions
│   ├── action.rs        abstract editor actions
│   ├── history.rs       undo/redo stack with coalescing
│   ├── register.rs      yank register
│   ├── mode.rs          Normal / Insert / Visual / VisualLine
│   └── mod.rs           Editor: owns buffer, cursor, history, register
├── io/
│   ├── reader.rs        file loading with line-ending/BOM detection
│   └── writer.rs        file saving with line-ending/BOM restoration
├── keymap/
│   ├── dispatcher.rs    selects keymap, drives resolve()
│   ├── normal.rs        non-modal keybinding profile
│   ├── vim.rs           modal keybinding profile with state machines
│   └── mod.rs           Keymap trait, Command enum
└── terminal/
    ├── input.rs         key event reading
    ├── renderer.rs      full-screen drawing, gutter, search highlighting
    └── mod.rs           RAII terminal guard
```

Separation of concerns:

- `Editor` (buffer, cursor, history, register) knows nothing about keybindings,
  filesystem, terminal, or modes.
- `App` owns the editor plus file path, line ending, BOM, mode, selection
  anchor, prompt, search state, and dirty state.
- Keymaps resolve keys to `Command`s; commands are dispatched by `App`.
- `io` is unaware of rendering; `renderer` is unaware of filesystem.
- Both keybinding profiles share one editor engine.

## Testing

```sh
cargo test
```

Integration tests live in `tests/`; unit tests are alongside source.
Coverage includes CLI parsing, I/O round-trips, buffer operations, cursor
movement, editor action dispatch, keymap resolution, undo/redo, visual mode,
search, and renderer line formatting.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
