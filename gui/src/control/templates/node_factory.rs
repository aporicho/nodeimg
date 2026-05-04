use crate::control::ControlRole;
use crate::renderer::Rect;
use crate::tree::layout::{BoxStyle, Decoration, LeafKind};
use crate::tree::{
    NodeKind, NodeLayoutMeta, NodeLocalRuntime, NodeMutationMeta, NodePaintMeta, NodeProps,
    RuntimeSlot, RuntimeSlots, StableId, TreeNode,
};

pub(super) fn container(id: String, style: BoxStyle, decoration: Option<Decoration>) -> TreeNode {
    TreeNode {
        id: StableId::from(id),
        props: NodeProps::default(),
        style,
        decoration,
        kind: NodeKind::Container,
        rect: zero_rect(),
        children: Vec::new(),
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: NodeLayoutMeta::default(),
        paint_meta: NodePaintMeta::default(),
        mutation_meta: NodeMutationMeta::default(),
        runtime_slots: RuntimeSlots::default(),
    }
}

pub(super) fn leaf(id: String, kind: LeafKind, style: BoxStyle) -> TreeNode {
    TreeNode {
        id: StableId::from(id),
        props: NodeProps::default(),
        style,
        decoration: None,
        kind: NodeKind::Leaf(kind),
        rect: zero_rect(),
        children: Vec::new(),
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: NodeLayoutMeta::default(),
        paint_meta: NodePaintMeta::default(),
        mutation_meta: NodeMutationMeta::default(),
        runtime_slots: RuntimeSlots::default(),
    }
}

pub(super) trait TreeNodeExt {
    fn with_semantic_role(self, role: ControlRole) -> Self;
    fn with_runtime_slot<T: RuntimeSlot>(self, slot: T) -> Self;
}

impl TreeNodeExt for TreeNode {
    fn with_semantic_role(mut self, role: ControlRole) -> Self {
        self.props.semantic_role = Some(role);
        self
    }

    fn with_runtime_slot<T: RuntimeSlot>(mut self, slot: T) -> Self {
        self.runtime_slots
            .ensure_with_policy::<T>(T::default_policy(), || slot);
        self
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
