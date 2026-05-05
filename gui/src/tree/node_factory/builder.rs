use std::borrow::Cow;

use crate::control::ControlRole;
use crate::renderer::Rect;
use crate::tree::layout::{BoxStyle, Decoration, LeafKind, RelayoutBoundaryReason};
use crate::tree::mutation_meta::NodeMutationMeta;
use crate::tree::node::{NodeKind, NodeLayoutMeta, NodeLocalRuntime, TreeNode};
use crate::tree::props::NodeProps;
use crate::tree::repaint::{NodePaintMeta, RepaintBoundaryReason};
use crate::tree::runtime_slots::{RuntimeSlot, RuntimeSlots};
use crate::tree::{RectMoveInvalidation, StableId};

pub(crate) struct TreeNodeBuilder {
    node: TreeNode,
}

impl TreeNodeBuilder {
    pub(crate) fn container(id: impl Into<StableId>, style: BoxStyle) -> Self {
        Self::new(id, NodeKind::Container, style)
    }

    pub(crate) fn leaf(id: impl Into<StableId>, leaf: LeafKind, style: BoxStyle) -> Self {
        Self::new(id, NodeKind::Leaf(leaf), style)
    }

    fn new(id: impl Into<StableId>, kind: NodeKind, style: BoxStyle) -> Self {
        Self {
            node: TreeNode {
                id: id.into(),
                props: NodeProps::default(),
                style,
                decoration: None,
                kind,
                rect: zero_rect(),
                children: Vec::new(),
                local_runtime: NodeLocalRuntime::default(),
                layout_meta: NodeLayoutMeta::default(),
                paint_meta: NodePaintMeta::default(),
                mutation_meta: NodeMutationMeta::default(),
                runtime_slots: RuntimeSlots::default(),
            },
        }
    }

    pub(crate) fn decoration(mut self, decoration: Decoration) -> Self {
        self.node.decoration = Some(decoration);
        self
    }

    pub(crate) fn maybe_decoration(mut self, decoration: Option<Decoration>) -> Self {
        self.node.decoration = decoration;
        self
    }

    pub(crate) fn rect(mut self, rect: Rect) -> Self {
        self.node.rect = rect;
        self
    }

    pub(crate) fn semantic_role(mut self, role: ControlRole) -> Self {
        self.node.props.semantic_role = Some(role);
        self
    }

    pub(crate) fn owner(mut self, owner: impl Into<Cow<'static, str>>) -> Self {
        self.node.props.owner_id = Some(owner.into());
        self
    }

    pub(crate) fn layout_boundary(mut self, reason: RelayoutBoundaryReason) -> Self {
        self.node.layout_meta.set_boundary(reason);
        self
    }

    pub(crate) fn paint_boundary(mut self, reason: RepaintBoundaryReason) -> Self {
        self.node.paint_meta.set_boundary(reason);
        self
    }

    pub(crate) fn rect_move_invalidation(mut self, invalidation: RectMoveInvalidation) -> Self {
        self.node.mutation_meta.rect_move = invalidation;
        self
    }

    pub(crate) fn runtime_slot<T: RuntimeSlot>(mut self, slot: T) -> Self {
        self.node
            .runtime_slots
            .ensure_with_policy::<T>(T::default_policy(), || slot);
        self
    }

    pub(crate) fn build(self) -> TreeNode {
        self.node
    }
}

impl From<TreeNodeBuilder> for TreeNode {
    fn from(builder: TreeNodeBuilder) -> Self {
        builder.build()
    }
}

fn zero_rect() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    }
}
