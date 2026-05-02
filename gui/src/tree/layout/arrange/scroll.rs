use crate::tree::layout::types::{DesiredSize, Direction, LayoutTree, Overflow};

pub(super) fn scroll_offset_for_node<T: LayoutTree>(
    tree: &mut T,
    node: T::NodeId,
    child_sizes: &[DesiredSize],
) -> f32 {
    let style = tree.style(node);
    if style.overflow != Overflow::Scroll {
        return 0.0;
    }

    let offset = tree.scroll_offset(node);
    let n = child_sizes.len();
    let total_gap = if n > 1 {
        style.gap * (n as f32 - 1.0)
    } else {
        0.0
    };
    let content_height = match style.direction {
        Direction::Column => child_sizes.iter().map(|s| s.height).sum::<f32>() + total_gap,
        Direction::Row => child_sizes.iter().map(|s| s.height).fold(0.0f32, f32::max),
    };

    tree.set_content_height(node, content_height);
    offset
}
