#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Single-char typing. Coalesces with adjacent same-kind inserts.
    Typing,
    /// Newline, tab, or other structural inserts. Never coalesces.
    Structural,
    /// Deletions. Never coalesces.
    Deletion,
}

#[derive(Debug, Clone)]
pub struct Change {
    pub char_idx: usize,
    pub removed: String,
    pub inserted: String,
    pub cursor_before: usize,
    pub cursor_after: usize,
    pub kind: ChangeKind,
}

impl Change {
    pub fn insert(
        char_idx: usize,
        text: impl Into<String>,
        cursor_before: usize,
        cursor_after: usize,
        kind: ChangeKind,
    ) -> Self {
        Self {
            char_idx,
            removed: String::new(),
            inserted: text.into(),
            cursor_before,
            cursor_after,
            kind,
        }
    }

    pub fn delete(
        char_idx: usize,
        removed: impl Into<String>,
        cursor_before: usize,
        cursor_after: usize,
    ) -> Self {
        Self {
            char_idx,
            removed: removed.into(),
            inserted: String::new(),
            cursor_before,
            cursor_after,
            kind: ChangeKind::Deletion,
        }
    }
}

#[derive(Debug, Default)]
pub struct History {
    undo_stack: Vec<Change>,
    redo_stack: Vec<Change>,
}

impl History {
    pub fn record(&mut self, change: Change) {
        // Coalesce only contiguous single-char typing.
        if change.kind == ChangeKind::Typing {
            if let Some(last) = self.undo_stack.last_mut() {
                if last.kind == ChangeKind::Typing
                    && last.removed.is_empty()
                    && change.removed.is_empty()
                    && last.char_idx + last.inserted.chars().count() == change.char_idx
                {
                    last.inserted.push_str(&change.inserted);
                    last.cursor_after = change.cursor_after;
                    self.redo_stack.clear();
                    return;
                }
            }
        }
        self.undo_stack.push(change);
        self.redo_stack.clear();
    }

    pub fn pop_undo(&mut self) -> Option<Change> {
        self.undo_stack.pop()
    }

    pub fn pop_redo(&mut self) -> Option<Change> {
        self.redo_stack.pop()
    }

    pub fn push_undo(&mut self, c: Change) {
        self.undo_stack.push(c);
    }

    pub fn push_redo(&mut self, c: Change) {
        self.redo_stack.push(c);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_pops() {
        let mut h = History::default();
        h.record(Change::delete(0, "x", 1, 0));
        assert!(h.can_undo());
        assert!(!h.can_redo());
        let c = h.pop_undo().unwrap();
        assert_eq!(c.removed, "x");
        h.push_redo(c);
        assert!(!h.can_undo());
        assert!(h.can_redo());
    }

    #[test]
    fn contiguous_typing_coalesces() {
        let mut h = History::default();
        h.record(Change::insert(0, "a", 0, 1, ChangeKind::Typing));
        h.record(Change::insert(1, "b", 1, 2, ChangeKind::Typing));
        h.record(Change::insert(2, "c", 2, 3, ChangeKind::Typing));
        let c = h.pop_undo().unwrap();
        assert_eq!(c.inserted, "abc");
        assert_eq!(c.char_idx, 0);
        assert_eq!(c.cursor_after, 3);
    }

    #[test]
    fn non_contiguous_typing_does_not_coalesce() {
        let mut h = History::default();
        h.record(Change::insert(0, "a", 0, 1, ChangeKind::Typing));
        h.record(Change::insert(5, "b", 5, 6, ChangeKind::Typing));
        assert_eq!(h.pop_undo().unwrap().inserted, "b");
        assert_eq!(h.pop_undo().unwrap().inserted, "a");
    }

    #[test]
    fn structural_insert_does_not_coalesce() {
        let mut h = History::default();
        h.record(Change::insert(0, "a", 0, 1, ChangeKind::Typing));
        h.record(Change::insert(1, "\n", 1, 2, ChangeKind::Structural));
        assert_eq!(h.pop_undo().unwrap().inserted, "\n");
        assert_eq!(h.pop_undo().unwrap().inserted, "a");
    }

    #[test]
    fn deletion_does_not_coalesce() {
        let mut h = History::default();
        h.record(Change::insert(0, "a", 0, 1, ChangeKind::Typing));
        h.record(Change::delete(0, "a", 1, 0));
        assert_eq!(h.pop_undo().unwrap().kind, ChangeKind::Deletion);
        assert_eq!(h.pop_undo().unwrap().kind, ChangeKind::Typing);
    }

    #[test]
    fn new_record_clears_redo() {
        let mut h = History::default();
        h.record(Change::insert(0, "a", 0, 1, ChangeKind::Typing));
        let c = h.pop_undo().unwrap();
        h.push_redo(c);
        assert!(h.can_redo());
        h.record(Change::insert(0, "b", 0, 1, ChangeKind::Typing));
        assert!(!h.can_redo());
    }
}