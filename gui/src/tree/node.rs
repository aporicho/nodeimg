use super::layout::{BoxStyle, Decoration, LeafKind};
use super::props::NodeProps;
use super::runtime_slots::RuntimeSlots;
use super::StableId;
use crate::renderer::Rect;
use crate::widget::props::WidgetProps;

pub use super::id::NodeId;

#[derive(PartialEq)]
pub enum NodeKind {
    Container,
    Leaf(LeafKind),
    Widget(Box<dyn WidgetProps>),
}

pub struct TreeNode {
    pub id: StableId,
    pub props: NodeProps,
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub kind: NodeKind,
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub local_runtime: NodeLocalRuntime,
    pub runtime_slots: RuntimeSlots,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeLocalRuntime {
    pub scroll_offset: f32,
    pub content_height: f32,
    pub hovered: bool,
    pub pressed: bool,
    pub layout: LayoutRuntime,
    pub animation: AnimationRuntime,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutRuntime {
    pub dirty: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimationRuntime {
    pub active: bool,
}

impl TreeNode {
    pub fn props_match(
        &self,
        style: &BoxStyle,
        decoration: &Option<Decoration>,
        kind: &NodeKind,
    ) -> bool {
        self.style == *style && self.decoration == *decoration && self.kind == *kind
    }

    pub fn scroll_offset(&self) -> f32 {
        self.local_runtime.scroll_offset
    }

    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.local_runtime.scroll_offset = offset;
    }

    pub fn content_height(&self) -> f32 {
        self.local_runtime.content_height
    }

    pub fn set_content_height(&mut self, height: f32) {
        self.local_runtime.content_height = height;
    }
}
