use super::node::arrange_in_containing_block;
use crate::renderer::Rect;
use crate::tree::layout::types::LayoutTree;

/// 自顶向下分配位置，直接写入树节点。
pub(crate) fn arrange<T: LayoutTree>(
    tree: &mut T,
    node: T::NodeId,
    available: Rect,
    measure_text: &mut dyn FnMut(&str, &crate::renderer::TextStyle) -> (f32, f32),
) {
    arrange_in_containing_block(tree, node, available, available, measure_text);
}
