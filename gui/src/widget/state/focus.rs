use crate::tree::NodeId;

/// 框架级焦点状态。只跟踪哪个运行时节点拥有键盘焦点。
pub struct FocusState {
    focused: Option<NodeId>,
    focusable: Vec<NodeId>,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            focused: None,
            focusable: Vec::new(),
        }
    }

    pub fn set_focusable(&mut self, ids: Vec<NodeId>) {
        if let Some(focused) = self.focused {
            if !ids.contains(&focused) {
                self.focused = None;
            }
        }
        self.focusable = ids;
    }

    pub fn focus(&mut self, id: NodeId) {
        self.focused = Some(id);
    }

    pub fn blur(&mut self) {
        self.focused = None;
    }

    pub fn is_focused(&self, id: NodeId) -> bool {
        self.focused == Some(id)
    }

    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    pub fn tab_next(&mut self) {
        self.focused = self.advance(1);
    }

    pub fn tab_prev(&mut self) {
        self.focused = self.advance(-1);
    }

    fn advance(&self, delta: isize) -> Option<NodeId> {
        if self.focusable.is_empty() {
            return None;
        }
        let current = self
            .focused
            .and_then(|id| self.focusable.iter().position(|&focusable| focusable == id));
        let next = match current {
            Some(index) => {
                (index as isize + delta).rem_euclid(self.focusable.len() as isize) as usize
            }
            None => 0,
        };
        Some(self.focusable[next])
    }
}
