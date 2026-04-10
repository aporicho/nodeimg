use super::types::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub width: f32,
    pub color: Color,
}

pub struct RectStyle {
    pub color: Color,
    pub border: Option<Border>,
    pub radius: [f32; 4], // 左上、右上、右下、左下
    pub shadow: Option<Shadow>,
}

pub struct TextStyle {
    pub color: Color,
    pub size: f32, // 字号（逻辑像素）
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub color: Color,
    pub offset: [f32; 2], // x, y 偏移
    pub blur: f32,         // 模糊半径
    pub spread: f32,       // 扩展距离
}
