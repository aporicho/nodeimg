use crate::control::ControlRole;
use crate::renderer::Rect;
use crate::tree::layout::{BoxStyle, Decoration, LeafKind};
use crate::tree::{
    NodeKind, NodeLayoutMeta, NodeLocalRuntime, NodeMutationMeta, NodePaintMeta, NodeProps,
    RepaintBoundaryReason, RuntimeSlots, StableId, TreeNode,
};

pub(in crate::overlay::retained) fn container(
    id: String,
    style: BoxStyle,
    decoration: Option<Decoration>,
) -> TreeNode {
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

pub(in crate::overlay::retained) fn leaf(id: String, kind: LeafKind, style: BoxStyle) -> TreeNode {
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

pub(in crate::overlay::retained) trait TreeNodeExt {
    fn with_semantic_role(self, role: ControlRole) -> Self;
    fn with_paint_boundary(self, reason: RepaintBoundaryReason) -> Self;
}

impl TreeNodeExt for TreeNode {
    fn with_semantic_role(mut self, role: ControlRole) -> Self {
        self.props.semantic_role = Some(role);
        self
    }

    fn with_paint_boundary(mut self, reason: RepaintBoundaryReason) -> Self {
        self.paint_meta.set_boundary(reason);
        self
    }
}

pub(in crate::overlay::retained) fn zero_rect() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    }
}
