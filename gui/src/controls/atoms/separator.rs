//! 分隔线原子控件。无状态，纯视觉。

use iced::widget::container;
use iced::{Color, Element, Theme};

/// 垂直分隔线（默认 16px 高、1px 宽）
pub fn separator<'a, M: 'a>() -> Element<'a, M> {
    container("")
        .width(1)
        .height(24)
        .style(|_: &Theme| container::Style {
            background: Some(Color::from_rgb8(0xe4, 0xe4, 0xe7).into()),
            ..Default::default()
        })
        .into()
}
