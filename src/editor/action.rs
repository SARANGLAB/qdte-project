#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // cursor movement
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveLineStart,
    MoveLineEnd,
    MoveLineEndBlock,
    MoveBufferStart,
    MoveBufferEnd,
    MoveLeftLineClamped,
    MoveRightLineClamped,

    // text editing
    InsertChar(char),
    InsertNewline,
    InsertTab,
    DeleteBackward,
    DeleteForward,
    DeleteLine,
    DeleteRange { start: usize, end: usize },

    // undo / redo
    Undo,
    Redo,

    // word motions
    WordForward,
    WordBackward,
    WordEnd,
    WordForwardBig,
    WordBackwardBig,
    WordEndBig,

    // yank / paste
    YankLine,
    YankRange { start: usize, end: usize, linewise: bool },
    PasteAfter,
    PasteBefore,

    // Toggle Case
    ToggleCase,
    ToggleCaseRange { start: usize, end: usize },

    // Indent
    IndentLine,
    UnindentLine,
    IndentRange { start_line: usize, end_line: usize },
    UnindentRange { start_line: usize, end_line: usize },
    
    // find / till motions
    FindCharForward(char),
    FindCharBackward(char),
    TillCharForward(char),
    TillCharBackward(char),
}

impl Action {
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Action::InsertChar(_)
                | Action::InsertNewline
                | Action::InsertTab
                | Action::DeleteBackward
                | Action::DeleteForward
                | Action::DeleteLine
                | Action::DeleteRange { .. }
                | Action::PasteAfter
                | Action::PasteBefore
                | Action::Undo
                | Action::Redo
                | Action::ToggleCase           
                | Action::ToggleCaseRange { .. } 
                | Action::IndentLine
                | Action::UnindentLine
                | Action::IndentRange { .. }
                | Action::UnindentRange { .. }
        )
    }
}