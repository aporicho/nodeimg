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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextFamily {
    Sans,
    Monospace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextWeight {
    Normal,
    Medium,
    Semibold,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub color: Color,
    pub size: f32, // 字号（逻辑像素）
    pub family: TextFamily,
    pub line_height: f32, // 行高倍数
    pub weight: TextWeight,
    pub italic: bool,
}

impl TextStyle {
    pub const DEFAULT_LINE_HEIGHT: f32 = 1.2;

    pub fn new(color: Color, size: f32) -> Self {
        Self {
            color,
            size,
            family: TextFamily::Sans,
            line_height: Self::DEFAULT_LINE_HEIGHT,
            weight: TextWeight::Normal,
            italic: false,
        }
    }

    pub fn with_family(mut self, family: TextFamily) -> Self {
        self.family = family;
        self
    }

    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    pub fn with_weight(mut self, weight: TextWeight) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub color: Color,
    pub offset: [f32; 2], // x, y 偏移
    pub blur: f32,        // 模糊半径
    pub spread: f32,      // 扩展距离
}
