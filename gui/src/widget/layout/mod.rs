mod arrange;
mod measure;
pub mod resolve;
mod types;

pub use types::*;

use crate::renderer::Rect;

/// 两遍布局：measure → arrange。直接写入树节点的 rect。
pub fn layout<T: LayoutTree>(tree: &mut T, root: T::NodeId, available: Rect) {
    arrange::arrange(tree, root, available);
}
