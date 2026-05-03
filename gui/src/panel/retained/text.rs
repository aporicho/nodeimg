use crate::tree::layout::{BoxStyle, Size, TextLayout, TextOverflow};

pub(in crate::panel::retained) fn label_style() -> BoxStyle {
    BoxStyle {
        width: Size::Fill,
        height: Size::Auto,
        flex_shrink: 1.0,
        ..BoxStyle::default()
    }
}

pub(in crate::panel::retained) fn ellipsis_text_layout() -> TextLayout {
    TextLayout {
        overflow: TextOverflow::Ellipsis,
        ..TextLayout::default()
    }
}
