/// 文字编辑状态机。TextInput 和 TextArea 共用。
pub struct TextEditState {
    text: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

impl TextEditState {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            cursor: text.len(),
            selection_anchor: None,
        }
    }

    // ── 读取 ──

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn has_selection(&self) -> bool {
        self.selection_anchor.is_some() && self.selection_anchor != Some(self.cursor)
    }

    /// 返回选择范围 (start, end)，start <= end。
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        if anchor == self.cursor {
            return None;
        }
        Some((anchor.min(self.cursor), anchor.max(self.cursor)))
    }

    pub fn selected_text(&self) -> &str {
        match self.selection_range() {
            Some((start, end)) => &self.text[start..end],
            None => "",
        }
    }

    // ── 编辑 ──

    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection_inner();
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.selection_anchor = None;
    }

    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection_inner();
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.selection_anchor = None;
    }

    pub fn backspace(&mut self) {
        if self.has_selection() {
            self.delete_selection_inner();
            return;
        }
        if self.cursor > 0 {
            let prev = prev_char_boundary(&self.text, self.cursor);
            self.text.drain(prev..self.cursor);
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.has_selection() {
            self.delete_selection_inner();
            return;
        }
        if self.cursor < self.text.len() {
            let next = next_char_boundary(&self.text, self.cursor);
            self.text.drain(self.cursor..next);
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.cursor.min(self.text.len());
        self.selection_anchor = None;
    }

    // ── 光标移动 ──

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = prev_char_boundary(&self.text, self.cursor);
        }
        self.selection_anchor = None;
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor = next_char_boundary(&self.text, self.cursor);
        }
        self.selection_anchor = None;
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
        self.selection_anchor = None;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.text.len();
        self.selection_anchor = None;
    }

    pub fn move_to(&mut self, pos: usize) {
        self.cursor = pos.min(self.text.len());
        self.selection_anchor = None;
    }

    // ── 选择 ──

    pub fn select_left(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        if self.cursor > 0 {
            self.cursor = prev_char_boundary(&self.text, self.cursor);
        }
    }

    pub fn select_right(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        if self.cursor < self.text.len() {
            self.cursor = next_char_boundary(&self.text, self.cursor);
        }
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.text.len();
    }

    pub fn select_to(&mut self, pos: usize) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = pos.min(self.text.len());
    }

    // ── 剪贴板 ──

    pub fn copy(&self) -> Option<String> {
        if self.has_selection() {
            Some(self.selected_text().to_string())
        } else {
            None
        }
    }

    pub fn cut(&mut self) -> Option<String> {
        if self.has_selection() {
            let text = self.selected_text().to_string();
            self.delete_selection_inner();
            Some(text)
        } else {
            None
        }
    }

    pub fn paste(&mut self, text: &str) {
        self.insert_str(text);
    }

    // ── 内部 ──

    fn delete_selection_inner(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            self.text.drain(start..end);
            self.cursor = start;
            self.selection_anchor = None;
        }
    }
}

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    let mut i = pos.saturating_sub(1);
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn next_char_boundary(s: &str, pos: usize) -> usize {
    let mut i = pos + 1;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}
