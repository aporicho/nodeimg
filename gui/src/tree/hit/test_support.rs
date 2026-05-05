use crate::renderer::{Color, Rect};
use crate::tree::layout::{BoxStyle, Decoration, LeafKind};
use crate::tree::node::{NodeKind, TreeNode};
use crate::tree::TreeNodeBuilder;

pub(super) fn node_with_rect(
    id: &'static str,
    style: BoxStyle,
    decoration: Option<Decoration>,
    kind: NodeKind,
    rect: Rect,
) -> TreeNode {
    match kind {
        NodeKind::Container => TreeNodeBuilder::container(id, style),
        NodeKind::Leaf(leaf) => TreeNodeBuilder::leaf(id, leaf, style),
    }
    .maybe_decoration(decoration)
    .rect(rect)
    .build()
}

/// 构造一个设定好 rect 的 Container 节点
pub(super) fn container_with_rect(
    style: BoxStyle,
    decoration: Option<Decoration>,
    rect: Rect,
) -> TreeNode {
    node_with_rect("test", style, decoration, NodeKind::Container, rect)
}

pub(super) fn leaf_with_rect(
    id: &'static str,
    leaf: LeafKind,
    style: BoxStyle,
    rect: Rect,
) -> TreeNode {
    node_with_rect(id, style, None, NodeKind::Leaf(leaf), rect)
}

pub(super) fn hittable_style() -> BoxStyle {
    BoxStyle {
        hittable: true,
        ..Default::default()
    }
}

/// 默认 decoration（仅用于绘制；命中必须由 style 显式声明）
pub(super) fn decor() -> Decoration {
    Decoration {
        background: Some(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }),
        border: None,
        radius: [0.0; 4],
        shadow: None,
    }
}
