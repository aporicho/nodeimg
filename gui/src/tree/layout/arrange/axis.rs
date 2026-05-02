use super::child_style::FlexChildStyle;
use crate::renderer::Rect;
use crate::tree::layout::types::{DesiredSize, Size};

pub(super) fn desired_main_size(size: &DesiredSize, is_column: bool) -> f32 {
    if is_column {
        size.height
    } else {
        size.width
    }
}

pub(super) fn desired_cross_size(size: &DesiredSize, is_column: bool) -> f32 {
    if is_column {
        size.width
    } else {
        size.height
    }
}

pub(super) fn content_main_size(content: Rect, is_column: bool) -> f32 {
    if is_column {
        content.h
    } else {
        content.w
    }
}

pub(super) fn content_cross_size(content: Rect, is_column: bool) -> f32 {
    if is_column {
        content.w
    } else {
        content.h
    }
}

pub(super) fn main_axis_size(style: FlexChildStyle, is_column: bool) -> Size {
    if is_column {
        style.height
    } else {
        style.width
    }
}

pub(super) fn is_main_fill(style: FlexChildStyle, is_column: bool) -> bool {
    matches!(main_axis_size(style, is_column), Size::Fill)
}
