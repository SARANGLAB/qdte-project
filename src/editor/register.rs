#[derive(Debug, Clone, Default)]
pub struct Register {
    pub content: String,
    pub linewise: bool,
}

impl Register {
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.linewise = false;
    }

    pub fn set_linewise(&mut self, content: String) {
        self.content = content;
        self.linewise = true;
    }
}