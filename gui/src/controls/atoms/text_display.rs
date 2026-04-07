//! 文本显示原子控件。无状态，只读文本。

use iced::widget::text;
use iced::{Color, Element};

/// 文本显示变体
pub enum TextVariant {
    /// 标签：zinc-500，16px
    Label,
    /// 值：zinc-900，16px
    Value,
    /// 标题：zinc-500，16px（同 Label，语义区分）
    Title,
}

/// 创建只读文本。
pub fn text_display<'a, M: 'a>(content: &str, variant: TextVariant) -> Element<'a, M> {
    let color = match variant {
        TextVariant::Label | TextVariant::Title => Color::from_rgb8(0x71, 0x71, 0x7a),
        TextVariant::Value => Color::from_rgb8(0x18, 0x18, 0x1b),
    };

    text(content.to_owned()).size(16).color(color).into()
}
