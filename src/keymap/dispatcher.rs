use crossterm::event::KeyEvent;

use super::{Command, Keymap};

pub struct Dispatcher<K: Keymap> {
    keymap: K,
}

impl<K: Keymap> Dispatcher<K> {
    pub fn new(keymap: K) -> Self {
        Self { keymap }
    }

    pub fn resolve(&self, key: KeyEvent) -> Option<Command> {
        self.keymap.resolve(key)
    }
}