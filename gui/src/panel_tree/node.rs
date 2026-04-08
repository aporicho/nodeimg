use crate::renderer::{Color, Rect};

pub type NodeId = usize;

/// 面板树中的持久节点。
pub struct PanelNode {
    pub id: &'static str,
    pub kind: NodeKind,
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub props_hash: u64,
}

pub enum NodeKind {
    Column,
    Button { label: &'static str, color: Color },
}
