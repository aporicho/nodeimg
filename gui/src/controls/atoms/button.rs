//! 按钮原子控件。无状态，支持图标 + 文字。

use iced::widget::{button, row, svg, text};
use iced::{Border, Color, Element, Theme};

/// 按钮变体
pub enum ButtonVariant {
    /// 默认：透明背景，zinc-500 文字
    Default,
    /// 强调：深色背景，白色文字
    Primary,
}

/// 创建一个按钮。
/// - `label`: 按钮文字（可为空）
/// - `icon`: 图标路径（可为空，如 "plus.svg"）
/// - `variant`: 视觉变体
/// - `on_press`: 点击时产生的消息
pub fn icon_button<'a, M: Clone + 'a>(
    label: &str,
    icon: &str,
    variant: ButtonVariant,
    on_press: M,
) -> Element<'a, M> {
    let icon_color = match variant {
        ButtonVariant::Default => Color::from_rgb8(0x71, 0x71, 0x7a), // zinc-500
        ButtonVariant::Primary => Color::WHITE,
    };

    let mut content = row![].spacing(6).align_y(iced::Alignment::Center);

    if !icon.is_empty() {
        let icon_path = format!("assets/icons/{icon}");
        let color = icon_color;
        let icon_widget = svg(svg::Handle::from_path(icon_path))
            .width(20)
            .height(20)
            .style(move |_theme, _status| svg::Style {
                color: Some(color),
            });
        content = content.push(icon_widget);
    }

    if !label.is_empty() {
        content = content.push(text(label.to_owned()).size(16));
    }

    let style_fn = match variant {
        ButtonVariant::Default => default_style as fn(&Theme, button::Status) -> button::Style,
        ButtonVariant::Primary => primary_style,
    };

    button(content).on_press(on_press).padding([6, 12]).style(style_fn).into()
}

fn default_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: Color::from_rgb8(0x71, 0x71, 0x7a),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn primary_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Color::from_rgb8(0x18, 0x18, 0x1b).into()),
        text_color: Color::WHITE,
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
