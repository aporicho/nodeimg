use crate::tree::layout::{BoxStyle, Decoration, LeafKind};
use crate::tree::TreeNodeBuilder;

pub(crate) fn container(
    id: String,
    style: BoxStyle,
    decoration: Option<Decoration>,
) -> TreeNodeBuilder {
    match decoration {
        Some(decoration) => TreeNodeBuilder::container(id, style).decoration(decoration),
        None => TreeNodeBuilder::container(id, style),
    }
}

pub(crate) fn leaf(id: String, kind: LeafKind, style: BoxStyle) -> TreeNodeBuilder {
    TreeNodeBuilder::leaf(id, kind, style)
}
