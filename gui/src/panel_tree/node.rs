use crate::controls::infra::layout::BoxStyle;
use crate::controls::infra::widget::Widget;
use crate::renderer::Rect;

pub type NodeId = usize;

pub enum NodeKind {
    Container { style: BoxStyle },
    Widget(Box<dyn Widget>),
}

pub struct PanelNode {
    pub id: &'static str,
    pub kind: NodeKind,
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub props_hash: u64,
}
