#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // cursor movement
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveLineStart,
    MoveLineEnd,
    MoveBufferStart,
    MoveBufferEnd,
    MoveLeftLineClamped,
    MoveRightLineClamped,

    // text editing
    InsertChar(char),
    InsertNewline,
    DeleteBackward,
    DeleteForward,
    DeleteLine,
    // undo / redo
    Undo,
    Redo,
}

impl Action {
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Action::InsertChar(_)
                | Action::InsertNewline
                | Action::DeleteBackward
                | Action::DeleteForward
                | Action::DeleteLine
                | Action::Undo
                | Action::Redo
        )
    }
}
