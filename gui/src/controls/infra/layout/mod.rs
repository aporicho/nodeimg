mod arrange;
mod measure;
pub mod scroll;
mod types;

pub use types::*;

use crate::renderer::Rect;

/// 两遍布局：measure → arrange。返回所有有 id 节点的最终位置。
pub fn layout(root: &LayoutBox, available: Rect) -> LayoutResult {
    let mut result = LayoutResult {
        rects: Vec::new(),
        scroll_areas: Vec::new(),
    };
    arrange::arrange(root, available, &mut result.rects, &mut result.scroll_areas);
    result
}
