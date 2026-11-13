# Sarang QDTE

**Quick & Dirty Text Editor** — a terminal-based text editor built with Rust.

A [Sarang Community]([https://github.com/sarang-community](https://github.com/SARANGLAB/qdte-project)) project.

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

**v0.1.0 — early development.** Core editing is functional. Many features from
the roadmap are still in progress (see [Roadmap](#roadmap)). The codebase is
small and changes frequently.

## Features

Current as of v0.1.0:

- **Two keybinding profiles**
  - *Normal* — non-modal, traditional shortcuts (Ctrl+S, Ctrl+X, arrows).
  - *Vim* — modal editing (`h j k l`, `i a`, `dd`, `gg`, `Esc`).
  - Selected automatically by how the binary is invoked (`vim` → Vim keymap).
- **File I/O** with round-trip preservation of:
  - Line endings (LF or CRLF — detected per file, preserved on write).
  - UTF-8 BOM (stripped from buffer, restored on write).
- **Unicode-correct cursor**
  - Grapheme-cluster aware horizontal movement (CJK, emoji ZWJ sequences,
    regional-indicator flags all move as one unit).
  - Visual column tracking with Unicode width for correct rendering.
- **Full-screen renderer**
  - Vertical and horizontal scrolling.
  - Tab expansion (tab stop = 4).
  - Status bar with mode, filename, dirty flag, cursor position.
  - `~` for lines past EOF.
- **New files are not created until save.** Opening `qdte newfile.txt` creates
  an empty buffer in memory; the file appears on disk only after `Ctrl+S`.

## Building

Requires Rust 1.75 or later.

```sh
git clone https://github.com/sarang-community/sarang-qdte
cd sarang-qdte
cargo build --release
```

The binary is `target/release/sarang-qdte` (`sarang-qdte.exe` on Windows).

## Running

```sh
qdte                    # empty buffer
qdte file.txt           # open existing file, or start new buffer
qdte --help             # usage
qdte --version          # version
```

### Choosing a keybinding profile

The keymap is selected by how the binary is invoked. To use Vim keybindings,
make QDTE accessible under a `vim`, `nvim`, or `vi` name.

**Linux / macOS:**

```sh
ln -sf /path/to/sarang-qdte ~/.local/bin/vim
ln -sf /path/to/sarang-qdte ~/.local/bin/nano
```

**Windows (PowerShell):**

```powershell
Copy-Item sarang-qdte.exe vim.exe
Copy-Item sarang-qdte.exe nano.exe
```

QDTE inspects `argv[0]` — `vim`, `nvim`, and `vi` select Vim keybindings;
anything else (including `qdte`, `nano`) uses the non-modal keybindings.

## Keybindings

### Normal profile (default)

Non-modal editing. QDTE behaves like a conventional terminal editor.

| Key         | Action                    |
|-------------|---------------------------|
| `Ctrl+S`    | Save                      |
| `Ctrl+X`    | Quit (warns if unsaved)   |
| `Ctrl+Q`    | Quit (alias)              |
| `Ctrl+C`    | Quit (alias)              |
| `←` `→` `↑` `↓` | Move cursor           |
| `Home`      | Start of line             |
| `End`       | End of line               |
| `Enter`     | Insert newline            |
| `Tab`       | Insert tab                |
| `Backspace` | Delete character before   |
| `Delete`    | Delete character after    |

### Vim profile

Modal editing. Invoke via `vim`, `nvim`, or `vi` as described above.

**Normal mode:**

| Key       | Action                              |
|-----------|-------------------------------------|
| `h j k l` | Left, down, up, right               |
| `0`       | Start of line                       |
| `$`       | End of line                         |
| `gg`      | Start of buffer                     |
| `G`       | End of buffer                       |
| `i`       | Insert mode (at cursor)             |
| `a`       | Append (after cursor, insert mode)  |
| `I`       | Insert at start of line             |
| `A`       | Append at end of line               |
| `o`       | Open line below (insert mode)       |
| `O`       | Open line above (insert mode)       |
| `x`       | Delete character under cursor       |
| `dd`      | Delete current line                 |
| `Esc`     | Cancel pending operator             |

**Insert mode:**

| Key         | Action                    |
|-------------|---------------------------|
| Printable   | Insert character          |
| `Enter`     | Insert newline            |
| `Tab`       | Insert tab                |
| `Backspace` | Delete before cursor      |
| `Delete`    | Delete after cursor       |
| Arrows      | Move cursor               |
| `Home` `End`| Start / end of line       |
| `Esc`       | Return to normal mode     |
| `Ctrl+S`    | Save                      |
| `Ctrl+X`    | Quit (warns if unsaved)   |

The two profiles share a single editor engine — only key-to-action mapping
differs.

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
- [ ] Open command (`Ctrl+O`) and Save As
- [ ] Undo / redo
- [ ] Search
- [ ] Visual mode, yank, paste
- [ ] `:` command-line mode
- [ ] Word motions (`w`, `b`, `e`)
- [ ] Line number gutter
- [ ] Configuration file
- [ ] Distribution and packaging (Linux, Windows, macOS)

## Architecture

```
src/
├── main.rs              entry point, event loop
├── app.rs               application state (editor + file metadata + mode)
├── cli/                 argument parsing and argv[0] detection
├── editor/
│   ├── buffer.rs        Ropey-backed text buffer (char-indexed)
│   ├── cursor.rs        grapheme-aware cursor with movement primitives
│   ├── action.rs        abstract editor actions
│   ├── mode.rs          Normal / Insert mode
│   └── mod.rs           Editor: owns buffer + cursor, applies actions
├── io/
│   ├── reader.rs        file loading with line-ending/BOM detection
│   └── writer.rs        file saving with line-ending/BOM restoration
├── keymap/
│   ├── dispatcher.rs    selects keymap by invocation, drives resolve()
│   ├── normal.rs        non-modal keybinding profile
│   ├── vim.rs           modal keybinding profile with state machine
│   └── mod.rs           Keymap trait, Command enum
└── terminal/
    ├── input.rs         key event reading
    ├── renderer.rs      full-screen drawing
    └── mod.rs           RAII terminal guard (raw mode + alternate screen)
```

Separation of concerns:

- `Editor` (buffer + cursor) knows nothing about keybindings or filesystem.
- `App` owns the editor plus file path, line ending, BOM, dirty flag, mode.
- Keymaps resolve keys to `Command`s; commands are dispatched by the app.
- `io` is unaware of rendering; `renderer` is unaware of filesystem.
- Both keybinding profiles share one editor engine.

## Testing

```sh
cargo test
```

Unit tests live alongside source (`#[cfg(test)] mod tests`); integration tests
are in `tests/`. Current coverage includes: CLI parsing, I/O round-trips
(LF/CRLF, BOM), buffer operations, cursor movement (including emoji ZWJ and
regional indicators), editor action dispatch, keymap resolution, vim state
machine, and renderer line formatting.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
