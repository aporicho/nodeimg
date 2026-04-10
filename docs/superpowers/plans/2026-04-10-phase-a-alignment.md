# 阶段 A 数据模型扩展实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 扩展 `gui/` crate 的数据模型以对齐 `2.14.0-concepts.md` 和 `2.0.0-gui.md` 文档——LeafKind 从 1 变体扩展到 10 变体，BoxStyle 加 4 个字段，Decoration 加 shadow，补齐 3 个 todo!() 控件的 build()。

**Architecture:** 骨架优先策略。类型体系扩展到文档描述的完整状态，但 paint/hit/layout 对新变体 fallthrough 不处理。所有新字段带默认值，demo 行为与实施前完全一致。依赖方向：`gesture/kind.rs`（新增）独立定义 Gesture 枚举，`widget/layout/types.rs` 通过 `crate::gesture::Gesture` 引用。

**Tech Stack:** Rust 2021 edition，wgpu 作为底层渲染，Cow<'static, str> 作为 id 类型，Arc<dyn Trait> 作为 trait object 包装。

**Spec:** `docs/superpowers/specs/2026-04-10-phase-a-alignment-design.md`

---

## 背景摘要

本次工作实现 spec 定义的阶段 A 全部内容。工作全程不修改 demo.rs、Context 结构、PanelLayer/PanelFrame 系统。每一步之后跑 `cargo check -p gui` 验证编译。

**文档描述的 LeafKind 10 变体**：Text、Image、Icon、Circle、Line、Curve、Path、Grid、Connection、CustomPaint

**BoxStyle 新增 4 字段**：position (Flow/Absolute)、transform (仿射变换)、hittable (三态)、gestures (Vec<Gesture>)

**Decoration 新增字段**：shadow: Option<Shadow>

**新建 Gesture 枚举**：Tap / DoubleTap / Drag / LongPress（不包含 Resize）

**补齐 3 个 todo!() 控件**：TextInput、Toggle、Dropdown 的 build() 方法

## 文件结构

### 新建文件

| 路径 | 职责 |
|---|---|
| `gui/src/gesture/kind.rs` | 定义 `Gesture` 枚举。声明式的手势类型，给 BoxStyle 引用 |

### 修改文件

| 路径 | 改动内容 |
|---|---|
| `gui/src/renderer/style.rs` | `Shadow` 加 `#[derive(PartialEq)]`（连带影响 `RectStyle`） |
| `gui/src/gesture/mod.rs` | 导出 `kind::Gesture` |
| `gui/src/widget/layout/types.rs` | LeafKind 扩到 10 变体，BoxStyle 加 4 字段，Decoration 加 shadow，新增 TextureHandle/CustomPainter/CustomPaintFn/Position/Transform |
| `gui/src/panel/tree/paint.rs` | 更新 LeafKind 的 match，shadow 通路接通 |
| `gui/src/panel/tree/layout.rs` | `text_content()` match 兼容新 LeafKind 变体 |
| `gui/src/widget/atoms/button.rs` | Decoration 字面量加 `shadow: None` |
| `gui/src/widget/atoms/slider.rs` | Decoration 两处字面量加 `shadow: None` |
| `gui/src/widget/atoms/text_input.rs` | 实现 build() |
| `gui/src/widget/atoms/toggle.rs` | 实现 build() |
| `gui/src/widget/atoms/dropdown.rs` | 实现 build() |

### 本次不涉及的文件

- `gui/src/widget/layout/arrange.rs` —— Absolute 布局留给后续
- `gui/src/widget/layout/measure.rs` —— 新 LeafKind 变体在 measure 阶段靠 text_content() 返回 None 自然处理
- `gui/src/panel/tree/hit.rs` —— hittable 字段语义为 "None 沿用旧判据"，hit.rs 无需改动
- `gui/src/gesture/arena.rs` 等识别器文件 —— Gesture 枚举只是声明式类型，不影响现有识别器代码
- `app/src/demo.rs` —— 除非编译错误强迫，否则不改
- Context.rs、PanelLayer、PanelFrame、PanelRenderer 等架构级文件

---

## Task 1: Shadow 加 PartialEq 派生

**Files:**
- Modify: `gui/src/renderer/style.rs:22`

**背景：** `Shadow` 当前派生了 `Debug, Clone, Copy`，缺 `PartialEq`。Decoration 要派生 `PartialEq`，要求其所有字段（包括 `shadow: Option<Shadow>`）都支持。`Shadow` 的字段（`Color`, `[f32; 2]`, `f32`, `f32`）都已经 `PartialEq`，直接加派生即可。

- [ ] **Step 1: 修改 Shadow 派生**

在 `gui/src/renderer/style.rs:22` 找到：

```rust
#[derive(Debug, Clone, Copy)]
pub struct Shadow {
    pub color: Color,
    pub offset: [f32; 2],
    pub blur: f32,
    pub spread: f32,
}
```

改为：

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub color: Color,
    pub offset: [f32; 2],
    pub blur: f32,
    pub spread: f32,
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：`Checking gui v0.1.0 ... Finished` 无错误、无 warning。

- [ ] **Step 3: 暂不 commit**

本次所有改动作为一个 commit，在最后统一提交。

---

## Task 2: 新建 Gesture 枚举

**Files:**
- Create: `gui/src/gesture/kind.rs`
- Modify: `gui/src/gesture/mod.rs`

**背景：** 定义声明式的手势类型枚举，给 BoxStyle 的 `gestures: Vec<Gesture>` 引用。`widget/layout/types.rs → gesture/kind.rs` 的依赖方向没有循环（gesture 模块反向依赖 widget/action 的 Action）。

- [ ] **Step 1: 创建 kind.rs**

新建 `gui/src/gesture/kind.rs`，内容：

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

- [ ] **Step 2: 导出到 mod.rs**

修改 `gui/src/gesture/mod.rs`，在模块声明段加 `mod kind;`，在 `pub use` 段加 `pub use kind::Gesture;`。完整结果：

```rust
mod arena;
mod drag;
mod kind;
mod long_press;
mod recognizer;
mod tap;

pub use arena::GestureArena;
pub use drag::DragRecognizer;
pub use kind::Gesture;
pub use long_press::LongPressRecognizer;
pub use recognizer::{GestureDisposition, GestureRecognizer};
pub use tap::TapRecognizer;
```

- [ ] **Step 3: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：无错误、无 warning。`Gesture` 类型对外可见但目前没有使用者。

---

## Task 3: 扩展 LeafKind 到 10 变体（含 TextureHandle、CustomPainter）

**Files:**
- Modify: `gui/src/widget/layout/types.rs:1, 147-154`

**背景：** 这是最核心的类型扩展。引入 `TextureHandle` 作为不透明纹理句柄，`CustomPainter` trait + `CustomPaintFn` 包装让 LeafKind 能派生 Debug/Clone/PartialEq。

- [ ] **Step 1: 更新 types.rs 的 import**

找到 `gui/src/widget/layout/types.rs:1`：

```rust
use crate::renderer::{Border, Color, Rect};
```

改为：

```rust
use crate::renderer::{Border, Color, Point, Rect, Renderer};
use std::sync::Arc;
```

- [ ] **Step 2: 在 types.rs 顶部添加 TextureHandle、CustomPainter、CustomPaintFn**

在 import 之后、`BoxStyle` 定义之前（大约 `gui/src/widget/layout/types.rs:3` 之前）插入：

```rust
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
        Arc::as_ptr(&self.0) == Arc::as_ptr(&other.0)
    }
}

```

- [ ] **Step 3: 替换 LeafKind 定义**

找到 `gui/src/widget/layout/types.rs:146-154` 现有的 LeafKind：

```rust
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
```

替换为：

```rust
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
```

- [ ] **Step 4: 验证编译（会报 match 不完整）**

```bash
cargo check -p gui 2>&1 | tail -40
```

期望：看到 `gui/src/panel/tree/paint.rs` 和/或 `gui/src/panel/tree/layout.rs` 报 `non-exhaustive patterns` 或 `unused variable` 之类的错误/警告。这是 Task 4 和 Task 5 要修的。如果有其他意外错误，先修好再往下。

---

## Task 4: 验证 paint.rs 兼容新 LeafKind 变体

**Files:**
- Read-only check: `gui/src/panel/tree/paint.rs:23-29`

**背景：** paint.rs 当前用 `if let LeafKind::Text { ... }` 处理文本，不是 `match`。`if let` 不要求穷尽变体，所以 Task 3 新增 9 种变体**不会**导致 paint.rs 编译失败。本 Task 只做验证，不改代码。Decoration.shadow 的接通是 Task 9 的工作。

- [ ] **Step 1: 确认 paint.rs 现状**

Read `gui/src/panel/tree/paint.rs:23-29`。应该看到：

```rust
if let NodeKind::Leaf(LeafKind::Text { content, font_size, color }) = &node.kind {
    renderer.draw_text(
        Point { x: rect.x, y: rect.y },
        content,
        &TextStyle { color: *color, size: *font_size },
    );
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：paint.rs 无报错。如果有意外错误/警告指向 paint.rs，停下分析。

---

## Task 5: 更新 layout.rs 的 text_content() match

**Files:**
- Modify: `gui/src/panel/tree/layout.rs:33-38`

**背景：** `text_content()` 当前用 `match` 匹配 `LeafKind::Text`，其他分支 `_` 返回 None。新增 9 个变体会让 `_` 分支自动覆盖它们。**但要确认没有被误判为 exhaustive**。

- [ ] **Step 1: 检查 layout.rs 现状**

Read `gui/src/panel/tree/layout.rs:33-38`，当前代码：

```rust
fn text_content(&self, node: NodeId) -> Option<(&str, f32)> {
    match &self.get(node)?.kind {
        NodeKind::Leaf(LeafKind::Text { content, font_size, .. }) => Some((content, *font_size)),
        _ => None,
    }
}
```

`_ => None` 已经是兜底分支，新变体不会导致编译错误。**本步骤无需改动代码**——但要验证。

- [ ] **Step 2: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -30
```

期望：layout.rs 无报错。Task 3 + Task 4 + Task 5 结束后应该 `cargo check` 通过（但还没加 shadow、position 等字段）。

- [ ] **Step 3: 如果有未使用 import 警告**

如果 `cargo check` 报 `unused import` 或 `unused variable`（因为新变体的字段），加 `#[allow(dead_code)]` 标注到 LeafKind 定义上方。比如：

```rust
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]  // 新变体暂未使用，阶段 C 填充
pub enum LeafKind {
    ...
}
```

- [ ] **Step 4: 再次验证编译**

```bash
cargo check -p gui 2>&1 | tail -10
```

期望：零错误、零 warning。

---

## Task 6: BoxStyle 加 Position、Transform 字段

**Files:**
- Modify: `gui/src/widget/layout/types.rs`（BoxStyle 定义 + Default impl）

**背景：** 加两个新字段：`position: Position`（Flow/Absolute 枚举）和 `transform: Option<Transform>`。字段存在即可，`arrange.rs` 不处理 Absolute 的实际定位逻辑（阶段 C）。

- [ ] **Step 1: 在 types.rs 添加 Position 和 Transform 定义**

在 `Edges` 定义之前（大约 `types.rs:50` 之前），插入：

```rust
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

```

- [ ] **Step 2: 给 BoxStyle 加 position 和 transform 字段**

找到 `types.rs` 里的 `BoxStyle` 结构体（现在的 `// ── 溢出 ──` 之后），在 `overflow: Overflow,` 这一行之后添加：

```rust
    // ── 定位（新增）──
    pub position: Position,
    pub transform: Option<Transform>,
```

- [ ] **Step 3: 更新 BoxStyle 的 Default impl**

在 `impl Default for BoxStyle` 里，在 `overflow: Overflow::Visible,` 之后添加：

```rust
            position: Position::Flow,
            transform: None,
```

- [ ] **Step 4: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：零错误、零 warning。如果有 `BoxStyle { ... }` 字面量构造没用 `..default()` 的地方会报"missing fields"，但 grep 确认过 `button.rs` 和 `slider.rs` 全部使用 `..BoxStyle::default()` 结尾，不会破。

---

## Task 7: BoxStyle 加 hittable、gestures 字段

**Files:**
- Modify: `gui/src/widget/layout/types.rs`

**背景：** hittable 用 `Option<bool>` 三态设计（None = 沿用 decoration 判据），gestures 用 `Vec<Gesture>` 声明式列表。

- [ ] **Step 1: 给 BoxStyle 加 hittable 和 gestures**

在 BoxStyle 的最后（`transform` 字段之后）添加：

```rust
    // ── 交互（新增）──
    /// 是否参与命中测试。None 表示沿用 "有 Decoration 则可命中" 的旧判据。
    pub hittable: Option<bool>,
    /// 命中后注册哪些手势识别器。
    pub gestures: Vec<crate::gesture::Gesture>,
```

- [ ] **Step 2: 更新 BoxStyle 的 Default impl**

在 `transform: None,` 之后添加：

```rust
            hittable: None,
            gestures: Vec::new(),
```

- [ ] **Step 3: 移除 BoxStyle 的 Copy（如有）和调整 Clone**

`Vec<Gesture>` 不是 Copy。检查 BoxStyle 的 derive 是不是含 `Copy`：

```bash
grep -n "derive.*BoxStyle\|#\[derive" gui/src/widget/layout/types.rs | head -5
```

BoxStyle 当前 derive 是 `#[derive(Debug, Clone, PartialEq)]`——没有 Copy。好。`Clone` 仍然有效（`Vec<Gesture>` 支持 Clone），`PartialEq` 也有效（Vec<Gesture> 支持 PartialEq，Gesture 派生了 Eq）。

- [ ] **Step 4: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：零错误、零 warning。

---

## Task 8: Decoration 加 shadow 字段 + 更新所有字面量

**Files:**
- Modify: `gui/src/widget/layout/types.rs`（Decoration 定义）
- Modify: `gui/src/widget/atoms/button.rs:43-47`
- Modify: `gui/src/widget/atoms/slider.rs:84-88, 98-102`

**背景：** 三处 Decoration 字面量（button 1 处 + slider 2 处）需要统一加 `shadow: None`。Decoration 没有 `Default` impl，只能全部字段列出。

- [ ] **Step 1: 更新 Decoration 定义加 shadow**

找到 `types.rs` 里的 Decoration：

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Decoration {
    pub background: Option<Color>,
    pub border: Option<Border>,
    pub radius: [f32; 4],
}
```

改为：

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Decoration {
    pub background: Option<Color>,
    pub border: Option<Border>,
    pub radius: [f32; 4],
    pub shadow: Option<crate::renderer::Shadow>,
}
```

同时更新 Decoration 上面的 doc comment 中的"shadow 暂不支持"文字（如果有，删除或改为"shadow 已支持"）。

- [ ] **Step 2: 验证编译会报三处 missing field**

```bash
cargo check -p gui 2>&1 | tail -30
```

期望：看到 button.rs 和 slider.rs 报 `missing field shadow`。

- [ ] **Step 3: 修复 button.rs**

找到 `gui/src/widget/atoms/button.rs:43-47`：

```rust
            decoration: Some(Decoration {
                background: Some(bg_color),
                border: Some(Border { width: 1.0, color: border_color }),
                radius: [4.0; 4],
            }),
```

改为：

```rust
            decoration: Some(Decoration {
                background: Some(bg_color),
                border: Some(Border { width: 1.0, color: border_color }),
                radius: [4.0; 4],
                shadow: None,
            }),
```

- [ ] **Step 4: 修复 slider.rs 第一处（track）**

找到 `gui/src/widget/atoms/slider.rs:84-88`：

```rust
                    decoration: Some(Decoration {
                        background: Some(track_color),
                        border: None,
                        radius: [track_radius; 4],
                    }),
```

改为：

```rust
                    decoration: Some(Decoration {
                        background: Some(track_color),
                        border: None,
                        radius: [track_radius; 4],
                        shadow: None,
                    }),
```

- [ ] **Step 5: 修复 slider.rs 第二处（fill）**

找到 `gui/src/widget/atoms/slider.rs:98-102`：

```rust
                            decoration: Some(Decoration {
                                background: Some(fill_color),
                                border: None,
                                radius: [track_radius; 4],
                            }),
```

改为：

```rust
                            decoration: Some(Decoration {
                                background: Some(fill_color),
                                border: None,
                                radius: [track_radius; 4],
                                shadow: None,
                            }),
```

- [ ] **Step 6: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：零错误、零 warning。

---

## Task 9: paint.rs 接通 Decoration.shadow 到 RectStyle

**Files:**
- Modify: `gui/src/panel/tree/paint.rs:14-21`

**背景：** 这是本阶段**唯一的 paint 行为改动**。仅作用于 Container 的 decoration 通路，不影响新 LeafKind 变体的 fallthrough 策略。

- [ ] **Step 1: 修改 paint_node 里的 draw_rect 调用**

找到 `gui/src/panel/tree/paint.rs:14-21`：

```rust
    if let Some(dec) = &node.decoration {
        renderer.draw_rect(rect, &RectStyle {
            color: dec.background.unwrap_or(Color::TRANSPARENT),
            border: dec.border,
            radius: dec.radius,
            shadow: None,
        });
    }
```

改为：

```rust
    if let Some(dec) = &node.decoration {
        renderer.draw_rect(rect, &RectStyle {
            color: dec.background.unwrap_or(Color::TRANSPARENT),
            border: dec.border,
            radius: dec.radius,
            shadow: dec.shadow,
        });
    }
```

- [ ] **Step 2: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：零错误、零 warning。

---

## Task 10: 补齐 TextInputProps.build()

**Files:**
- Modify: `gui/src/widget/atoms/text_input.rs:23-25`

**背景：** 视觉：带边框的方框，上方小灰字显示 label，下方正常字显示 value。参考 Button 的两层结构（Container decoration + Text 子节点）。

- [ ] **Step 1: 替换 build() 方法**

找到 `gui/src/widget/atoms/text_input.rs:23-25`：

```rust
    fn build(&self, _id: &str) -> WidgetBuild {
        todo!("TextInputProps::build")
    }
```

替换为：

```rust
    fn build(&self, id: &str) -> WidgetBuild {
        use crate::widget::desc::Desc;
        use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Edges, LeafKind};
        use crate::renderer::{Border, Color};

        // shadcn zinc 色系
        let bg = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let border_color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };   // zinc-200
        let label_color = Color { r: 0.443, g: 0.443, b: 0.478, a: 1.0 };    // zinc-500
        let value_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };    // zinc-900

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
                // label
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::label")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: self.label.to_string(),
                        font_size: 11.0,
                        color: label_color,
                    },
                },
                // value
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::value")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
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

- [ ] **Step 2: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：零错误、零 warning。注意：`Cow` 已经在文件顶部 import 了，不需要重新 import。

---

## Task 11: 补齐 ToggleProps.build()

**Files:**
- Modify: `gui/src/widget/atoms/toggle.rs:23-25`

**背景：** 视觉：左边圆角轨道 + 圆点（用 `justify_content: Start/End` 控制位置避免 Absolute 依赖），右边 label。

- [ ] **Step 1: 替换 build() 方法**

找到 `gui/src/widget/atoms/toggle.rs:23-25` 的 `todo!()`，替换为：

```rust
    fn build(&self, id: &str) -> WidgetBuild {
        use crate::widget::desc::Desc;
        use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Align, Justify, Edges, LeafKind};
        use crate::renderer::Color;

        // shadcn 色系
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
                // 开关轨道 (32 x 18)
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
                        // 滑块圆点 (14 x 14)
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
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
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

- [ ] **Step 2: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：零错误、零 warning。

---

## Task 12: 补齐 DropdownProps.build()

**Files:**
- Modify: `gui/src/widget/atoms/dropdown.rs:24-26`

**背景：** 视觉：类似 Button，显示选中的选项 + 右侧的 `▾`（Unicode 字符，不依赖 Icon 图元）。

- [ ] **Step 1: 替换 build() 方法**

找到 `gui/src/widget/atoms/dropdown.rs:24-26` 的 `todo!()`，替换为：

```rust
    fn build(&self, id: &str) -> WidgetBuild {
        use crate::widget::desc::Desc;
        use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Align, Edges, LeafKind};
        use crate::renderer::{Border, Color};

        // shadcn zinc 色系
        let bg = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let border_color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };  // zinc-200
        let text_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };    // zinc-900
        let arrow_color = Color { r: 0.443, g: 0.443, b: 0.478, a: 1.0 };   // zinc-500

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
                // 显示选中项
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::selected")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: selected_text,
                        font_size: 12.0,
                        color: text_color,
                    },
                },
                // 弹性占位
                Desc::Container {
                    id: Cow::Owned(format!("{id}::spacer")),
                    style: BoxStyle {
                        flex_grow: 1.0,
                        ..BoxStyle::default()
                    },
                    decoration: None,
                    children: vec![],
                },
                // 箭头
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::arrow")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
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

- [ ] **Step 2: 验证编译**

```bash
cargo check -p gui 2>&1 | tail -20
```

期望：零错误、零 warning。

---

## Task 13: clippy 和全量构建验证

**Files:** 无需修改

**背景：** 到这里所有改动完成。跑 clippy 确认代码质量，跑全量 build 确保整个工作空间都能编译。

- [ ] **Step 1: clippy**

```bash
cargo clippy -p gui -- -D warnings 2>&1 | tail -30
```

期望：无 warning 被升级为 error。如果有 warning 显示，根据提示修复（通常是 unused import、dead_code 等）。

- [ ] **Step 2: 全量 build**

```bash
cargo build -p nodeimg-app --release 2>&1 | tail -20
```

期望：`Finished release [optimized]` 成功编译。

- [ ] **Step 3: grep 验证 todo!() 清零**

```bash
grep -r "todo!" gui/src/widget/atoms/ 2>&1
```

期望：空输出（无匹配）。

- [ ] **Step 4: grep 验证 LeafKind 10 变体**

```bash
grep -E "^\s+(Text|Image|Icon|Circle|Line|Curve|Path|Grid|Connection|CustomPaint)" gui/src/widget/layout/types.rs
```

期望：看到 10 个变体的引入行。

- [ ] **Step 5: grep 验证 BoxStyle 新字段**

```bash
grep -E "pub (position|transform|hittable|gestures):" gui/src/widget/layout/types.rs
```

期望：看到 4 行。

- [ ] **Step 6: grep 验证 Decoration.shadow**

```bash
grep "pub shadow:" gui/src/widget/layout/types.rs
```

期望：看到 `pub shadow: Option<crate::renderer::Shadow>,`。

- [ ] **Step 7: grep 验证 Gesture 枚举**

```bash
cat gui/src/gesture/kind.rs | grep -E "^\s+(Tap|DoubleTap|Drag|LongPress)"
```

期望：看到 4 个变体。

---

## Task 14: demo 手测

**Files:** 无需修改

**背景：** 纯粹的人眼测试。启动 demo 程序，确认视觉和交互与改动前完全一致。不需要测新功能（新字段都没生效），只需要确认没有 regression。

- [ ] **Step 1: 启动 demo**

```bash
cargo run -p nodeimg-app --release
```

- [ ] **Step 2: 交互清单**

依次验证：

1. 窗口打开，看到 demo 面板在 (100, 100) 位置，大小 300x200
2. 面板内能看到 Button A、Button B、Slider "Radius"
3. 左键拖拽面板空白区域，面板能移动
4. 拖到面板右下角（6px 内），光标变成 resize 图标，可拖动改大小
5. 点击 Button A 和 Button B，终端打印 `Action::Click`
6. 拖动 Slider 的填充条，终端打印 `Action::DragMove`，控制台可能看到值变化日志
7. 中键拖动背景，相机平移（背景点阵跟随）
8. 滚轮，相机缩放

**全部与改动前一致 → 通过。**

- [ ] **Step 3: 关闭 demo**

`Ctrl+C` 或关闭窗口。

---

## Task 15: 提交两个 commit

**Files:** git 工作树

**背景：** 文档和代码两个独立 commit。先文档，后代码。

- [ ] **Step 1: 确认 git 状态**

```bash
git status
```

期望：
- Changes not staged: `docs/target/2.0.0-gui.md`
- Untracked: `docs/target/2.14.0-concepts.md`
- Changes not staged: 多个 gui/src/ 下的 rs 文件

如果状态不符合，停下检查。

- [ ] **Step 2: commit 文档**

```bash
git add docs/target/2.0.0-gui.md docs/target/2.14.0-concepts.md
git commit -m "$(cat <<'EOF'
docs: 对齐新架构 —— 删除图元控件层级，统一四级分层

本 session 多轮讨论后确定的控件层级调整：

- 新建 docs/target/2.14.0-concepts.md，定义四级控件层次：
  图元 → 原子控件 → 组合控件 → 特定框架控件
  容器作为贯穿所有层级的结构骨架

- 2.0.0-gui.md 对齐新层级：
  * §0 模块总览：删除"面板系统"子图，统一为 Context 一棵树
  * §2 事件系统：加入手势竞技场 + hit chain 模型
  * §4 UIState：移除面板几何状态（挂到 widget 节点）
  * §5 控件树：大改示意图和文件结构，树节点 Leaf 直接使用不套图元控件
  * §6 控件库：精简为索引表，指向 concepts.md
  * §7 面板：改为特定框架控件描述

- 消除图元控件层级（Primitive Widget）：
  原子控件直接组合 Container + Leaf，不套薄包装层。符合业界做法
  （Flutter/Radix/Material 都没有强制图元包装）。

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 3: 验证文档 commit 成功**

```bash
git log --oneline -3
```

期望：最上一行是刚才的 docs commit。

- [ ] **Step 4: commit 代码**

```bash
git add gui/src/gesture/kind.rs \
        gui/src/gesture/mod.rs \
        gui/src/renderer/style.rs \
        gui/src/widget/layout/types.rs \
        gui/src/widget/atoms/button.rs \
        gui/src/widget/atoms/slider.rs \
        gui/src/widget/atoms/text_input.rs \
        gui/src/widget/atoms/toggle.rs \
        gui/src/widget/atoms/dropdown.rs \
        gui/src/panel/tree/paint.rs

git commit -m "$(cat <<'EOF'
feat(gui): 阶段 A 数据模型扩展 —— LeafKind/BoxStyle/Decoration 对齐文档

按照 spec 2026-04-10-phase-a-alignment-design.md 执行的骨架优先扩展：

数据模型扩展：
- LeafKind 从 1 变体扩展到 10 变体（Text/Image/Icon/Circle/Line/Curve/
  Path/Grid/Connection/CustomPaint）
- 引入 TextureHandle 不透明句柄，避免 widget 层依赖 wgpu
- 引入 CustomPainter trait + CustomPaintFn 包装，让 LeafKind 能派生
  Debug/Clone/PartialEq
- BoxStyle 加 position (Flow/Absolute)、transform (仿射)、hittable
  (三态)、gestures (Vec<Gesture>) 四个字段
- Decoration 加 shadow: Option<Shadow> 字段

新建 Gesture 枚举：
- gui/src/gesture/kind.rs 定义 Tap/DoubleTap/Drag/LongPress
- 放在 gesture/ 模块下保持层级清洁

补齐三个 todo!() 控件：
- TextInputProps: 带边框方框 + label + value 文字
- ToggleProps: 圆角轨道 + 圆点（用 justify_content 而非 Absolute）
- DropdownProps: Button 风格 + 选中项 + ▾ 箭头

paint 行为改动：
- paint.rs 把 dec.shadow 传给 RectStyle.shadow（之前硬编码 None）
- 这是本阶段唯一的 paint 改动，仅作用于 Container decoration 通路

骨架优先原则：
- 新 LeafKind 变体在 paint/layout 中 fallthrough，不实际渲染
- Position::Absolute 字段存在但 arrange.rs 仍按 Flow 处理
- Transform 字段存在但 paint 不应用矩阵
- hittable=None 沿用旧的 decoration.is_some() 判据
- gestures 字段存在但 hit test/arena 暂不消费

demo 行为与实施前完全一致：Button、Slider、面板拖拽、面板 resize 均未
受影响。后续阶段 B（目录重组）和阶段 C（消除 PanelLayer）将独立推进。

Refs:
- docs/target/2.14.0-concepts.md
- docs/target/2.0.0-gui.md
- docs/superpowers/specs/2026-04-10-phase-a-alignment-design.md

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 5: 验证代码 commit 成功**

```bash
git log --oneline -3
git status
```

期望：
- 最上面一行是刚才的 feat commit
- `git status` 报告工作树干净（nothing to commit）

---

## 总结

15 个 Task 完成后：

- [ ] LeafKind 有 10 个变体，类型体系完整
- [ ] BoxStyle 有 4 个新字段，默认值合理
- [ ] Decoration 有 shadow 字段，已接通 paint 通路
- [ ] Gesture 枚举定义在 `gesture/kind.rs`
- [ ] 3 个 todo!() 控件的 build() 全部实现
- [ ] `cargo check -p gui` 零错零 warning
- [ ] `cargo clippy -p gui -- -D warnings` 通过
- [ ] `cargo build -p nodeimg-app --release` 成功
- [ ] demo 启动正常，视觉与交互与改动前一致
- [ ] 两个独立 commit 入库：docs + feat
- [ ] 文档里描述的字段和变体在代码中真实存在

**不在本计划范围内**：Absolute 定位的 arrange 实现、Transform 矩阵 paint、hittable/gestures 的实际消费、新 LeafKind 变体的实际渲染、PanelLayer 消除、目录重组。这些属于后续阶段 B 和阶段 C。
