use super::node::arrange_in_containing_block;
use crate::renderer::Rect;
use crate::tree::layout::box_model::absolute_available_rect;
use crate::tree::layout::measure::measure;
use crate::tree::layout::types::{LayoutTree, Position};

pub(super) struct AbsoluteArrangeInput<'a, T: LayoutTree> {
    pub(super) tree: &'a mut T,
    pub(super) abs_children: &'a [T::NodeId],
    pub(super) child_absolute_containing_block: Rect,
    pub(super) measure_text: &'a mut dyn FnMut(&str, &crate::renderer::TextStyle) -> (f32, f32),
}

pub(super) fn arrange_absolute_children<T: LayoutTree>(input: AbsoluteArrangeInput<'_, T>) {
    for &child in input.abs_children {
        let child_style = input.tree.style(child).clone();
        let position = match child_style.position {
            Position::Absolute(position) => position,
            Position::Flow | Position::Relative(_) => {
                unreachable!("partition 已保证这里只有 Absolute")
            }
        };
        let desired = measure(&*input.tree, child, input.measure_text);

        let child_available = absolute_available_rect(
            input.child_absolute_containing_block,
            position,
            child_style.width,
            child_style.height,
            (desired.width, desired.height),
        );

        arrange_in_containing_block(
            input.tree,
            child,
            child_available,
            input.child_absolute_containing_block,
            input.measure_text,
        );
    }
}
