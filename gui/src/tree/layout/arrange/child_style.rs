use crate::tree::layout::types::{Align, BoxStyle, Position, Size};

#[derive(Debug, Clone, Copy)]
pub(super) struct FlexChildStyle {
    pub(super) grow: f32,
    pub(super) shrink: f32,
    pub(super) width: Size,
    pub(super) height: Size,
    pub(super) position: Position,
    pub(super) align_self: Option<Align>,
    pub(super) min_width: f32,
    pub(super) max_width: f32,
    pub(super) min_height: f32,
    pub(super) max_height: f32,
    pub(super) main_margin: f32,
}

impl FlexChildStyle {
    pub(super) fn from_box_style(style: &BoxStyle, is_column: bool) -> Self {
        let main_margin = if is_column {
            style.margin.vertical()
        } else {
            style.margin.horizontal()
        };
        Self {
            grow: style.flex_grow,
            shrink: style.flex_shrink,
            width: style.width,
            height: style.height,
            position: style.position,
            align_self: style.align_self,
            min_width: style.min_width,
            max_width: style.max_width,
            min_height: style.min_height,
            max_height: style.max_height,
            main_margin,
        }
    }
}
