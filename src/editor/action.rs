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