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

    // text editing
    InsertChar(char),
    InsertNewline,
    DeleteBackward,
    DeleteForward,
}

impl Action {
    /// True if applying this action can change the buffer content.
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Action::InsertChar(_)
                | Action::InsertNewline
                | Action::DeleteBackward
                | Action::DeleteForward
        )
    }
}