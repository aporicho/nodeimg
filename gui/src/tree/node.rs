use super::layout::{BoxStyle, Decoration, LeafKind, RelayoutBoundaryReason};
use super::props::NodeProps;
use super::runtime_slots::RuntimeSlots;
use super::{Revision, StableId};
use crate::renderer::Rect;
use crate::widget::props::WidgetProps;

pub use super::repaint::NodePaintMeta;

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
    pub layout_meta: NodeLayoutMeta,
    pub paint_meta: NodePaintMeta,
    pub runtime_slots: RuntimeSlots,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeLocalRuntime {
    pub scroll_offset: f32,
    pub content_height: f32,
    pub hovered: bool,
    pub pressed: bool,
    pub visible: bool,
    pub layout: LayoutRuntime,
    pub animation: AnimationRuntime,
}

impl Default for NodeLocalRuntime {
    fn default() -> Self {
        Self {
            scroll_offset: 0.0,
            content_height: 0.0,
            hovered: false,
            pressed: false,
            visible: true,
            layout: LayoutRuntime::default(),
            animation: AnimationRuntime::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutRuntime {
    pub dirty: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimationRuntime {
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeLayoutMeta {
    pub boundary: Option<RelayoutBoundaryReason>,
    pub style_revision: Revision,
    pub text_revision: Revision,
    pub children_revision: Revision,
    pub explicit_rect_revision: Revision,
}

impl NodeLayoutMeta {
    pub fn boundary(reason: RelayoutBoundaryReason) -> Self {
        Self {
            boundary: Some(reason),
            ..Self::default()
        }
    }

    pub fn set_boundary(&mut self, reason: RelayoutBoundaryReason) {
        self.boundary = Some(reason);
    }

    pub fn bump_style(&mut self) {
        self.style_revision = self.style_revision.next();
    }

    pub fn bump_text(&mut self) {
        self.text_revision = self.text_revision.next();
    }

    pub fn bump_children(&mut self) {
        self.children_revision = self.children_revision.next();
    }

    pub fn bump_explicit_rect(&mut self) {
        self.explicit_rect_revision = self.explicit_rect_revision.next();
    }
}

impl Default for NodeLayoutMeta {
    fn default() -> Self {
        Self {
            boundary: None,
            style_revision: Revision::ZERO,
            text_revision: Revision::ZERO,
            children_revision: Revision::ZERO,
            explicit_rect_revision: Revision::ZERO,
        }
    }
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
