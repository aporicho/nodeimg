use crate::animation::AnimationStore;
use crate::shell::AppEvent;
use crate::tree::{NodeId, Tree};

use super::focus::FocusState;
use super::reducer;
use super::ControlVisualState;

/// Framework-level interaction state for hover, press, pointer capture, and focus.
pub struct InteractionState {
    hovered: Option<NodeId>,
    pressed: Option<NodeId>,
    captured: Option<NodeId>,
    focus: FocusState,
}

impl InteractionState {
    pub fn new() -> Self {
        Self {
            hovered: None,
            pressed: None,
            captured: None,
            focus: FocusState::new(),
        }
    }

    pub(crate) fn handle_event(
        &mut self,
        tree: &Tree,
        animations: Option<&AnimationStore>,
        event: &AppEvent,
    ) {
        reducer::apply_event(self, tree, animations, event);
    }

    pub fn visual_state(&self, node_id: NodeId, disabled: bool) -> ControlVisualState {
        if disabled {
            ControlVisualState::Disabled
        } else if self.pressed == Some(node_id) || self.captured == Some(node_id) {
            ControlVisualState::Pressed
        } else if self.focus.is_focused(node_id) {
            ControlVisualState::Focused
        } else if self.hovered == Some(node_id) {
            ControlVisualState::Hovered
        } else {
            ControlVisualState::Normal
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

    pub(crate) fn set_hovered(&mut self, hovered: Option<NodeId>) {
        self.hovered = hovered;
    }

    pub(crate) fn set_pressed(&mut self, pressed: Option<NodeId>) {
        self.pressed = pressed;
    }

    pub(crate) fn set_captured(&mut self, captured: Option<NodeId>) {
        self.captured = captured;
    }

    pub(crate) fn clear_pointer_state(&mut self) {
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

    pub(crate) fn tab_next(&mut self) {
        self.focus.tab_next();
    }

    pub(crate) fn tab_prev(&mut self) {
        self.focus.tab_prev();
    }
}

impl Default for InteractionState {
    fn default() -> Self {
        Self::new()
    }
}
