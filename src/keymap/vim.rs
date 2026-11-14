use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::editor::{Action, Mode};

use super::{Command, Keymap};

#[derive(Debug, Default)]
enum Pending {
    #[default]
    None,
    D,
    G,
    Y,
}

pub struct VimKeymap {
    pending: Pending,
}

impl VimKeymap {
    pub fn new() -> Self {
        Self { pending: Pending::None }
    }
}

impl Default for VimKeymap {
    fn default() -> Self {
        Self::new()
    }
}

impl Keymap for VimKeymap {
    fn resolve(&mut self, mode: Mode, key: KeyEvent) -> Option<Command> {
        match mode {
            Mode::Normal => self.resolve_normal(key),
            Mode::Insert => self.resolve_insert(key),
        }
    }

    fn shows_mode(&self) -> bool {
        true
    }

    fn block_cursor(&self, mode: Mode) -> bool {
        matches!(mode, Mode::Normal)
    }
}

impl VimKeymap {
    fn resolve_normal(&mut self, key: KeyEvent) -> Option<Command> {
        use KeyCode::*;
        use KeyModifiers as M;

        // Global shortcuts
        match (key.code, key.modifiers) {
            (Char('S'), m) if m.contains(M::CONTROL) => return Some(Command::SaveAs),
            (Char('s'), m) if m.contains(M::CONTROL) && m.contains(M::SHIFT) => {
                return Some(Command::SaveAs);
            }
            (F(2), M::NONE) => return Some(Command::SaveAs),
            (Char('s'), M::CONTROL) => return Some(Command::Save),
            (Char('o'), M::CONTROL) => return Some(Command::Open),
            (Char('x'), M::CONTROL) | (Char('q'), M::CONTROL) | (Char('c'), M::CONTROL) => {
                return Some(Command::Quit);
            }
            (Char('r'), M::CONTROL) => return Some(Command::Editor(Action::Redo)),
            (Esc, _) => {
                self.pending = Pending::None;
                return None;
            }
            _ => {}
        }

        // Handle pending operator
        match std::mem::take(&mut self.pending) {
        Pending::D => {
            if key.code == Char('d') && key.modifiers == M::NONE {
                return Some(Command::Editor(Action::DeleteLine));
            }
            return None;
        }
        Pending::G => {
            if key.code == Char('g') && key.modifiers == M::NONE {
                return Some(Command::Editor(Action::MoveBufferStart));
            }
            return None;
        }
        Pending::Y => {
            if key.code == Char('y') && key.modifiers == M::NONE {
                return Some(Command::Editor(Action::YankLine));
            }
            return None;
        }
        Pending::None => {}
    }

        Some(match (key.code, key.modifiers) {
            (Char('u'), M::NONE) => Command::Editor(Action::Undo),
            (Char('h'), M::NONE) => Command::Editor(Action::MoveLeftLineClamped),
            (Char('j'), M::NONE) => Command::Editor(Action::MoveDown),
            (Char('k'), M::NONE) => Command::Editor(Action::MoveUp),
            (Char('l'), M::NONE) => Command::Editor(Action::MoveRightLineClamped),
            (Char('0'), M::NONE) => Command::Editor(Action::MoveLineStart),
            (Char('$'), M::SHIFT) => Command::Editor(Action::MoveLineEndBlock),
            (Char('G'), M::SHIFT) => Command::Editor(Action::MoveBufferEnd),
            (Char('g'), M::NONE) => {
                self.pending = Pending::G;
                return None;
            }
            (Char('i'), M::NONE) => Command::SetMode(Mode::Insert),
            (Char('a'), M::NONE) => Command::Seq(vec![
                Command::Editor(Action::MoveRightLineClamped),
                Command::SetMode(Mode::Insert),
            ]),
            (Char('I'), M::SHIFT) => Command::Seq(vec![
                Command::Editor(Action::MoveLineStart),
                Command::SetMode(Mode::Insert),
            ]),
            (Char('A'), M::SHIFT) => Command::Seq(vec![
                Command::Editor(Action::MoveLineEnd),
                Command::SetMode(Mode::Insert),
            ]),
            (Char('o'), M::NONE) => Command::Seq(vec![
                Command::Editor(Action::MoveLineEnd),
                Command::Editor(Action::InsertNewline),
                Command::SetMode(Mode::Insert),
            ]),
            (Char('O'), M::SHIFT) => Command::Seq(vec![
                Command::Editor(Action::MoveLineStart),
                Command::Editor(Action::InsertNewline),
                Command::Editor(Action::MoveUp),
                Command::SetMode(Mode::Insert),
            ]),
            (Char('x'), M::NONE) => Command::Editor(Action::DeleteForward),
            (Char('d'), M::NONE) => {
                self.pending = Pending::D;
                return None;
            }
            (Char('w'), M::NONE) => Command::Editor(Action::WordForward),
            (Char('b'), M::NONE) => Command::Editor(Action::WordBackward),
            (Char('e'), M::NONE) => Command::Editor(Action::WordEnd),
            (Char('W'), M::SHIFT) => Command::Editor(Action::WordForwardBig),
            (Char('B'), M::SHIFT) => Command::Editor(Action::WordBackwardBig),
            (Char('E'), M::SHIFT) => Command::Editor(Action::WordEndBig),
            (Char('y'), M::NONE) => {
                self.pending = Pending::Y;
                return None;
            }
            (Char('p'), M::NONE) => Command::Editor(Action::PasteAfter),
            (Char('P'), M::SHIFT) => Command::Editor(Action::PasteBefore),
            _ => return None,
        })
    }

    fn resolve_insert(&mut self, key: KeyEvent) -> Option<Command> {
        use KeyCode::*;
        use KeyModifiers as M;

        Some(match (key.code, key.modifiers) {
            (Esc, _) => Command::SetMode(Mode::Normal),
            (Char('s'), M::CONTROL) => Command::Save,
            (Char('x'), M::CONTROL) | (Char('q'), M::CONTROL) | (Char('c'), M::CONTROL) => {
                Command::Quit
            }
            (Char(c), M::NONE) | (Char(c), M::SHIFT) => Command::Editor(Action::InsertChar(c)),
            (Char('z'), M::CONTROL) => Command::Editor(Action::Undo),
            (Char('y'), M::CONTROL) => Command::Editor(Action::Redo),
            (Enter, _) => Command::Editor(Action::InsertNewline),
            (Tab, _) => Command::Editor(Action::InsertTab),
            (Backspace, _) => Command::Editor(Action::DeleteBackward),
            (Delete, _) => Command::Editor(Action::DeleteForward),
            (Left, _) => Command::Editor(Action::MoveLeft),
            (Right, _) => Command::Editor(Action::MoveRight),
            (Up, _) => Command::Editor(Action::MoveUp),
            (Down, _) => Command::Editor(Action::MoveDown),
            (Home, _) => Command::Editor(Action::MoveLineStart),
            (End, _) => Command::Editor(Action::MoveLineEnd),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    fn normal(km: &mut VimKeymap, c: char) -> Option<Command> {
        km.resolve(Mode::Normal, key(KeyCode::Char(c), KeyModifiers::NONE))
    }

    fn normal_shift(km: &mut VimKeymap, c: char) -> Option<Command> {
        km.resolve(Mode::Normal, key(KeyCode::Char(c), KeyModifiers::SHIFT))
    }

    #[test]
    fn h_j_k_l_map_to_movement() {
        let mut km = VimKeymap::new();
        assert!(matches!(normal(&mut km, 'h'), Some(Command::Editor(Action::MoveLeftLineClamped))));
        assert!(matches!(normal(&mut km, 'j'), Some(Command::Editor(Action::MoveDown))));
        assert!(matches!(normal(&mut km, 'k'), Some(Command::Editor(Action::MoveUp))));
        assert!(matches!(normal(&mut km, 'l'), Some(Command::Editor(Action::MoveRightLineClamped))));
    }

    #[test]
    fn i_enters_insert() {
        let mut km = VimKeymap::new();
        assert!(matches!(normal(&mut km, 'i'), Some(Command::SetMode(Mode::Insert))));
    }

    #[test]
    fn a_moves_then_inserts() {
        let mut km = VimKeymap::new();
        match normal(&mut km, 'a') {
            Some(Command::Seq(v)) if v.len() == 2 => {
                assert!(matches!(v[0], Command::Editor(Action::MoveRightLineClamped)));
                assert!(matches!(v[1], Command::SetMode(Mode::Insert)));
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn cap_a_goes_to_line_end() {
        let mut km = VimKeymap::new();
        match normal_shift(&mut km, 'A') {
            Some(Command::Seq(v)) if v.len() == 2 => {
                assert!(matches!(v[0], Command::Editor(Action::MoveLineEnd)));
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn dd_deletes_line() {
        let mut km = VimKeymap::new();
        assert!(normal(&mut km, 'd').is_none());
        assert!(matches!(
            normal(&mut km, 'd'),
            Some(Command::Editor(Action::DeleteLine))
        ));
    }

    #[test]
    fn d_other_cancels_without_processing() {
        let mut km = VimKeymap::new();
        assert!(normal(&mut km, 'd').is_none());
        assert!(normal(&mut km, 'x').is_none());
        // Now pending cleared — 'x' alone works
        assert!(matches!(
            normal(&mut km, 'x'),
            Some(Command::Editor(Action::DeleteForward))
        ));
    }

    #[test]
    fn gg_goes_to_buffer_start() {
        let mut km = VimKeymap::new();
        assert!(normal(&mut km, 'g').is_none());
        assert!(matches!(
            normal(&mut km, 'g'),
            Some(Command::Editor(Action::MoveBufferStart))
        ));
    }

    #[test]
    fn shift_g_goes_to_buffer_end() {
        let mut km = VimKeymap::new();
        assert!(matches!(
            normal_shift(&mut km, 'G'),
            Some(Command::Editor(Action::MoveBufferEnd))
        ));
    }

    #[test]
    fn insert_mode_typing_inserts() {
        let mut km = VimKeymap::new();
        match km.resolve(Mode::Insert, key(KeyCode::Char('a'), KeyModifiers::NONE)) {
            Some(Command::Editor(Action::InsertChar('a'))) => {}
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn insert_mode_esc_returns_to_normal() {
        let mut km = VimKeymap::new();
        assert!(matches!(
            km.resolve(Mode::Insert, key(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Command::SetMode(Mode::Normal))
        ));
    }

    #[test]
    fn normal_mode_typing_unknown_does_nothing() {
        let mut km = VimKeymap::new();
        assert!(normal(&mut km, 'q').is_none());
        assert!(normal(&mut km, 'z').is_none());
    }

    #[test]
    fn ctrl_s_saves_in_both_modes() {
        let mut km = VimKeymap::new();
        let ctrl_s = key(KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(matches!(km.resolve(Mode::Normal, ctrl_s), Some(Command::Save)));
        assert!(matches!(km.resolve(Mode::Insert, ctrl_s), Some(Command::Save)));
    }
    #[test]
    fn yy_yanks_line() {
        let mut km = VimKeymap::new();
        assert!(normal(&mut km, 'y').is_none());
        assert!(matches!(
            normal(&mut km, 'y'),
            Some(Command::Editor(Action::YankLine))
        ));
    }

    #[test]
    fn y_other_cancels() {
        let mut km = VimKeymap::new();
        assert!(normal(&mut km, 'y').is_none());
        assert!(normal(&mut km, 'w').is_none());
        // Subsequent 'w' works alone
        assert!(matches!(
            normal(&mut km, 'w'),
            Some(Command::Editor(Action::WordForward))
        ));
    }

    #[test]
    fn word_motions_map() {
        let mut km = VimKeymap::new();
        assert!(matches!(normal(&mut km, 'w'), Some(Command::Editor(Action::WordForward))));
        assert!(matches!(normal(&mut km, 'b'), Some(Command::Editor(Action::WordBackward))));
        assert!(matches!(normal(&mut km, 'e'), Some(Command::Editor(Action::WordEnd))));
    }

    #[test]
    fn paste_maps() {
        let mut km = VimKeymap::new();
        assert!(matches!(normal(&mut km, 'p'), Some(Command::Editor(Action::PasteAfter))));
        assert!(matches!(
            normal_shift(&mut km, 'P'),
            Some(Command::Editor(Action::PasteBefore))
        ));
    }
    #[test]
    fn dollar_moves_to_last_char() {
        let mut km = VimKeymap::new();
        assert!(matches!(
            normal_shift(&mut km, '$'),
            Some(Command::Editor(Action::MoveLineEndBlock))
        ));
    }
}
