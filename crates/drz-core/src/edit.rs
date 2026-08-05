#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub inserted: String,
}

impl TextEdit {
    pub fn new_end_byte(&self) -> usize {
        self.start_byte + self.inserted.len()
    }
}
