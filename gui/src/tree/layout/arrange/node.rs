use super::absolute::{arrange_absolute_children, AbsoluteArrangeInput};
use super::child_style::FlexChildStyle;
use super::flow::{arrange_flow_children, FlowArrangeInput};
use super::scroll::scroll_offset_for_node;
use crate::renderer::Rect;
use crate::tree::layout::box_model::{
    border_box_from_available, layout_boxes, ChildCoordinateSpace,
};
use crate::tree::layout::measure::measure;
use crate::tree::layout::types::{DesiredSize, Direction, LayoutTree, Position, Size};

pub(super) fn arrange_in_containing_block<T: LayoutTree>(
    tree: &mut T,
    node: T::NodeId,
    available: Rect,
    absolute_containing_block: Rect,
    measure_text: &mut dyn FnMut(&str, &crate::renderer::TextStyle) -> (f32, f32),
) {
    let style = tree.style(node).clone();
    let desired_size = style
        .position
        .is_absolute()
        .then(|| measure(&*tree, node, measure_text));

    let border_available = border_box_from_available(available, style.margin);
    let computed_rect = Rect {
        x: border_available.x,
        y: border_available.y,
        w: match style.width {
            Size::Fixed(w) => w,
            Size::Fill => border_available.w,
            Size::Auto
                if matches!(
                    style.position,
                    Position::Absolute(pos) if pos.has_horizontal_stretch()
                ) =>
            {
                border_available.w
            }
            Size::Auto if matches!(style.position, Position::Absolute(_)) => desired_size
                .map(|size| size.width)
                .unwrap_or(border_available.w),
            _ => border_available.w,
        }
        .clamp(style.min_width, style.max_width),
        h: match style.height {
            Size::Fixed(h) => h,
            Size::Fill => border_available.h,
            Size::Auto
                if matches!(
                    style.position,
                    Position::Absolute(pos) if pos.has_vertical_stretch()
                ) =>
            {
                border_available.h
            }
            Size::Auto if matches!(style.position, Position::Absolute(_)) => desired_size
                .map(|size| size.height)
                .unwrap_or(border_available.h),
            _ => border_available.h,
        }
        .clamp(style.min_height, style.max_height),
    };
    let node_rect = tree.explicit_rect(node).unwrap_or(computed_rect);

    tree.set_rect(node, node_rect);

    let children = tree.children(node);
    if children.is_empty() {
        return;
    }

    let (flow_children, abs_children): (Vec<_>, Vec<_>) = children
        .iter()
        .copied()
        .partition(|&c| !tree.style(c).position.is_absolute());

    let child_space = if style.transform.is_some() {
        ChildCoordinateSpace::Local
    } else {
        ChildCoordinateSpace::Parent
    };
    let boxes = layout_boxes(available, node_rect, style.padding, child_space);
    let _ = (boxes.margin_box, boxes.border_box);
    let content = boxes.content_box;
    let child_absolute_containing_block =
        if style.position.is_positioned() || style.transform.is_some() {
            content
        } else {
            absolute_containing_block
        };

    let is_column = style.direction == Direction::Column;
    let child_sizes: Vec<DesiredSize> = flow_children
        .iter()
        .map(|&c| measure(&*tree, c, measure_text))
        .collect();
    let scroll_offset = scroll_offset_for_node(tree, node, &child_sizes);
    let child_styles: Vec<FlexChildStyle> = flow_children
        .iter()
        .map(|&c| FlexChildStyle::from_box_style(tree.style(c), is_column))
        .collect();

    arrange_flow_children(FlowArrangeInput {
        tree,
        flow_children: &flow_children,
        child_sizes: &child_sizes,
        child_styles: &child_styles,
        parent_style: &style,
        content,
        child_absolute_containing_block,
        scroll_offset,
        measure_text,
    });

    arrange_absolute_children(AbsoluteArrangeInput {
        tree,
        abs_children: &abs_children,
        child_absolute_containing_block,
        measure_text,
    });
}
