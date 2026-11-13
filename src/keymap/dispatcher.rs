use crossterm::event::KeyEvent;

use crate::editor::Mode;

use super::{normal::NormalKeymap, vim::VimKeymap, Command, Keymap};

pub struct Dispatcher<K: Keymap> {
    keymap: K,
}

impl<K: Keymap> Dispatcher<K> {
    pub fn new(keymap: K) -> Self {
        Self { keymap }
    }

    pub fn resolve(&mut self, mode: Mode, key: KeyEvent) -> Option<Command> {
        self.keymap.resolve(mode, key)
    }

    pub fn initial_mode(&self) -> Mode {
        self.keymap.initial_mode()
    }

    pub fn shows_mode(&self) -> bool {
        self.keymap.shows_mode()
    }
}

impl Dispatcher<Box<dyn Keymap>> {
    pub fn for_vim_mode(vim: bool) -> Self {
        if vim {
            Self::new(Box::new(VimKeymap::new()))
        } else {
            Self::new(Box::new(NormalKeymap))
        }
    }
}