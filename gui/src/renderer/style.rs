use super::types::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub width: f32,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub color: Color,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f32,
}

impl Stroke {
    pub fn new(width: f32, color: Color) -> Self {
        Self {
            width,
            color,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
        }
    }

    pub fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }

    pub fn with_join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }

    pub fn with_miter_limit(mut self, miter_limit: f32) -> Self {
        self.miter_limit = miter_limit;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fill {
    pub color: Color,
    pub rule: FillRule,
}

impl Fill {
    pub fn non_zero(color: Color) -> Self {
        Self {
            color,
            rule: FillRule::NonZero,
        }
    }

    pub fn even_odd(color: Color) -> Self {
        Self {
            color,
            rule: FillRule::EvenOdd,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Debug, Clone, PartialEq)]
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
