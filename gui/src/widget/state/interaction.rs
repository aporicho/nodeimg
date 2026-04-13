use crate::shell::AppEvent;
use crate::tree::{NodeId, Tree};

use super::focus::FocusState;
use super::reducer;
use super::registry;
use super::visual::WidgetVisualState;

/// 框架级交互状态存储。统一管理 hover / press / capture / focus。
pub struct InteractionStore {
    hovered: Option<NodeId>,
    pressed: Option<NodeId>,
    captured: Option<NodeId>,
    focus: FocusState,
}

impl InteractionStore {
    pub fn new() -> Self {
        Self {
            hovered: None,
            pressed: None,
            captured: None,
            focus: FocusState::new(),
        }
    }

    pub fn sync_with_tree(&mut self, tree: &Tree) {
        self.hovered = self.hovered.filter(|&id| tree.get(id).is_some());
        self.pressed = self.pressed.filter(|&id| tree.get(id).is_some());
        self.captured = self.captured.filter(|&id| tree.get(id).is_some());
        self.focus.set_focusable(registry::focusable_nodes(tree));
        if let Some(id) = self.focus.focused() {
            if tree.get(id).is_none() {
                self.focus.blur();
            }
        }
    }

    pub fn handle_event(&mut self, tree: &Tree, event: &AppEvent) {
        reducer::apply_event(self, tree, event);
    }

    pub fn visual_state(&self, node_id: NodeId, disabled: bool) -> WidgetVisualState {
        if disabled {
            WidgetVisualState::Disabled
        } else if self.pressed == Some(node_id) || self.captured == Some(node_id) {
            WidgetVisualState::Pressed
        } else if self.focus.is_focused(node_id) {
            WidgetVisualState::Focused
        } else if self.hovered == Some(node_id) {
            WidgetVisualState::Hovered
        } else {
            WidgetVisualState::Normal
        }
    }

    pub fn hovered(&self) -> Option<NodeId> {
        self.hovered
    }

    pub fn focused(&self) -> Option<NodeId> {
        self.focus.focused()
    }

    pub fn captured(&self) -> Option<NodeId> {
        self.captured
    }

    pub fn set_hovered(&mut self, hovered: Option<NodeId>) {
        self.hovered = hovered;
    }

    pub fn set_pressed(&mut self, pressed: Option<NodeId>) {
        self.pressed = pressed;
    }

    pub fn set_captured(&mut self, captured: Option<NodeId>) {
        self.captured = captured;
    }

    pub fn clear_pointer_state(&mut self) {
        self.hovered = None;
        self.pressed = None;
        self.captured = None;
    }

    pub fn focus(&mut self, node_id: NodeId) {
        self.focus.focus(node_id);
    }

    pub fn blur(&mut self) {
        self.focus.blur();
    }

    pub fn tab_next(&mut self) {
        self.focus.tab_next();
    }

    pub fn tab_prev(&mut self) {
        self.focus.tab_prev();
    }
}

impl Default for InteractionStore {
    fn default() -> Self {
        Self::new()
    }
}
