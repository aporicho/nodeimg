# 阶段 C.4 设计：PanelProps widget

## 背景

阶段 C.1+C.2+C.3（commit `a0efe8d`）完成了"声明式命中 + 手势基础设施"：`tree/layout/arrange.rs` 支持 `Position::Absolute`，`tree/hit.rs` 返回 `HitChain`，`gesture/resize.rs` 实现了 `ResizeRecognizer`。但这些基础设施**还没被任何 widget 消费**——`widget/atoms/` 下的 5 个原子控件都不声明 gestures，demo.rs 仍走旧 panel 系统（`panel/frame.rs` + `panel/layer.rs` + `panel/renderer.rs`）。

阶段 C 大纲把面板系统的声明式改造拆成 C.4（Panel widget）→ C.5（Canvas 分支）→ C.6（Context 重构）→ C.7（demo 重写）→ C.8（删除旧 panel）。本 spec **只覆盖 C.4**。C.5~C.8 依赖本 spec 提供的 PanelProps，但设计和实施独立。

本 spec 的核心任务：新建 `widget/frameworks/panel.rs`，定义 `PanelProps`——第一个"复合 widget"（不是 atom，有子结构 + gestures 声明），作为后续把面板纳入声明式树的起点。

## 目标与非目标

### 目标

1. **新建 `widget/frameworks/` 目录**：作为"复合 widget / 框架控件"的落点，区别于 `widget/atoms/`（原子控件）。内含一个 `panel.rs` 文件。
2. **定义 `PanelProps`**：字段为 `id, title, x, y, w, h, content: Vec<Desc>`。实现 `WidgetProps` trait 的全部 6 个方法（`widget_type` / `as_any` / `clone_box` / `props_eq` / `debug_fmt` / `build`）。
3. **`build()` 生成声明式子树**：外框（`Absolute` + 全尺寸 + `gestures: [Resize]`）→ [标题栏（固定高度 32，`gestures: [Drag]`） + 内容区（`flex_grow: 1`，透明）]。
4. **Controlled 状态模型**：Panel 本身不持任何状态。`x/y/w/h` 每帧由父组件通过 props 传入。拖拽/resize 产生的 Action 由父组件处理并更新自己的状态。本 spec 不修改 reconcile 语义。
5. **13 个单元测试覆盖**：8 个结构测试 + 3 个 `props_eq` 测试 + 2 个与 C.1/C.2 联动的集成测试（通过 `hit_test` 验证命中路径到达正确的 gesture 节点）。
6. **零 demo 改动**：`app/src/demo.rs` 不修改，旧 `panel/` 系统不动。C.4 的交付是一个库 + 一组单元测试，demo 里不实例化 `PanelProps`。

### 非目标

- **不让 demo 使用 PanelProps** — C.7 才做。demo 继续用 `PanelLayer` + `PanelFrame` + `PanelRenderer`。
- **不实现 Uncontrolled / stateful Panel** — 运行时状态保留语义交给 Controlled 绕开，不动 reconcile。如果未来需要 Uncontrolled，再单开 spec 讨论。
- **不声明式地创建 gesture arena** — C.4 只声明 `gestures: [Resize]` / `[Drag]`，不改 `gesture/arena.rs`，也不改 `demo.rs` 里的 arena 创建逻辑。真正"按 hit chain 自动构造 recognizer"是 C.6 / C.7。
- **不加 close 按钮、折叠、自定义样式、min/max 约束** — 字段保持最小。未来通过追加 `PanelProps` 字段扩展。
- **不加主题系统 / 样式配置** — 装饰使用模块内硬编码常量。C.7 再统一重做视觉。
- **不改 `widget/atoms/`** — 原子控件保持原状。
- **不改 `widget/action.rs`** — 已在 C.3 加了 Resize 变体，Drag 变体早就存在，Panel 只是复用它们。
- **不修复已知的反向依赖**（`tree/` → `widget::props`）——和 C.1~C.3 一样，留给未来的专项清理。

## 指导原则

1. **Controlled 优先，状态留给父组件** — Panel 是"纯画匠"：拿到 props 就画，不记任何东西。拖拽事件由父组件接收并更新 props。这让 C.4 的测试简单（只测 `build()` 的输出）、不需要改 reconcile、和 React/Elm 风格一致。
2. **最小功能集** — 只做"有外框 + 有标题栏 + 可拖 + 可 resize + 有内容槽"这个最小闭环。close/折叠/约束/主题/样式都是未来迭代点，不进本 spec。
3. **文件结构 = 软件结构** — 复合 widget 单独放 `widget/frameworks/`，和 `atoms/` 平级。即便现在只有 Panel 一个，目录也要先建起来（后续 C.5 会加 `node_card.rs`）。
4. **零 demo 破坏** — 本阶段不碰 demo。库增量交付，demo 行为与 C.3 结束时完全一致。
5. **测试优先于手测** — 结构、`props_eq`、和命中链的联动全部用单元测试覆盖。手测（视觉确认）留给 C.7。

## 决策记录

### 决策 1 · Controlled（props 权威）vs Uncontrolled（widget 持状态）

**决定**：Panel 采用 **Controlled 模型**。`PanelProps` 的 `x/y/w/h` 字段是每帧的"当前值"，不是"初始值"。父组件拿到拖拽/resize Action 后更新自己的状态，下一帧传入新的 props。`PanelNode.rect` 按常规 reconcile 流程从 props 重新计算。

**替代方案**：
- **Uncontrolled**：`PanelProps` 字段改名为 `initial_x/initial_y/initial_w/initial_h`，只在 mount 时生效。Panel 挂载后，`PanelNode.rect` 的后续修改必须绕过 reconcile。需要给 reconcile 加一条"对 Panel 类型 widget 的 rect 跳过 props 覆盖"的特殊规则。

**理由**：
- Uncontrolled 需要给 reconcile 加"运行时字段保留"机制，这是大纲里列的"核心难点"，并且会在 props_eq 语义（initial_x 变了算不算变？）、父组件重设位置（比如"重置面板位置"按钮）等场景上引入微妙的 edge case。
- Controlled 的测试极其简单：给 props，验证 `build()` 输出。不需要模拟 mount + drag 交互。
- Controlled 和 React/Elm 的声明式模型一致，心智负担低。Uncontrolled 更像 OS 窗口系统或 SwiftUI `@State`，但那套需要额外的状态管理抽象。
- C.4 是独立交付的库，没有 demo 配合。Controlled 能纯库测；Uncontrolled 需要测 reconcile 规则，scope 膨胀。
- "父组件得记住 panel_x/y"这个缺点在 C.7 demo 重写时自然解决——到时候 demo 本来就要重构状态管理。现在不用在 C.4 提前承担那份设计负担。

### 决策 2 · `build()` 的树形状

**决定**：`PanelProps::build()` 返回的 `WidgetBuild` 代表外框本身：

```
WidgetBuild {
    style: BoxStyle {
        position: Absolute { x, y },
        width:  Fixed(w),
        height: Fixed(h),
        direction: Column,
        gestures: [Resize],
        ...
    },
    decoration: Some(Decoration { frame_bg, frame_border, radius, ... }),
    children: vec![
        titlebar_container,  // Desc::Container，id = "{widget_id}::titlebar"
        content_container,   // Desc::Container，id = "{widget_id}::content"
    ],
}
```

其中 titlebar_container 是：

```
Desc::Container {
    id: "{widget_id}::titlebar",
    style: {
        height: Fixed(TITLE_BAR_HEIGHT),  // 32
        padding: Edges::all(8),
        gestures: [Drag],
        ...
    },
    decoration: Some({ titlebar_bg }),
    children: vec![
        Desc::Leaf {
            id: "{widget_id}::title",
            kind: LeafKind::Text { content: panel.title, font_size, color },
        }
    ],
}
```

content_container 是：

```
Desc::Container {
    id: "{widget_id}::content",
    style: { flex_grow: 1.0, ... },
    decoration: None,  // 透明，事件穿透到子控件
    children: panel.content,  // 用户传入的 Vec<Desc>
}
```

**替代方案**：
- 把标题栏的 id 直接设为 `widget_id`，和外框共享身份。Drag Action 的 id 直接就是面板 id。
- 把整个 Panel 做成只有一层 Container（没有 titlebar 子容器），让外框同时有 `[Drag, Resize]`。

**理由**：
- 让每个子节点有独立的 id（`::titlebar` / `::title` / `::content` 后缀）便于调试、日志、和后续 C.6 的 arena 查找。id 共享会让 hit chain 里出现两个同 id 的节点，debug 时难区分。
- "Drag Action 产出的 id 不是面板 id" 这个问题由 C.6 的 arena 创建逻辑解决——arena 在创建 DragRecognizer 时可以按 hit chain 向上查找最近的 `widget_type() == "Panel"` 节点，用那个节点的 id 作为 target_id。本 spec 不做这一步，只保证树结构正确。
- 不把 Drag 放在外框、只让标题栏有 Drag，避免用户在内容区拖动也能移动面板（这是常见的 UX 陷阱，不符合 OS 窗口约定）。
- 三层结构（外框 → 标题栏 + 内容区）是 OS 窗口的标准形态，易于理解和扩展（未来加 close 按钮就放在标题栏子节点里）。

### 决策 3 · 字段最小化

**决定**：`PanelProps` 只有 7 个字段：`id, title, x, y, w, h, content`。不加：`min_w, min_h, resizable, draggable, close_button, title_height, bg_color, border_color, collapsed, z_order`。

**替代方案**：
- 加完整字段集（以上全部），一次定型，未来不再扩展。
- 加可选字段（`Option<...>`）+ `Default` 实现，调用方按需覆盖。

**理由**：
- YAGNI——C.4 的唯一用户是未来的 demo（C.7），demo 需要什么再加什么。现在猜测的字段很可能猜错。
- 追加字段对 WidgetProps trait 不破坏兼容——只要 `props_eq` 继续工作，现有代码不受影响。
- 最小字段集让测试清单也最小（13 个测试刚好够用），不会在未使用的字段上浪费测试代码。
- `content: Vec<Desc>` 已经足够覆盖"多个子控件"的需求，不需要额外的 `header_extra` / `footer` 之类的槽位。

### 决策 4 · 装饰使用硬编码占位常量

**决定**：外框和标题栏的装饰（`background`、`border`、`radius` 等）在 `panel.rs` 模块顶部以 `const` / `fn` 的形式硬编码，不暴露为 `PanelProps` 字段。

**替代方案**：
- 把装饰作为 `Option<Decoration>` 字段暴露给调用方。
- 走主题系统查表（主题系统不存在）。

**理由**：
- C.7 会引入主题系统（大纲列表的 M6），届时所有 widget 的装饰都要从主题表查。现在把装饰做成 props 字段会在 C.7 时全部拆掉，是一次浪费。
- 硬编码占位让 `props_eq` 实现简单（不需要比较 `Decoration`，它不实现 `PartialEq`）。
- 占位视觉可以很朴素（单色背景 + 1px 边框 + 4px 圆角），C.7 会重做。
- 如果调用方现在就想自定义样式，那是 C.7 的需求而非 C.4 的需求。

### 决策 5 · Controlled 模型下的 Action 路由由父组件自行处理

**决定**：`PanelProps` 的 `build()` 只负责声明 `gestures: [Drag]` 和 `[Resize]`，不负责解释 Action 该送到哪里。Drag/Resize Action 的 `id` 字段是触发节点（标题栏 / 外框）的节点 id（带 `::titlebar` 后缀），父组件需要自己把 Action 翻译成"哪个 panel 被操作"。

**替代方案**：
- 在 `widget/action.rs` 里新增 `Action::PanelDragMove` / `Action::PanelResizeMove` 变体，专门给 Panel 用，payload 直接是 panel_id。
- 让 `PanelProps.build()` 返回一个"Action 映射函数"，gesture arena 在产出 Action 前走一次该函数做 id 替换。

**理由**：
- 新增 Action 变体会让 `Action` enum 膨胀（Panel 是第一个复合 widget，以后每个复合 widget 都要加专属变体？不可持续）。
- Action 映射函数引入新机制（closure 存到哪？gesture arena 如何获取？），和现有的声明式模型不兼容。
- 父组件的"翻译"逻辑可以很简单：Drag Action 的 id 以 `::titlebar` 结尾 → 去掉后缀得到 panel_id；Resize Action 的 id 没有后缀（因为外框就是 widget 本身）→ 直接就是 panel_id。
- 这个翻译逻辑属于 demo 层面的胶水代码（或未来 C.6 的 arena 创建胶水），不该污染 widget 层。
- 本 spec 的测试只验证"gestures 在正确的节点上"，不验证"Action 路由到正确的 panel_id"——那是 demo/arena 的职责，C.6/C.7 才做。

### 决策 6 · 内容区不加 decoration

**决定**：content_container 的 `decoration: None`。内容区完全透明，不参与命中测试（混合判据：`gestures` 空 + `decoration` None → 不可命中）。

**替代方案**：
- 给内容区加一个透明 decoration（`background: Color::TRANSPARENT`）或占位 decoration。
- 给内容区设 `hittable: Some(true)`，强制可命中。

**理由**：
- 不可命中让事件"穿透"到内部的子控件（按钮、滑块等），这是内容区应有的行为——点内容区就是点内容，不该被 Panel 捕获。
- 加 decoration 会让内容区参与命中判据，打破"透明穿透"语义。
- `hittable: Some(true)` 反其道而行——把内容区变成"捕获层"，子控件的点击会被 Panel 拦截。这是反向需求，不是 C.4 要做的事。

### 决策 7 · `widget/frameworks/` 目录 vs 平铺

**决定**：新建 `widget/frameworks/` 子目录，里面放 `mod.rs` 和 `panel.rs`。在 `widget/mod.rs` 加 `pub mod frameworks;`。

**替代方案**：
- 把 `panel.rs` 直接放在 `widget/` 下（平铺），和 `widget/atoms/` 同级。
- 把 `panel.rs` 放进 `widget/atoms/panel.rs`，和其他原子同目录。

**理由**：
- Panel 不是原子——它有子结构（标题栏 + 内容区）、声明 gestures、被设计为"框架控件"。放进 `atoms/` 语义错误，会让未来的维护者困惑"什么是 atom，什么不是"。
- 平铺让 `widget/` 根目录开始杂乱——将来 C.5 会加 `node_card.rs`，M5 可能加 `dialog.rs` / `menu.rs`。目录能起到分类作用。
- `widget/frameworks/` 这个命名来自大纲文档的讨论，保持一致有助于未来读者的理解。
- 即便现在只有 Panel 一个，目录也要先建起来——文件结构早定好，后续的扩展不需要再做结构调整。

## 架构

### 本 spec 变更的文件

| 路径 | 改动类型 | 说明 |
|------|---------|------|
| `gui/src/widget/frameworks/mod.rs` | **新建** | `pub mod panel;` |
| `gui/src/widget/frameworks/panel.rs` | **新建** | `PanelProps` 定义 + `impl WidgetProps` + `#[cfg(test)] mod tests`（13 个测试） |
| `gui/src/widget/mod.rs` | 修改 | 加 `pub mod frameworks;` |

本 spec **零改动**的文件：

- `app/src/demo.rs`
- `gui/src/widget/atoms/*.rs`（5 个原子控件）
- `gui/src/widget/action.rs`（Drag / Resize 变体已在 C.3 就绪）
- `gui/src/widget/props.rs`（trait 定义不变）
- `gui/src/gesture/**`（C.3 刚建好的 gesture 系统不动）
- `gui/src/tree/**`（Tree / reconcile / hit_test / arrange 都不动）
- `gui/src/panel/**`（旧面板系统继续运行）

### PanelProps API

```rust
use std::borrow::Cow;
use crate::tree::Desc;

#[derive(Clone)]
pub struct PanelProps {
    pub id:      Cow<'static, str>,
    pub title:   Cow<'static, str>,
    pub x:       f32,
    pub y:       f32,
    pub w:       f32,
    pub h:       f32,
    pub content: Vec<Desc>,
}
```

**字段语义**：

- `id` — 身份标识。调用方通过 `Desc::Widget { id, props }` 传给 Tree，这个 id 就是 `build()` 收到的 `id: &str` 参数，也是外框（widget 节点）在 Tree 里的 NodeId。
- `title` — 标题栏显示的文字。
- `x, y` — 绝对坐标（像素）。每帧由父组件传入，代表"当前位置"。
- `w, h` — 尺寸（像素）。每帧由父组件传入，代表"当前大小"。
- `content` — 内容区的子控件列表。可以是按钮、滑块、任何 `Desc`（包括嵌套的 Widget）。

**为什么用 `Cow<'static, str>` 而不是 `String`**：和现有 `widget/atoms/button.rs` 的约定一致（见 `ButtonProps::label`），支持字面量零拷贝。

### WidgetProps 实现

```rust
impl WidgetProps for PanelProps {
    fn widget_type(&self) -> &'static str { "Panel" }

    fn as_any(&self) -> &dyn Any { self }

    fn clone_box(&self) -> Box<dyn WidgetProps> { Box::new(self.clone()) }

    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other.as_any()
            .downcast_ref::<Self>()
            .map_or(false, |o| {
                self.id == o.id
                    && self.title == o.title
                    && self.x == o.x && self.y == o.y
                    && self.w == o.w && self.h == o.h
                    && descs_eq(&self.content, &o.content)
            })
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { /* ... */ }

    fn build(&self, id: &str) -> WidgetBuild { /* 见下面 */ }
}
```

`descs_eq` 比较两个 `Vec<Desc>` 是否等价。由于 `Desc` 不实现 `PartialEq`（因为 `Desc::Widget` 持 `Box<dyn WidgetProps>`，没法泛型比），需要在 `panel.rs` 里自定义（可能只比较 `id` + `discriminant`，或直接用 `ptr::eq` 作浅比较）。具体实现策略在 plan 阶段决定——最简单的做法是**`props_eq` 不比较 content**，只要 id/title/x/y/w/h 相同就视为相等，content 变化靠 reconcile 层的 child diff 自动处理。这是业界惯例（React 也不深比较 children）。

### `build()` 实现草图

```rust
fn build(&self, id: &str) -> WidgetBuild {
    use crate::tree::Desc;
    use crate::tree::layout::{BoxStyle, Decoration, Size, Position, Direction, Edges, LeafKind};
    use crate::renderer::{Border, Color};
    use crate::gesture::Gesture;

    // 占位装饰常量（C.7 会接主题系统）
    let frame_bg     = Color { r: 0.133, g: 0.145, b: 0.196, a: 1.0 };  // base-900
    let frame_border = Color { r: 0.263, g: 0.278, b: 0.333, a: 1.0 };  // base-700
    let titlebar_bg  = Color { r: 0.180, g: 0.196, b: 0.255, a: 1.0 };  // base-800
    let title_color  = Color { r: 0.902, g: 0.910, b: 0.941, a: 1.0 };  // base-200
    let frame_radius = 6.0;
    let title_font   = 13.0;

    WidgetBuild {
        style: BoxStyle {
            position: Position::Absolute { x: self.x, y: self.y },
            width:  Size::Fixed(self.w),
            height: Size::Fixed(self.h),
            direction: Direction::Column,
            gestures: vec![Gesture::Resize],
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(frame_bg),
            border: Some(Border { width: 1.0, color: frame_border }),
            radius: [frame_radius; 4],
            shadow: None,
        }),
        children: vec![
            // 标题栏
            Desc::Container {
                id: Cow::Owned(format!("{id}::titlebar")),
                style: BoxStyle {
                    height: Size::Fixed(TITLE_BAR_HEIGHT),
                    padding: Edges::symmetric(6.0, 10.0),
                    gestures: vec![Gesture::Drag],
                    ..BoxStyle::default()
                },
                decoration: Some(Decoration {
                    background: Some(titlebar_bg),
                    border: None,
                    radius: [frame_radius, frame_radius, 0.0, 0.0],
                    shadow: None,
                }),
                children: vec![
                    Desc::Leaf {
                        id: Cow::Owned(format!("{id}::title")),
                        style: BoxStyle {
                            width:  Size::Auto,
                            height: Size::Auto,
                            ..BoxStyle::default()
                        },
                        kind: LeafKind::Text {
                            content: self.title.to_string(),
                            font_size: title_font,
                            color: title_color,
                        },
                    },
                ],
            },
            // 内容区
            Desc::Container {
                id: Cow::Owned(format!("{id}::content")),
                style: BoxStyle {
                    flex_grow: 1.0,
                    padding: Edges::all(8.0),
                    ..BoxStyle::default()
                },
                decoration: None,
                children: self.content.clone(),
            },
        ],
    }
}

const TITLE_BAR_HEIGHT: f32 = 32.0;
```

### 命中语义

```
外框 WidgetNode (id=panel_id)
  style.gestures  = [Resize]
  decoration      = Some(frame_bg)
  → hittable（gestures 非空）。外框边缘 6px 内命中 Resize，其他位置穿透到子节点。
  ├── 标题栏 Container (id="panel_id::titlebar")
  │     style.gestures = [Drag]
  │     decoration     = Some(titlebar_bg)
  │     → hittable。点击标题栏区域产生 Drag 手势。
  │     └── Text Leaf (id="panel_id::title")
  │           不命中（Leaf Text 的 gestures 空 + decoration None）。
  └── 内容区 Container (id="panel_id::content")
        style.gestures = []
        decoration     = None
        → 不可命中。事件穿透到用户放进去的子控件。
```

在 `hit_test(tree, panel_id, x, y)` 下，命中链从叶子到根的典型形态：

- 点击标题栏 → `[panel_id::titlebar, panel_id]`
- 点击内容区的按钮 → `[button::label, button, panel_id::content, panel_id]`（假设 button 的 label Text 不命中，button 自身有 decoration）
- 点击外框角（非标题栏区域） → `[panel_id]`（只有外框本身，Resize 手势触发）
- 点击外部 → `[]`（空链）

反向遍历（C.2 引入的 z-order 语义）：如果内容区有重叠子控件，后添加的先命中。本 spec 的测试覆盖不涉及这种边界，留给实际使用场景（C.7+）。

## 测试计划

### 测试组织

在 `gui/src/widget/frameworks/panel.rs` 末尾添加 `#[cfg(test)] mod tests`，参考 C.1/C.2/C.3 的模式（测试和被测代码同文件，helper 在模块顶部）。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Desc;
    use crate::tree::layout::{Position, Size, LeafKind};
    use crate::gesture::Gesture;

    fn sample_props() -> PanelProps {
        PanelProps {
            id: Cow::Borrowed("test"),
            title: Cow::Borrowed("Title"),
            x: 10.0,
            y: 20.0,
            w: 300.0,
            h: 200.0,
            content: vec![],
        }
    }

    // ... 13 个 #[test] ...
}
```

### 13 个单元测试

#### 结构测试（8 个）

1. **`widget_type_is_panel`** — `PanelProps::widget_type()` 返回 `"Panel"`。
2. **`build_outer_is_absolute`** — `build()` 的 `style.position == Position::Absolute { x: 10.0, y: 20.0 }`，`width == Size::Fixed(300.0)`，`height == Size::Fixed(200.0)`。
3. **`build_outer_has_resize_gesture`** — `style.gestures == vec![Gesture::Resize]`。
4. **`build_outer_has_frame_decoration`** — `decoration.is_some()`，`background.is_some()`，`border.is_some()`。
5. **`build_children_count_is_two`** — `children.len() == 2`（标题栏 + 内容区）。
6. **`build_titlebar_has_drag_gesture`** — `children[0]` 是 `Desc::Container`，其 `style.gestures == vec![Gesture::Drag]`，`style.height == Size::Fixed(TITLE_BAR_HEIGHT)`。
7. **`build_titlebar_contains_title_text`** — `children[0].children[0]` 是 `Desc::Leaf`，其 `kind` 是 `LeafKind::Text { content: "Title", .. }`。
8. **`build_content_flex_grow_holds_user_children`** — `children[1]` 是 `Desc::Container`，其 `style.flex_grow == 1.0`，`decoration.is_none()`，`children == panel.content`（同元素数）。

#### `props_eq` 测试（3 个）

9. **`props_eq_identical`** — 两个字段完全相同的 `PanelProps`，`props_eq` 返回 `true`。
10. **`props_eq_different_position`** — 只有 `x` 或 `y` 不同，`props_eq` 返回 `false`。
11. **`props_eq_different_title`** — 只有 `title` 不同，`props_eq` 返回 `false`。

#### 集成测试（2 个，与 C.1/C.2 联动）

12. **`hit_on_titlebar_reaches_drag_node`** — 构造一个 `Tree`，插入 `sample_props()` 对应的 widget 节点，跑一次 `layout()` 让 rect 计算出来，然后 `hit_test(tree, root, 50.0, 30.0)`（标题栏内的点），验证返回的 `HitChain` 非空，且链上有某个节点的 `style.gestures` 包含 `Gesture::Drag`。
13. **`hit_on_corner_reaches_resize_node`** — 同上，但 `hit_test` 的点选在外框右下角（比如 `(310, 220)`，正好是 `(x+w, y+h)`），验证 `HitChain` 非空且有节点的 `style.gestures` 包含 `Gesture::Resize`。

#### 测试注意事项

- 测试 12/13 需要把 `Desc` 描述转成实际的 `Tree`（通过 `reconcile` 或类似的入口），然后调用 `layout()` 才能有 rect。如果 `Tree::insert` 直接接受 `PanelNode` 则更简单，但那样就绕过了 Widget 节点的 `build()` 流程。plan 阶段决定具体实现路径。
- 测试 12/13 的 `hit_test` 调用需要传一个 `measure_text` 闭包，可以用 C.1 的 `no_measure` helper。
- `sample_props()` 里的 `content: vec![]` 让测试 8 的"用户子控件"是空 Vec，这足以验证"flex_grow=1 + children 跟着 props"。可以再加一个小测试用 `content: vec![Desc::Leaf { ... }]` 覆盖非空场景，如果觉得有必要。

## 风险与缓解

### 已知风险

1. **`Desc` 没有 `PartialEq`** — `props_eq` 无法深比较 `content`。**缓解**：`props_eq` 不比较 content（按 React 惯例），只比较 id/title/x/y/w/h。测试 9 覆盖这一点。

2. **`Tree` 构造 / reconcile 的入口 API 不熟悉** — 测试 12/13 需要走一遍 "Desc → Tree" 的流程，但当前代码里 reconcile 怎么被调用还没仔细看。**缓解**：plan 阶段先写一个最小的 Tree 构造 helper（可能 5-10 行），或者参考 `panel/renderer.rs::update()` 的写法。如果这一步被证明复杂，可以把测试 12/13 简化为"直接构造 PanelNode 树，跳过 Widget 展开"，但那样覆盖面略小。

3. **widget_type 返回值和 atoms 的命名不一致** — `ButtonProps::widget_type()` 返回 `"Button"`（首字母大写），其他 atoms 类似。本 spec 约定 `PanelProps::widget_type()` 返回 `"Panel"`，保持一致。注意不是 `"panel"` 或 `"PanelProps"`。

4. **`LeafKind::Text` 的 `font_size` 和 `color` 字段可能不是默认值** — 需要在 `build()` 里硬编码具体值。已在"`build()` 实现草图"里给了占位常量。

### 非风险（明确不担心）

- **demo 破坏** — 本 spec 不改 demo 任何文件。不会影响 demo 行为。
- **C.1/C.2/C.3 回归** — 本 spec 不改 tree/ 或 gesture/ 任何文件。这三个阶段的 30 个单元测试继续 PASS。
- **编译 / clippy 的 exhaustive match** — 本 spec 不给 `Action` 或 `Gesture` enum 加变体，不触发 demo 的 match 失配问题。

## 交付物

### 代码

- `gui/src/widget/frameworks/mod.rs`（新建，~1 行 `pub mod panel;`）
- `gui/src/widget/frameworks/panel.rs`（新建，~250-300 行：`PanelProps` 结构体 + `impl WidgetProps` + 13 个单元测试 + 占位装饰常量 + `TITLE_BAR_HEIGHT` 常量）
- `gui/src/widget/mod.rs`（修改，加一行 `pub mod frameworks;`）

### Commit

一次性 commit：`feat(gui): C.4 PanelProps widget（复合控件框架）`。Commit message 说明：
- 新建 `widget/frameworks/` 为复合控件落点
- `PanelProps` 采用 Controlled 模型，x/y/w/h 每帧由父组件传入
- `build()` 生成外框 + 标题栏（Drag）+ 内容区（透明穿透）
- 13 个单元测试覆盖结构、props_eq、和 C.1/C.2 命中联动
- 零 demo 改动

### 验证

- `cargo test -p gui --lib widget::frameworks::panel::tests` — 13 passed
- `cargo test -p gui` — 30 + 13 = 43 passed（保留 C.1/C.2/C.3 的 30 个）
- `cargo check --workspace` — 无错误
- `cargo clippy -p gui` — 无新增警告
- `cargo build -p app --release` — demo 编译通过，行为与 C.3 结束时完全一致

## 参考

- 大纲：`docs/superpowers/plans/2026-04-10-phase-c-panel-system-unification.md`
- 前置：`docs/superpowers/specs/2026-04-11-phase-c-layout-hit-gesture-design.md`（C.1+C.2+C.3）
- WidgetProps trait：`gui/src/widget/props.rs`
- 现有 atoms 参考：`gui/src/widget/atoms/button.rs`
- Desc 定义：`gui/src/tree/desc.rs`
- LeafKind：`gui/src/tree/layout/types.rs::LeafKind::Text`
