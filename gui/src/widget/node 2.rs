use std::borrow::Cow;
use crate::widget::layout::{BoxStyle, Decoration, LeafKind};
use crate::widget::props::WidgetProps;
use crate::renderer::Rect;

pub type NodeId = usize;

#[derive(PartialEq)]
pub enum NodeKind {
    Container,
    Leaf(LeafKind),
    Widget(Box<dyn WidgetProps>),
}

pub struct PanelNode {
    pub id: Cow<'static, str>,
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub kind: NodeKind,
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub scroll_offset: f32,
    pub content_height: f32,
}

impl PanelNode {
    pub fn props_match(&self, style: &BoxStyle, decoration: &Option<Decoration>, kind: &NodeKind) -> bool {
        self.style == *style && self.decoration == *decoration && self.kind == *kind
    }
}
