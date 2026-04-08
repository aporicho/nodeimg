use crate::renderer::Rect;

/// 布局节点。有 children 的是容器，没有的是叶子。
pub struct LayoutBox {
    pub id: Option<&'static str>,
    pub style: BoxStyle,
    pub children: Vec<LayoutBox>,
}

/// 盒子样式（Flexbox + 盒模型）
#[derive(Clone)]
pub struct BoxStyle {
    // ── 盒模型 ──
    pub padding: Edges,
    pub margin: Edges,
    pub width: Size,
    pub height: Size,

    // ── Flexbox ──
    pub direction: Direction,
    pub gap: f32,
    pub align_items: Align,
    pub justify_content: Justify,
    pub flex_grow: f32,

    // ── 溢出 ──
    pub overflow: Overflow,
}

impl Default for BoxStyle {
    fn default() -> Self {
        Self {
            padding: Edges::ZERO,
            margin: Edges::ZERO,
            width: Size::Auto,
            height: Size::Auto,
            direction: Direction::Column,
            gap: 0.0,
            align_items: Align::Stretch,
            justify_content: Justify::Start,
            flex_grow: 0.0,
            overflow: Overflow::Visible,
        }
    }
}

/// 四边值
#[derive(Debug, Clone, Copy)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub const ZERO: Self = Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 };

    pub fn all(v: f32) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }

    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
    }

    pub fn horizontal(&self) -> f32 { self.left + self.right }
    pub fn vertical(&self) -> f32 { self.top + self.bottom }
}

/// 尺寸
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    Fixed(f32),
    Auto,
    Fill,
}

/// 主轴方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Row,
    Column,
}

/// 交叉轴对齐
#[derive(Debug, Clone, Copy)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

/// 主轴分布
#[derive(Debug, Clone, Copy)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

/// 溢出行为
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
}

/// measure 阶段的中间结果
#[derive(Debug, Clone, Copy)]
pub(crate) struct DesiredSize {
    pub width: f32,
    pub height: f32,
}

/// 布局结果
pub struct LayoutResult {
    pub rects: Vec<(&'static str, Rect)>,
    pub scroll_areas: Vec<ScrollArea>,
}

/// 可滚动区域信息
pub struct ScrollArea {
    pub id: &'static str,
    pub viewport: Rect,
    pub content_height: f32,
    pub offset: f32,
}
