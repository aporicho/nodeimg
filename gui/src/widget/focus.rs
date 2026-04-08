/// 控件焦点管理。追踪当前焦点、Tab 循环。
pub struct FocusState {
    focused: Option<&'static str>,
    focusable: Vec<&'static str>,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            focused: None,
            focusable: Vec::new(),
        }
    }

    /// 每帧更新可聚焦控件列表（按布局顺序）。
    pub fn set_focusable(&mut self, ids: Vec<&'static str>) {
        // 如果当前焦点不在新列表中，清除焦点
        if let Some(f) = self.focused {
            if !ids.contains(&f) {
                self.focused = None;
            }
        }
        self.focusable = ids;
    }

    pub fn focus(&mut self, id: &'static str) {
        self.focused = Some(id);
    }

    pub fn blur(&mut self) {
        self.focused = None;
    }

    pub fn is_focused(&self, id: &str) -> bool {
        self.focused == Some(id)
    }

    pub fn focused(&self) -> Option<&'static str> {
        self.focused
    }

    /// Tab 下一个
    pub fn tab_next(&mut self) {
        self.focused = self.advance(1);
    }

    /// Shift+Tab 上一个
    pub fn tab_prev(&mut self) {
        self.focused = self.advance(-1);
    }

    fn advance(&self, delta: isize) -> Option<&'static str> {
        if self.focusable.is_empty() {
            return None;
        }
        let current = self
            .focused
            .and_then(|f| self.focusable.iter().position(|&id| id == f));
        let next = match current {
            Some(i) => (i as isize + delta).rem_euclid(self.focusable.len() as isize) as usize,
            None => 0,
        };
        Some(self.focusable[next])
    }
}
