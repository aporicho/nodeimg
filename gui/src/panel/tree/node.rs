use crate::widget::layout::{BoxStyle, Decoration, LeafKind};
use crate::renderer::Rect;

pub type NodeId = usize;

pub enum NodeKind {
    Container { style: BoxStyle, decoration: Option<Decoration> },
    Leaf { style: BoxStyle, kind: LeafKind },
}

pub struct PanelNode {
    pub id: &'static str,
    pub kind: NodeKind,
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub props_hash: u64,
    pub scroll_offset: f32,
    pub content_height: f32,
}
