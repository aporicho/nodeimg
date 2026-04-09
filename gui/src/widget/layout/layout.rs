use crate::renderer::Rect;

use super::arrange;
use super::types::LayoutTree;

/// 两遍布局：measure → arrange。直接写入树节点的 rect。
pub fn layout<T: LayoutTree>(tree: &mut T, root: T::NodeId, available: Rect) {
    arrange::arrange(tree, root, available);
}
