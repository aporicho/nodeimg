use crate::tree::NodeId;

/// Framework-level keyboard focus state.
pub(crate) struct FocusState {
    focused: Option<NodeId>,
    focusable: Vec<NodeId>,
}

impl FocusState {
    pub(crate) fn new() -> Self {
        Self {
            focused: None,
            focusable: Vec::new(),
        }
    }

    pub(crate) fn focus(&mut self, id: NodeId) {
        self.focused = Some(id);
    }

    pub(crate) fn blur(&mut self) {
        self.focused = None;
    }

    pub(crate) fn is_focused(&self, id: NodeId) -> bool {
        self.focused == Some(id)
    }

    pub(crate) fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    pub(crate) fn tab_next(&mut self) {
        self.focused = self.advance(1);
    }

    pub(crate) fn tab_prev(&mut self) {
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
