use crate::renderer::{Border, Color, Rect};

/// 盒子样式（Flexbox + 盒模型）
#[derive(Debug, Clone, PartialEq)]
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

    // ── 尺寸约束 ──
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,

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
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            overflow: Overflow::Visible,
        }
    }
}

/// 四边值
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

/// 主轴分布
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// 布局引擎的树抽象。Flexbox 引擎通过此 trait 操作任意树结构。
pub trait LayoutTree {
    type NodeId: Copy;

    /// 读取节点的布局样式（纯数据，已预计算）
    fn style(&self, node: Self::NodeId) -> &BoxStyle;

    /// 获取子节点列表（返回 Vec 以避免借用冲突）
    fn children(&self, node: Self::NodeId) -> Vec<Self::NodeId>;

    /// 布局引擎算出位置后直接写入节点
    fn set_rect(&mut self, node: Self::NodeId, rect: Rect);

    /// 滚动容器的当前偏移量（非滚动节点返回 0.0）
    fn scroll_offset(&self, node: Self::NodeId) -> f32;

    /// 布局引擎在发现 Overflow::Scroll 容器时调用，记录内容总高度
    fn set_content_height(&mut self, node: Self::NodeId, height: f32);

    /// 如果节点是文字叶子，返回 (文字内容, 字号)。layout 引擎在 measure 阶段调用。
    fn text_content(&self, node: Self::NodeId) -> Option<(&str, f32)>;
}

/// 渲染图元类型。叶子节点的具体内容。
#[derive(Debug, Clone, PartialEq)]
pub enum LeafKind {
    /// 文本图元。尺寸在 resolve 阶段由 TextMeasurer 解析，创建时使用 Size::Auto。
    Text {
        content: String,
        font_size: f32,
        color: Color,
    },
}

/// 视觉装饰。附加在 Container 上，纯视觉属性，不影响布局。
/// paint 时 Decoration → RectStyle 转换（shadow 暂不支持）。
#[derive(Debug, Clone, PartialEq)]
pub struct Decoration {
    pub background: Option<Color>,
    pub border: Option<Border>,
    pub radius: [f32; 4],
}
