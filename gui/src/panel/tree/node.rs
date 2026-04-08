use crate::widget::layout::BoxStyle;
use crate::widget::r#trait::Widget;
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
