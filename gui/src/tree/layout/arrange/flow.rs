use super::axis::{
    content_cross_size, content_main_size, desired_cross_size, desired_main_size, is_main_fill,
};
use super::child_style::FlexChildStyle;
use super::node::arrange_in_containing_block;
use super::shrink::resolve_shrink_main_sizes;
use crate::renderer::Rect;
use crate::tree::layout::box_model::relative_offset_rect;
use crate::tree::layout::types::{
    Align, BoxStyle, DesiredSize, Direction, Justify, LayoutTree, Position,
};

pub(super) struct FlowArrangeInput<'a, T: LayoutTree> {
    pub(super) tree: &'a mut T,
    pub(super) flow_children: &'a [T::NodeId],
    pub(super) child_sizes: &'a [DesiredSize],
    pub(super) child_styles: &'a [FlexChildStyle],
    pub(super) parent_style: &'a BoxStyle,
    pub(super) content: Rect,
    pub(super) child_absolute_containing_block: Rect,
    pub(super) scroll_offset: f32,
    pub(super) measure_text: &'a mut dyn FnMut(&str, &crate::renderer::TextStyle) -> (f32, f32),
}

pub(super) fn arrange_flow_children<T: LayoutTree>(input: FlowArrangeInput<'_, T>) {
    let is_column = input.parent_style.direction == Direction::Column;
    let n = input.child_sizes.len();
    let total_gap = if n > 1 {
        input.parent_style.gap * (n as f32 - 1.0)
    } else {
        0.0
    };
    let main_available = content_main_size(input.content, is_column);
    let base_main_sizes: Vec<f32> = input
        .child_sizes
        .iter()
        .map(|size| desired_main_size(size, is_column))
        .collect();
    let total_base_main = base_main_sizes.iter().sum::<f32>() + total_gap;
    let should_shrink = total_base_main > main_available;

    let mut fixed_main: f32 = total_gap;
    let mut total_grow: f32 = 0.0;
    for (i, child_style) in input.child_styles.iter().enumerate() {
        let child_main = base_main_sizes[i];
        let is_fill = is_main_fill(*child_style, is_column);

        if !should_shrink && (child_style.grow > 0.0 || is_fill) {
            total_grow += if child_style.grow > 0.0 {
                child_style.grow
            } else {
                1.0
            };
            fixed_main += child_style.main_margin;
        } else {
            fixed_main += child_main;
        }
    }

    let remaining = (main_available - fixed_main).max(0.0);
    let shrink_main_sizes = if should_shrink {
        Some(resolve_shrink_main_sizes(
            &base_main_sizes,
            input.child_styles,
            is_column,
            main_available,
            total_gap,
        ))
    } else {
        None
    };

    let (mut main_offset, extra_gap) = match input.parent_style.justify_content {
        Justify::Start => (0.0, 0.0),
        Justify::End => (remaining.max(0.0), 0.0),
        Justify::Center => (remaining.max(0.0) / 2.0, 0.0),
        Justify::SpaceBetween => {
            if n > 1 && total_grow == 0.0 {
                (0.0, remaining / (n as f32 - 1.0))
            } else {
                (0.0, 0.0)
            }
        }
    };

    main_offset -= input.scroll_offset;

    for (i, &child) in input.flow_children.iter().enumerate() {
        let child_desired = &input.child_sizes[i];
        let child_style = input.child_styles[i];
        let is_fill = is_main_fill(child_style, is_column);
        let effective_grow = if should_shrink {
            0.0
        } else if child_style.grow > 0.0 {
            child_style.grow
        } else if is_fill {
            1.0
        } else {
            0.0
        };

        let child_main = if let Some(shrink_sizes) = &shrink_main_sizes {
            shrink_sizes[i]
        } else if effective_grow > 0.0 {
            remaining * effective_grow / total_grow
        } else {
            desired_main_size(child_desired, is_column)
        };

        let cross_available = content_cross_size(input.content, is_column);
        let child_cross_desired = desired_cross_size(child_desired, is_column);
        let child_align = child_style
            .align_self
            .unwrap_or(input.parent_style.align_items);
        let (cross_offset, child_cross) = match child_align {
            Align::Stretch => (0.0, cross_available),
            Align::Start => (0.0, child_cross_desired),
            Align::End => (cross_available - child_cross_desired, child_cross_desired),
            Align::Center => (
                (cross_available - child_cross_desired) / 2.0,
                child_cross_desired,
            ),
        };

        let child_rect = if is_column {
            Rect {
                x: input.content.x + cross_offset,
                y: input.content.y + main_offset,
                w: child_cross,
                h: child_main,
            }
        } else {
            Rect {
                x: input.content.x + main_offset,
                y: input.content.y + cross_offset,
                w: child_main,
                h: child_cross,
            }
        };

        let child_rect = match child_style.position {
            Position::Relative(position) => relative_offset_rect(child_rect, position.inset),
            Position::Flow | Position::Absolute(_) => child_rect,
        };

        arrange_in_containing_block(
            input.tree,
            child,
            child_rect,
            input.child_absolute_containing_block,
            input.measure_text,
        );

        main_offset += child_main + input.parent_style.gap + extra_gap;
    }
}
