use std::collections::HashSet;
use types::NodeId;

#[derive(Debug, Clone)]
pub struct PanelState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    Preview,
    Toolbar,
}

#[derive(Debug, Clone)]
pub struct UIState {
    pub selection: HashSet<NodeId>,
    pub preview_image: Option<types::Image>,
    pub is_running: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            selection: HashSet::new(),
            preview_image: None,
            is_running: false,
        }
    }
}
