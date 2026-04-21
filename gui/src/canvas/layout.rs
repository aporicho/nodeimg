use crate::renderer::Rect;

#[derive(Clone, Debug)]
pub struct CanvasNodeIdentity {
    pub owner_id: String,
    pub default_rect: Rect,
}

#[derive(Clone, Debug)]
pub struct CanvasNodeLayout {
    pub owner_id: String,
    pub rect: Rect,
    pub z_index: i32,
    pub collapsed: bool,
}

pub fn canvas_node_stable_id(owner_id: &str) -> String {
    format!("canvas_node::{owner_id}")
}

pub fn canvas_node_owner_id(stable_id: &str) -> Option<&str> {
    stable_id.strip_prefix("canvas_node::")
}
