use crate::renderer::Rect;

use super::arrange;
use super::types::{LayoutTree, Position};

/// 两遍布局：measure → arrange。直接写入树节点的 rect。
///
/// 如果根节点自身声明了 `Position::Absolute { x, y }`，
/// 则以该 (x, y) 作为 available 的起点，使根节点 rect 从指定坐标开始。
/// 这支持 Panel 等顶层 widget 把绝对位置编码在 WidgetBuild.style 里。
pub fn layout<T: LayoutTree>(
    tree: &mut T,
    root: T::NodeId,
    available: Rect,
    measure_text: &mut dyn FnMut(&str, f32) -> (f32, f32),
) {
    let effective_available = match tree.style(root).position {
        // 注意：仅调整起点 x/y，w/h 仍以调用方传入为准。
        // PanelProps 使用 Size::Fixed，arrange() 会用 Fixed 值覆盖 available.w/h，行为正确。
        // 若根节点使用 Size::Auto/Fill，available.w/h 会透传，布局结果视调用方传入而定。
        Position::Absolute { x, y } => Rect { x, y, ..available },
        Position::Flow => available,
    };
    arrange::arrange(tree, root, effective_available, measure_text);
}
