//! 标题栏组合控件。
//! 组成：TextDisplay(标题) + Space + 可选按钮 + 关闭按钮

use crate::controls::atoms::{icon_button, text_display, ButtonVariant, TextVariant};
use iced::widget::{container, row};
use iced::{Color, Element, Fill};

const TITLE_BAR_HEIGHT: f32 = 40.0;

/// 创建标题栏。
/// - `title`: 标题文字
/// - `close_msg`: 关闭按钮点击时产生的消息
pub fn title_bar<'a, M: Clone + 'a>(title: &str, close_msg: M) -> Element<'a, M> {
    let title_content = row![
        text_display(title, TextVariant::Title),
        iced::widget::Space::new().width(Fill),
        icon_button("", "xmark.svg", ButtonVariant::Default, close_msg),
    ]
    .align_y(iced::Alignment::Center);

    container(title_content)
        .width(Fill)
        .padding(iced::Padding::from([4, 12]))
        .style(title_bar_style)
        .into()
}

/// 创建带额外按钮的标题栏。
/// - `title`: 标题文字
/// - `buttons`: 额外按钮（显示在关闭按钮左侧）
/// - `close_msg`: 关闭按钮点击时产生的消息
pub fn title_bar_with_buttons<'a, M: Clone + 'a>(
    title: &str,
    buttons: Vec<Element<'a, M>>,
    close_msg: M,
) -> Element<'a, M> {
    let mut content = row![
        text_display(title, TextVariant::Title),
        iced::widget::Space::new().width(Fill),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    for btn in buttons {
        content = content.push(btn);
    }

    content = content.push(icon_button(
        "",
        "xmark.svg",
        ButtonVariant::Default,
        close_msg,
    ));

    container(content)
        .width(Fill)
        .padding(iced::Padding::from([4, 12]))
        .style(title_bar_style)
        .into()
}

fn title_bar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgb8(0xfa, 0xfa, 0xfa).into()),
        border: iced::Border {
            color: Color::from_rgb8(0xe4, 0xe4, 0xe7),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
