use crate::renderer::{Border, Color, Point, Rect, Renderer};
use std::sync::Arc;

/// 纹理句柄：不透明封装。widget 层无需依赖 wgpu。
/// Renderer 负责按 handle 查找真实的 wgpu::TextureView。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u64);

/// 自定义绘制回调。与 Flutter CustomPainter 对齐。
pub trait CustomPainter: std::fmt::Debug + Send + Sync {
    fn paint(&self, renderer: &mut Renderer, rect: Rect);
}

/// 包装 Arc<dyn CustomPainter>，让 LeafKind 能派生 Debug/Clone/PartialEq。
#[derive(Clone)]
pub struct CustomPaintFn(pub Arc<dyn CustomPainter>);

impl std::fmt::Debug for CustomPaintFn {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("CustomPaintFn")
            .field(&Arc::as_ptr(&self.0))
            .finish()
    }
}

impl PartialEq for CustomPaintFn {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Arc::as_ptr(&self.0), Arc::as_ptr(&other.0))
    }
}

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

    // ── 定位（新增）──
    pub position: Position,
    pub transform: Option<Transform>,

    // ── 交互（新增）──
    /// 是否参与命中测试。None 表示沿用 "有 Decoration 则可命中" 的旧判据。
    pub hittable: Option<bool>,
    /// 命中后注册哪些手势识别器。
    pub gestures: Vec<crate::gesture::Gesture>,
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
            position: Position::Flow,
            transform: None,
            hittable: None,
            gestures: Vec::new(),
        }
    }
}

/// 定位模式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    /// 参与 Flexbox 流式布局（默认）。
    Flow,
    /// 自由定位：相对于父容器的绝对坐标。
    Absolute { x: f32, y: f32 },
}

/// 仿射变换。阶段 A 仅定义类型，paint/hit 不实际应用。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translate: [f32; 2],
    pub scale: f32,
    pub rotate: f32,  // 弧度
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translate: [0.0, 0.0],
            scale: 1.0,
            rotate: 0.0,
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
    // ── 基础文本/图像 ──

    /// 文本图元。尺寸在 resolve 阶段由 TextMeasurer 解析，创建时使用 Size::Auto。
    Text {
        content: String,
        font_size: f32,
        color: Color,
    },
    /// 位图图像。
    Image {
        texture: TextureHandle,
        tint: Option<Color>,
    },
    /// 图标（图标字体或 SVG 资源）。
    Icon {
        icon_id: String,
        size: f32,
        color: Color,
    },

    // ── 几何图形 ──

    /// 圆形（端口圆点、单选按钮、状态点等）。
    Circle {
        radius: f32,
        fill: Option<Color>,
        stroke: Option<Border>,
    },
    /// 直线段。
    Line {
        start: Point,
        end: Point,
        width: f32,
        color: Color,
    },
    /// 贝塞尔曲线（三次，由 4 个控制点定义）。
    Curve {
        points: [Point; 4],
        width: f32,
        color: Color,
    },
    /// 自定义路径。阶段 A 留空占位，阶段 C 实现画布时填充。
    Path,

    // ── 应用特化 ──

    /// 画布背景点阵。
    Grid {
        spacing: f32,
        dot_color: Color,
        dot_size: f32,
    },
    /// 节点连线。paint 时根据 port id 查找端口位置算贝塞尔曲线。
    Connection {
        from_port: std::borrow::Cow<'static, str>,
        to_port: std::borrow::Cow<'static, str>,
    },

    // ── 逃生舱 ──

    /// 自定义绘制回调（直方图、色盘、波形图等）。
    CustomPaint(CustomPaintFn),
}

/// 视觉装饰。附加在 Container 上，纯视觉属性，不影响布局。
/// paint 时 Decoration → RectStyle 转换。
#[derive(Debug, Clone, PartialEq)]
pub struct Decoration {
    pub background: Option<Color>,
    pub border: Option<Border>,
    pub radius: [f32; 4],
    pub shadow: Option<crate::renderer::Shadow>,
}
