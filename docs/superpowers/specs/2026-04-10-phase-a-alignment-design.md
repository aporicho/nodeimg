# 阶段 A 数据模型扩展设计（对齐新架构文档）

## 背景

`docs/target/2.14.0-concepts.md` 和 `docs/target/2.0.0-gui.md` 经过多轮重写后，定义了四级控件层级（图元 → 原子 → 组合 → 特定框架）和完整的图元/容器/控件属性模型。文档的描述已经超前于代码实现。

当前代码状态：
- `LeafKind` 只有 `Text` 一个变体，文档描述 10 种
- `BoxStyle` 没有 `position`、`transform`、`hittable`、`gestures` 字段
- `Decoration` 没有 `shadow` 字段
- `TextInputProps`、`DropdownProps`、`ToggleProps` 的 `build()` 都是 `todo!()`
- `Gesture` 枚举未定义（`gesture/` 目录下只有 Tap/Drag/LongPress 的识别器实现，没有声明式枚举）

本次工作的目标是让这些"描述与实现"的差距闭合第一档：扩展数据模型，让文档里提到的字段和变体在代码中**真实存在**，但不承担它们的完整行为。

## 目标与非目标

### 目标

1. `LeafKind` 扩展为 10 个变体：`Text / Image / Icon / Circle / Line / Curve / Path / Grid / Connection / CustomPaint`
2. `BoxStyle` 增加 4 个字段：`position: Position`、`transform: Option<Transform>`、`hittable: Option<bool>`、`gestures: Vec<Gesture>`
3. `Decoration` 增加 `shadow: Option<Shadow>` 字段
4. `Gesture` 枚举在 `gesture/kind.rs` 中定义（Tap / DoubleTap / Drag / LongPress）
5. 补齐 `TextInputProps`、`DropdownProps`、`ToggleProps` 的 `build()` 方法

### 非目标

- 不实现 `Position::Absolute` 的实际布局逻辑（arrange.rs 不动）
- 不实现 `Transform` 的 paint 矩阵变换
- 不让 `hittable` 和 `gestures` 被 hit.rs / gesture arena 真正消费
- 不实现 Image / Icon / Path / Grid / Connection / CustomPaint 的实际绘制
- 不接入三个新 build() 的实际交互（TextInput 的光标、Toggle 的点击、Dropdown 的展开）
- 不触碰 PanelLayer / PanelFrame / 面板实例系统
- 不改 `Context` 结构
- 不改 `demo.rs`（除非因编译错误被迫）

这些被明确排除的项属于后续阶段 B（目录重组）和阶段 C（面板系统整合、canvas 分支实现）的范围。

## 指导原则

1. **骨架优先** —— 所有新类型和字段只保证类型体系完整，paint/hit 对新变体允许 no-op。文档里描述的字段要真实存在于代码中，而不是一次做到功能完备。
2. **零业务破坏** —— demo 在本阶段结束后应该仍能运行，视觉和交互与之前完全一致。所有新字段加合理默认值，所有分发 fallthrough 到既有行为。
3. **类型模块干净** —— `Gesture` 定义在 `gesture/` 而非 `widget/layout/types.rs`，避免 types.rs 直接依赖 widget/action 层。
4. **不透明句柄** —— `TextureHandle` 用新类型 wrapper 表示纹理，不让 widget 层泄漏 wgpu 依赖。

## 数据模型设计

### 1. LeafKind 扩展

位置：`gui/src/widget/layout/types.rs`

```rust
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

/// 包装 Arc<dyn CustomPainter>，让 LeafKind 能 derive Debug/Clone/PartialEq。
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
        Arc::as_ptr(&self.0) == Arc::as_ptr(&other.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LeafKind {
    // 基础文本/图像
    Text {
        content: String,
        font_size: f32,
        color: Color,
    },
    Image {
        texture: TextureHandle,
        tint: Option<Color>,
    },
    Icon {
        icon_id: String,
        size: f32,
        color: Color,
    },

    // 几何图形
    Circle {
        radius: f32,
        fill: Option<Color>,
        stroke: Option<Border>,
    },
    Line {
        start: Point,
        end: Point,
        width: f32,
        color: Color,
    },
    Curve {
        points: [Point; 4],
        width: f32,
        color: Color,
    },
    Path {
        // 占位：阶段 A 骨架，阶段 C 填充
    },

    // 应用特化
    Grid {
        spacing: f32,
        dot_color: Color,
        dot_size: f32,
    },
    Connection {
        from_port: String,
        to_port: String,
    },

    // 逃生舱
    CustomPaint(CustomPaintFn),
}
```

**设计要点：**

- `TextureHandle(u64)` 是简单的 ID 壳。阶段 A 里没人创建它，定义为类型就够。真正的 `Arc<wgpu::TextureView>` 映射将在后续阶段由 Renderer 内部维护。
- `Path` 暂留空体（无字段），占位，阶段 C 实现画布时再填。
- `CustomPainter` trait 要求 `Debug + Send + Sync`，以便 `LeafKind` 整体能派生 `Debug`、`Clone`、`PartialEq`。
- `Connection` 用 `String` 作为 port id，和 widget 树的 `Cow<'static, str>` id 体系一致。

### 2. BoxStyle 新增字段

位置：`gui/src/widget/layout/types.rs`

```rust
/// 定位模式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    Flow,                         // 参与 Flexbox（默认）
    Absolute { x: f32, y: f32 },  // 自由定位（脱离 Flexbox 流）
}

/// 仿射变换。用于相机和动画。
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

pub struct BoxStyle {
    // 盒模型
    pub padding: Edges,
    pub margin: Edges,
    pub width: Size,
    pub height: Size,

    // 定位（新增）
    pub position: Position,
    pub transform: Option<Transform>,

    // Flexbox
    pub direction: Direction,
    pub gap: f32,
    pub align_items: Align,
    pub justify_content: Justify,
    pub flex_grow: f32,

    // 尺寸约束
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,

    // 溢出
    pub overflow: Overflow,

    // 交互（新增）
    pub hittable: Option<bool>,
    pub gestures: Vec<crate::gesture::Gesture>,
}
```

**Default 值：**

```rust
impl Default for BoxStyle {
    fn default() -> Self {
        Self {
            // ...旧字段的默认值...
            position: Position::Flow,
            transform: None,
            hittable: None,         // None = 沿用 decoration.is_some() 判据
            gestures: Vec::new(),   // 默认无手势
        }
    }
}
```

**`hittable: Option<bool>` 的语义：**

- `None` —— 按旧逻辑"有 Decoration 则可命中"判断，这样现有代码无需改动 hit.rs 就能继续工作
- `Some(true)` —— 强制可命中（即使没有 Decoration）
- `Some(false)` —— 强制不可命中

阶段 A 里只使用 `None`，其他取值留给后续阶段。

### 3. Gesture 枚举

新文件：`gui/src/gesture/kind.rs`

```rust
/// 节点可声明的手势类型。
///
/// 命中测试后，hit chain 上每个节点的 gestures 列表告诉手势竞技场
/// 应该注册哪些识别器。本枚举与 gesture/ 下已有的识别器一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gesture {
    Tap,
    DoubleTap,
    Drag,
    LongPress,
}
```

更新 `gui/src/gesture/mod.rs` 添加：

```rust
pub mod kind;
pub use kind::Gesture;
```

**不包含 `Resize`** —— Resize 当前是 panel/resize.rs 里的面板特殊代码，会在阶段 C 合并面板系统时才引入相应的 ResizeRecognizer，那时再加入枚举变体。

**依赖方向**：`widget/layout/types.rs` 通过 `crate::gesture::Gesture` 引用。`gesture/` 目前依赖 `widget/action::Action`（识别器产出 Action），不会产生循环依赖。

### 4. Decoration 加 shadow

位置：`gui/src/widget/layout/types.rs`

```rust
use crate::renderer::Shadow;

#[derive(Debug, Clone, PartialEq)]
pub struct Decoration {
    pub background: Option<Color>,
    pub border: Option<Border>,
    pub radius: [f32; 4],
    pub shadow: Option<Shadow>,  // 新增
}
```

前置工作：`renderer/style.rs` 中的 `Shadow` 结构需要 `derive(PartialEq)`。如果其字段（`Color`, `[f32; 2]`, `f32`）都已经 `PartialEq`，直接 `#[derive(PartialEq)]` 即可。

### 5. 三个 todo!() 控件的 build()

#### 5.1 TextInput

位置：`gui/src/widget/atoms/text_input.rs`

视觉：带边框的方框，上方小灰字显示 label，下方正常字显示 value。

```rust
fn build(&self, id: &str) -> WidgetBuild {
    use crate::widget::desc::Desc;
    use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Edges, LeafKind};
    use crate::renderer::{Border, Color};
    use std::borrow::Cow;

    let bg = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    let border_color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };
    let label_color = Color { r: 0.443, g: 0.443, b: 0.478, a: 1.0 };
    let value_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };

    WidgetBuild {
        style: BoxStyle {
            direction: Direction::Column,
            gap: 4.0,
            padding: Edges::symmetric(8.0, 12.0),
            height: Size::Auto,
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(bg),
            border: Some(Border { width: 1.0, color: border_color }),
            radius: [4.0; 4],
            shadow: None,
        }),
        children: vec![
            Desc::Leaf {
                id: Cow::Owned(format!("{id}::label")),
                style: BoxStyle { width: Size::Auto, height: Size::Auto, ..BoxStyle::default() },
                kind: LeafKind::Text {
                    content: self.label.to_string(),
                    font_size: 11.0,
                    color: label_color,
                },
            },
            Desc::Leaf {
                id: Cow::Owned(format!("{id}::value")),
                style: BoxStyle { width: Size::Auto, height: Size::Auto, ..BoxStyle::default() },
                kind: LeafKind::Text {
                    content: self.value.to_string(),
                    font_size: 12.0,
                    color: value_color,
                },
            },
        ],
    }
}
```

#### 5.2 Toggle

位置：`gui/src/widget/atoms/toggle.rs`

视觉：左边一个圆角轨道 + 圆点，右边 label。通过 `justify_content: Start/End` 控制 thumb 位置，避免依赖还未实现的 Absolute 布局。

```rust
fn build(&self, id: &str) -> WidgetBuild {
    use crate::widget::desc::Desc;
    use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Align, Justify, Edges, LeafKind};
    use crate::renderer::Color;
    use std::borrow::Cow;

    let white = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    let off_bg = Color { r: 0.831, g: 0.831, b: 0.847, a: 1.0 };  // zinc-300
    let on_bg = Color { r: 0.231, g: 0.510, b: 0.965, a: 1.0 };   // blue-500
    let label_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };  // zinc-900

    let track_bg = if self.value { on_bg } else { off_bg };
    let thumb_justify = if self.value { Justify::End } else { Justify::Start };

    WidgetBuild {
        style: BoxStyle {
            direction: Direction::Row,
            gap: 8.0,
            align_items: Align::Center,
            height: Size::Auto,
            ..BoxStyle::default()
        },
        decoration: None,
        children: vec![
            // 开关轨道
            Desc::Container {
                id: Cow::Owned(format!("{id}::track")),
                style: BoxStyle {
                    width: Size::Fixed(32.0),
                    height: Size::Fixed(18.0),
                    padding: Edges::all(2.0),
                    direction: Direction::Row,
                    justify_content: thumb_justify,
                    align_items: Align::Center,
                    ..BoxStyle::default()
                },
                decoration: Some(Decoration {
                    background: Some(track_bg),
                    border: None,
                    radius: [9.0; 4],
                    shadow: None,
                }),
                children: vec![
                    // 滑块圆点
                    Desc::Container {
                        id: Cow::Owned(format!("{id}::thumb")),
                        style: BoxStyle {
                            width: Size::Fixed(14.0),
                            height: Size::Fixed(14.0),
                            ..BoxStyle::default()
                        },
                        decoration: Some(Decoration {
                            background: Some(white),
                            border: None,
                            radius: [7.0; 4],
                            shadow: None,
                        }),
                        children: vec![],
                    },
                ],
            },
            // 标签
            Desc::Leaf {
                id: Cow::Owned(format!("{id}::label")),
                style: BoxStyle { width: Size::Auto, height: Size::Auto, ..BoxStyle::default() },
                kind: LeafKind::Text {
                    content: self.label.to_string(),
                    font_size: 12.0,
                    color: label_color,
                },
            },
        ],
    }
}
```

#### 5.3 Dropdown

位置：`gui/src/widget/atoms/dropdown.rs`

视觉：类似 Button，显示选中的选项 + 右侧的 `▾`。用 `▾` 字符而不是 Icon 图元（Icon 还是骨架）。

```rust
fn build(&self, id: &str) -> WidgetBuild {
    use crate::widget::desc::Desc;
    use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Align, Edges, LeafKind};
    use crate::renderer::{Border, Color};
    use std::borrow::Cow;

    let bg = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    let border_color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };
    let text_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };
    let arrow_color = Color { r: 0.443, g: 0.443, b: 0.478, a: 1.0 };

    let selected_text = self.options.get(self.selected)
        .map(|s| s.to_string())
        .unwrap_or_default();

    WidgetBuild {
        style: BoxStyle {
            direction: Direction::Row,
            gap: 8.0,
            align_items: Align::Center,
            padding: Edges::symmetric(8.0, 12.0),
            height: Size::Auto,
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(bg),
            border: Some(Border { width: 1.0, color: border_color }),
            radius: [4.0; 4],
            shadow: None,
        }),
        children: vec![
            Desc::Leaf {
                id: Cow::Owned(format!("{id}::selected")),
                style: BoxStyle { width: Size::Auto, height: Size::Auto, ..BoxStyle::default() },
                kind: LeafKind::Text {
                    content: selected_text,
                    font_size: 12.0,
                    color: text_color,
                },
            },
            Desc::Container {
                id: Cow::Owned(format!("{id}::spacer")),
                style: BoxStyle { flex_grow: 1.0, ..BoxStyle::default() },
                decoration: None,
                children: vec![],
            },
            Desc::Leaf {
                id: Cow::Owned(format!("{id}::arrow")),
                style: BoxStyle { width: Size::Auto, height: Size::Auto, ..BoxStyle::default() },
                kind: LeafKind::Text {
                    content: "▾".to_string(),
                    font_size: 12.0,
                    color: arrow_color,
                },
            },
        ],
    }
}
```

### 共同样式约定

三个控件的外观遵循 shadcn zinc 色系，与 Button / Slider 保持一致：

| Token | RGB (0-1) | 用途 |
|-------|-----------|------|
| white | (1.0, 1.0, 1.0) | 背景 |
| zinc-200 | (0.894, 0.894, 0.906) | 边框 |
| zinc-300 | (0.831, 0.831, 0.847) | 关闭状态轨道 |
| zinc-500 | (0.443, 0.443, 0.478) | 小标签和箭头 |
| zinc-900 | (0.094, 0.094, 0.106) | 主要文字 |
| blue-500 | (0.231, 0.510, 0.965) | 激活状态 |

字号：11（小标签）/ 12（主文本）。padding：8/12（纵/横）。radius：4 px。所有 decoration 都明确写 `shadow: None`（阶段 A 已引入字段）。

## 执行顺序

每一步之后运行 `cargo check -p gui`，确保编译通过再往下走。

| 步骤 | 文件 | 改动 |
|---|---|---|
| 1 | `gui/src/renderer/style.rs` | 给 `Shadow` 加 `#[derive(PartialEq)]`（为了 Decoration 的 PartialEq） |
| 2 | `gui/src/gesture/kind.rs` (new) | 新建 `Gesture` 枚举 |
| 3 | `gui/src/gesture/mod.rs` | `pub mod kind; pub use kind::Gesture;` |
| 4 | `gui/src/widget/layout/types.rs` | LeafKind 扩展到 10 个变体 + TextureHandle + CustomPainter trait + CustomPaintFn |
| 5 | `gui/src/panel/tree/paint.rs` | 更新 LeafKind match，新变体 fallthrough 不绘制 |
| 6 | `gui/src/panel/tree/layout.rs` | `text_content()` 仅匹配 `LeafKind::Text`，其他返回 None |
| 7 | `gui/src/widget/layout/types.rs` | BoxStyle 加 position / transform / hittable / gestures，补 Default |
| 8 | `gui/src/widget/layout/types.rs` | Decoration 加 shadow 字段 |
| 9 | `gui/src/widget/atoms/button.rs` | Decoration 字面量加 `shadow: None` |
| 10 | `gui/src/widget/atoms/slider.rs` | Decoration 字面量加 `shadow: None`（两处：track, fill）|
| 11 | `gui/src/panel/tree/paint.rs` | paint 时 `dec.shadow` 传给 `RectStyle.shadow`（之前硬编码 None） |
| 12 | `gui/src/widget/atoms/text_input.rs` | 实现 build() |
| 13 | `gui/src/widget/atoms/toggle.rs` | 实现 build() |
| 14 | `gui/src/widget/atoms/dropdown.rs` | 实现 build() |
| 15 | 全量构建 | `cargo build -p nodeimg-app --release`，启动 demo 手测 |

### 已知风险

1. **步骤 7**：如果代码中有绕过 `..BoxStyle::default()` 构造 BoxStyle 的地方（完整字段字面量），新字段会要求补齐。需要先 grep `BoxStyle {` 定位所有字面量构造点
2. **步骤 4**：`CustomPainter` trait 的签名使用 `&mut Renderer`，需要确认 `widget/layout/types.rs` 能 import `crate::renderer::Renderer` 而不产生循环依赖。当前 types.rs 已 import `renderer::{Color, Rect, Border}`，再加 `Renderer` 应该也可以
3. **步骤 1**：`Shadow` 结构若有字段未实现 PartialEq，需要手动实现而不是派生

## Commit 策略

两个独立 commit：

### Commit 1: docs

```
docs: 对齐新架构 —— 删除图元控件层级，统一四级分层

files:
  docs/target/2.0.0-gui.md           (已修改)
  docs/target/2.14.0-concepts.md     (已新增)
```

内容是本 session 之前已完成的文档改动，独立提交以便回滚。

### Commit 2: feat(gui)

```
feat(gui): 阶段 A 数据模型扩展 —— LeafKind/BoxStyle/Decoration 对齐文档

- LeafKind 扩展到 10 变体（Text/Image/Icon/Circle/Line/Curve/Path/Grid/Connection/CustomPaint）
- BoxStyle 加 position/transform/hittable/gestures 字段
- Decoration 加 shadow 字段
- 新建 gesture/kind.rs 定义 Gesture 枚举
- 补齐 TextInput/Toggle/Dropdown 的 build() 方法
- 引入 TextureHandle 和 CustomPainter trait

新字段均有默认值，旧代码通过 ..default() 不受影响。
paint/hit/layout 对新 LeafKind 变体 fallthrough 不处理，
等待后续阶段 B/C 实现真正的渲染和交互逻辑。

files:
  gui/src/gesture/kind.rs          (新增)
  gui/src/gesture/mod.rs
  gui/src/renderer/style.rs
  gui/src/widget/layout/types.rs
  gui/src/widget/atoms/button.rs
  gui/src/widget/atoms/slider.rs
  gui/src/widget/atoms/text_input.rs
  gui/src/widget/atoms/toggle.rs
  gui/src/widget/atoms/dropdown.rs
  gui/src/panel/tree/paint.rs
  gui/src/panel/tree/layout.rs
```

## 成功判据

- [ ] `cargo check -p gui` 零报错零 warning
- [ ] `cargo clippy -p gui -- -D warnings` 通过
- [ ] `cargo build -p nodeimg-app --release` 成功
- [ ] 启动 demo 手测：Button、Slider、面板拖拽、面板 resize 与之前行为一致
- [ ] `grep -r "todo!" gui/src/widget/atoms/` 零结果
- [ ] `LeafKind` 有 10 个变体
- [ ] `BoxStyle` 字段包含 `position`, `transform`, `hittable`, `gestures`
- [ ] `Decoration` 字段包含 `shadow`
- [ ] `gui/src/gesture/kind.rs` 存在且定义了 `Gesture` 枚举
- [ ] 两个 commit 成功入库

## 后续工作

本次工作完成后，阶段 B 和阶段 C 仍需执行：

- **阶段 B**（目录重组）：`panel/tree/` 提升到 `tree/`，`widget/layout/` 迁移到 `tree/layout/`
- **阶段 C**（架构整合）：消除 `PanelLayer`/`PanelFrame`，面板变成树里的 Framework Widget，canvas 分支实现，Context 重构

这些工作与阶段 A 解耦，本次不涉及。
