# 阶段 C.4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 `PanelProps` 作为第一个复合 widget（`widget/frameworks/panel.rs`），采用 Controlled 状态模型，`build()` 生成外框 + 标题栏 + 内容区的声明式子树，配 13 个单元测试，零 demo 改动。

**Architecture:** 新增 `widget/frameworks/` 目录作为复合控件落点。`PanelProps` 实现 `WidgetProps` trait，`build()` 方法返回 `WidgetBuild { style, decoration, children }`：外层 widget 节点（Absolute + Fixed size + `gestures: [Resize]` + 边框装饰）包含两个子容器（标题栏 `gestures: [Drag]` + 透明内容区 `flex_grow=1`）。状态管理（x/y/w/h 的更新）交给调用方，本阶段不修改 reconcile。

**Tech Stack:** Rust、`WidgetProps` trait、`Cow<'static, str>`、`#[cfg(test)] mod tests`、`cargo test -p gui` 验证。

**Spec:** `docs/superpowers/specs/2026-04-11-phase-c4-panel-widget-design.md`

---

## 前置知识

### commit 策略

本 plan 产出 **1 个 commit**：`feat(gui): C.4 PanelProps widget（复合控件框架）`。所有改动在同一次 commit 内。

### 测试基础设施

C.1+C.2+C.3 完成后，`gui` crate 有 30 个单元测试（6 arrange + 12 hit + 12 resize）。本 plan 在 `widget/frameworks/panel.rs` 末尾添加 13 个新测试，合计 43 个。

### 关键类型回顾

```rust
// widget/props.rs
pub struct WidgetBuild {
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub children: Vec<Desc>,
}

pub trait WidgetProps: 'static {
    fn widget_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn WidgetProps>;
    fn props_eq(&self, other: &dyn WidgetProps) -> bool;
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
    fn build(&self, id: &str) -> WidgetBuild;
}

// tree/desc.rs
pub enum Desc {
    Container { id: Cow<'static, str>, style: BoxStyle, decoration: Option<Decoration>, children: Vec<Desc> },
    Leaf { id: Cow<'static, str>, style: BoxStyle, kind: LeafKind },
    Widget { id: Cow<'static, str>, props: Box<dyn WidgetProps> },
}

// tree/diff.rs
pub fn reconcile(tree: &mut Tree, desc: Desc);

// tree/layout/layout.rs
pub fn layout<T: LayoutTree>(
    tree: &mut T,
    root: T::NodeId,
    available: Rect,
    measure_text: &mut dyn FnMut(&str, f32) -> (f32, f32),
);

// tree/hit.rs
pub fn hit_test(tree: &Tree, root: NodeId, x: f32, y: f32) -> HitChain;
```

### 已确认设计决策（来自 spec）

1. **Controlled 模型**：x/y/w/h 每帧由 props 传入，Panel 无内部状态
2. **字段最小化**：`id, title, x, y, w, h, content: Vec<Desc>`
3. **树形状**：外框（widget 节点） + 标题栏（独立 Container）+ 内容区（独立 Container）
4. **子节点 id 用后缀**：`::titlebar` / `::title` / `::content`
5. **内容区透明**：`decoration: None`，不命中，事件穿透
6. **装饰硬编码**：模块内 const，不暴露为 props
7. **`props_eq` 不深比较 content**：只比较 id/title/x/y/w/h

---

## File Structure

### 本 plan 创建或修改的文件

| 路径 | 改动 | 所属 Task |
|------|-----|----------|
| `gui/src/widget/frameworks/mod.rs` | **新建** | Task 1 |
| `gui/src/widget/frameworks/panel.rs` | **新建** | Task 1-3 |
| `gui/src/widget/mod.rs` | 修改：加 `pub mod frameworks;` | Task 1 |

### 本 plan 不修改的文件

- `app/src/demo.rs`
- `gui/src/widget/atoms/*.rs`
- `gui/src/widget/action.rs`、`props.rs`、`resize_edge.rs`
- `gui/src/tree/**`
- `gui/src/gesture/**`
- `gui/src/panel/**`
- `gui/src/renderer/**`

---

## Tasks

### Task 1: 创建目录结构 + PanelProps 骨架

**Files:**
- Create: `gui/src/widget/frameworks/mod.rs`
- Create: `gui/src/widget/frameworks/panel.rs`
- Modify: `gui/src/widget/mod.rs`

**Goal:** 让新目录就位，`PanelProps` 作为能编译的骨架存在（`build()` 返回占位值），`cargo check -p gui` 通过。

- [ ] **Step 1.1: 确认目录不存在**

Run: `ls gui/src/widget/frameworks 2>&1 | head -5`
Expected: `ls: gui/src/widget/frameworks: No such file or directory`

如果目录已存在，停止并询问 coordinator。

- [ ] **Step 1.2: 读取 widget/mod.rs 当前内容**

Read `gui/src/widget/mod.rs`。应该看到：
```rust
pub mod atoms;
pub mod props;

pub mod action;
pub mod resize_edge;
mod focus;
mod mapping;
pub mod state;
mod text_edit;
```

- [ ] **Step 1.3: 在 widget/mod.rs 加 `pub mod frameworks;`**

用 Edit 工具，在 `pub mod atoms;` 之后追加一行：

```rust
pub mod atoms;
pub mod frameworks;
pub mod props;

pub mod action;
pub mod resize_edge;
mod focus;
mod mapping;
pub mod state;
mod text_edit;
```

- [ ] **Step 1.4: 创建 widget/frameworks/mod.rs**

用 Write 工具创建 `gui/src/widget/frameworks/mod.rs`：

```rust
pub mod panel;
```

- [ ] **Step 1.5: 创建 widget/frameworks/panel.rs 骨架**

用 Write 工具创建 `gui/src/widget/frameworks/panel.rs`，先写一个能编译的最小骨架（`build()` 返回"空"的 WidgetBuild，不做实际工作）：

```rust
use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::tree::Desc;
use crate::tree::layout::BoxStyle;
use crate::widget::props::{WidgetBuild, WidgetProps};

/// 标题栏固定高度（像素）
pub const TITLE_BAR_HEIGHT: f32 = 32.0;

/// Panel widget 的 props。
///
/// Controlled 模型：x/y/w/h 每帧由调用方传入，widget 本身不持任何状态。
/// 拖拽/resize 手势产生的 Action 由调用方处理并更新 props。
#[derive(Clone)]
pub struct PanelProps {
    pub id: Cow<'static, str>,
    pub title: Cow<'static, str>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub content: Vec<Desc>,
}

impl fmt::Debug for PanelProps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PanelProps")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("x", &self.x)
            .field("y", &self.y)
            .field("w", &self.w)
            .field("h", &self.h)
            .field("content_len", &self.content.len())
            .finish()
    }
}

impl WidgetProps for PanelProps {
    fn widget_type(&self) -> &'static str {
        "Panel"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn WidgetProps> {
        Box::new(self.clone())
    }

    fn props_eq(&self, _other: &dyn WidgetProps) -> bool {
        // TODO(Task 3): 实际实现
        false
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn build(&self, _id: &str) -> WidgetBuild {
        // TODO(Task 3): 实际实现
        WidgetBuild {
            style: BoxStyle::default(),
            decoration: None,
            children: Vec::new(),
        }
    }
}
```

**注意**：
- `Debug` 手动实现是因为 `Vec<Desc>` 里的 `Desc::Widget` 持 `Box<dyn WidgetProps>`，`Desc` 没 `Debug`（或有但可能有坑），先手动写避免编译报错
- `widget_type` 返回 `"Panel"`（首字母大写，和 `ButtonProps::widget_type()` 返回 `"Button"` 一致）
- `props_eq` 和 `build()` 是占位，Task 3 才填充

- [ ] **Step 1.6: cargo check 确认骨架可编译**

Run: `cargo check -p gui 2>&1 | tail -15`
Expected: 无新增错误。可能有若干 `dead_code` 警告（占位实现未使用字段），这些是预期的。

如果有编译错误，根据错误信息调整（常见：import 路径、trait 方法签名、`Cow<'static, str>` vs `String` 等）。

- [ ] **Step 1.7: 不 commit，继续 Task 2**

---

### Task 2: 写 13 个失败测试 + Tree helper

**Files:**
- Modify: `gui/src/widget/frameworks/panel.rs`（末尾追加 `#[cfg(test)] mod tests`）

**Goal:** 写全 13 个单元测试和一个用于集成测试的 Tree 构造 helper。运行后预期 13 个测试全部失败（因为 Task 1 的 `build()` 和 `props_eq` 还是占位）。

- [ ] **Step 2.1: 在 panel.rs 末尾追加 tests 模块骨架**

用 Edit 工具，在文件末尾（最后一个 `}` 之后）追加：

```rust

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gesture::Gesture;
    use crate::renderer::{Color, Rect};
    use crate::tree::layout::{LeafKind, Position, Size};
    use crate::tree::{hit_test, layout, reconcile, NodeId, Tree};

    /// 构造一份标准 props。所有测试从这个 baseline 出发。
    fn sample_props() -> PanelProps {
        PanelProps {
            id: Cow::Borrowed("test"),
            title: Cow::Borrowed("Title"),
            x: 10.0,
            y: 20.0,
            w: 300.0,
            h: 200.0,
            content: Vec::new(),
        }
    }

    /// 用于集成测试：把 PanelProps 装进 Desc::Widget，走 reconcile + layout，
    /// 返回可直接 hit_test 的 Tree + root NodeId。
    fn build_tree_for_hit(props: PanelProps) -> (Tree, NodeId) {
        let desc = Desc::Widget {
            id: Cow::Borrowed("test_panel"),
            props: Box::new(props),
        };

        let mut tree = Tree::new();
        reconcile(&mut tree, desc);

        let root = tree.root().expect("tree should have root after reconcile");

        let mut no_measure = |_text: &str, _size: f32| -> (f32, f32) { (0.0, 0.0) };
        layout(
            &mut tree,
            root,
            Rect { x: 0.0, y: 0.0, w: 1000.0, h: 1000.0 },
            &mut no_measure,
        );

        (tree, root)
    }
}
```

**注意**：`build_tree_for_hit` 的 available Rect 用 1000×1000（足够装下 sample_props 的 300×200 panel 任意位置）。如果 `reconcile` 或 `layout` 的签名与预期不符，读 `gui/src/panel/renderer.rs::update()` 看实际调用方式。

- [ ] **Step 2.2: 运行 cargo check 验证 helper 可编译**

Run: `cargo check -p gui 2>&1 | tail -15`
Expected: 编译通过。可能会有 `unused_variable` / `dead_code` 警告（helper 还没被测试用到），预期的。

如果有错误：
- `reconcile` 签名不对：读 `gui/src/tree/diff.rs` 顶部确认
- `layout` 调用签名：读 `gui/src/tree/layout/layout.rs`
- `Tree::root()` 返回 `Option<NodeId>`，用 `expect`

- [ ] **Step 2.3: 添加 `widget_type_is_panel` 测试**

在 tests 模块里（`build_tree_for_hit` 之后）追加：

```rust
    #[test]
    fn widget_type_is_panel() {
        let props = sample_props();
        assert_eq!(props.widget_type(), "Panel");
    }
```

- [ ] **Step 2.4: 添加 `build_outer_is_absolute` 测试**

```rust
    #[test]
    fn build_outer_is_absolute() {
        let props = sample_props();
        let build = props.build("test");

        // 外框 position = Absolute(x, y)
        match build.style.position {
            Position::Absolute { x, y } => {
                assert_eq!(x, 10.0);
                assert_eq!(y, 20.0);
            }
            other => panic!("expected Absolute, got {:?}", other),
        }

        // 外框 size = Fixed(w, h)
        match build.style.width {
            Size::Fixed(v) => assert_eq!(v, 300.0),
            other => panic!("expected Fixed width, got {:?}", other),
        }
        match build.style.height {
            Size::Fixed(v) => assert_eq!(v, 200.0),
            other => panic!("expected Fixed height, got {:?}", other),
        }
    }
```

**为什么用 match 而不是 `assert_eq!`**：不假设 `Position` / `Size` 实现了 `PartialEq`，用 match 更安全。

- [ ] **Step 2.5: 添加 `build_outer_has_resize_gesture` 测试**

```rust
    #[test]
    fn build_outer_has_resize_gesture() {
        let props = sample_props();
        let build = props.build("test");
        assert_eq!(build.style.gestures, vec![Gesture::Resize]);
    }
```

`Gesture` 在 C.3 已 derive `PartialEq`，`Vec<Gesture>` 可直接用 `assert_eq!`。

- [ ] **Step 2.6: 添加 `build_outer_has_frame_decoration` 测试**

```rust
    #[test]
    fn build_outer_has_frame_decoration() {
        let props = sample_props();
        let build = props.build("test");

        let dec = build.decoration.expect("outer should have decoration");
        assert!(dec.background.is_some(), "outer should have background");
        assert!(dec.border.is_some(), "outer should have border");
    }
```

- [ ] **Step 2.7: 添加 `build_children_count_is_two` 测试**

```rust
    #[test]
    fn build_children_count_is_two() {
        let props = sample_props();
        let build = props.build("test");
        assert_eq!(build.children.len(), 2, "panel should have titlebar + content");
    }
```

- [ ] **Step 2.8: 添加 `build_titlebar_has_drag_gesture` 测试**

```rust
    #[test]
    fn build_titlebar_has_drag_gesture() {
        let props = sample_props();
        let build = props.build("test");

        match &build.children[0] {
            Desc::Container { style, .. } => {
                assert_eq!(style.gestures, vec![Gesture::Drag]);
                match style.height {
                    Size::Fixed(h) => assert_eq!(h, TITLE_BAR_HEIGHT),
                    other => panic!("expected Fixed titlebar height, got {:?}", other),
                }
            }
            other => panic!("first child should be Container (titlebar), got {:?}", desc_variant_name(other)),
        }
    }

    /// Desc variant 名字（用于错误消息）
    fn desc_variant_name(d: &Desc) -> &'static str {
        match d {
            Desc::Container { .. } => "Container",
            Desc::Leaf { .. } => "Leaf",
            Desc::Widget { .. } => "Widget",
        }
    }
```

- [ ] **Step 2.9: 添加 `build_titlebar_contains_title_text` 测试**

```rust
    #[test]
    fn build_titlebar_contains_title_text() {
        let props = sample_props();
        let build = props.build("test");

        let titlebar = match &build.children[0] {
            Desc::Container { children, .. } => children,
            other => panic!("first child should be Container, got {:?}", desc_variant_name(other)),
        };

        assert_eq!(titlebar.len(), 1, "titlebar should have exactly one child (title text)");

        match &titlebar[0] {
            Desc::Leaf { kind: LeafKind::Text { content, .. }, .. } => {
                assert_eq!(content, "Title");
            }
            other => panic!("titlebar child should be Text leaf, got {:?}", desc_variant_name(other)),
        }
    }
```

- [ ] **Step 2.10: 添加 `build_content_flex_grow_holds_user_children` 测试**

```rust
    #[test]
    fn build_content_flex_grow_holds_user_children() {
        // 构造一个带一个用户子节点的 props
        let mut props = sample_props();
        props.content = vec![
            Desc::Leaf {
                id: Cow::Borrowed("user_child"),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: "inner".to_string(),
                    font_size: 12.0,
                    color: Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
                },
            }
        ];

        let build = props.build("test");

        match &build.children[1] {
            Desc::Container { style, decoration, children, .. } => {
                assert_eq!(style.flex_grow, 1.0);
                assert!(decoration.is_none(), "content area should be transparent");
                assert_eq!(children.len(), 1, "content area should hold user children");
            }
            other => panic!("second child should be Container (content), got {:?}", desc_variant_name(other)),
        }
    }
```

- [ ] **Step 2.11: 添加 `props_eq_identical` 测试**

```rust
    #[test]
    fn props_eq_identical() {
        let a = sample_props();
        let b = sample_props();
        assert!(a.props_eq(&b));
    }
```

- [ ] **Step 2.12: 添加 `props_eq_different_position` 测试**

```rust
    #[test]
    fn props_eq_different_position() {
        let a = sample_props();
        let mut b = sample_props();
        b.x = 999.0;
        assert!(!a.props_eq(&b), "x 不同应返回 false");

        let mut c = sample_props();
        c.y = 999.0;
        assert!(!a.props_eq(&c), "y 不同应返回 false");
    }
```

- [ ] **Step 2.13: 添加 `props_eq_different_title` 测试**

```rust
    #[test]
    fn props_eq_different_title() {
        let a = sample_props();
        let mut b = sample_props();
        b.title = Cow::Borrowed("Different");
        assert!(!a.props_eq(&b));
    }
```

- [ ] **Step 2.14: 添加 `hit_on_titlebar_reaches_drag_node` 集成测试**

```rust
    #[test]
    fn hit_on_titlebar_reaches_drag_node() {
        // sample_props: x=10, y=20, w=300, h=200
        // 标题栏 rect 大致是 (10, 20) 到 (310, 52)（32px 高）
        // 点击 (100, 30) 应落在标题栏内部
        let (tree, root) = build_tree_for_hit(sample_props());
        let chain = hit_test(&tree, root, 100.0, 30.0);

        assert!(!chain.is_empty(), "hit chain should not be empty");

        let has_drag = chain.iter().any(|id| {
            tree.get(id)
                .map(|n| n.style.gestures.contains(&Gesture::Drag))
                .unwrap_or(false)
        });
        assert!(has_drag, "hit chain should contain a node with Drag gesture");
    }
```

- [ ] **Step 2.15: 添加 `hit_on_corner_reaches_resize_node` 集成测试**

```rust
    #[test]
    fn hit_on_corner_reaches_resize_node() {
        // sample_props: x=10, y=20, w=300, h=200
        // 外框右下角 (310, 220)
        let (tree, root) = build_tree_for_hit(sample_props());
        let chain = hit_test(&tree, root, 310.0, 220.0);

        assert!(!chain.is_empty(), "hit chain should not be empty");

        let has_resize = chain.iter().any(|id| {
            tree.get(id)
                .map(|n| n.style.gestures.contains(&Gesture::Resize))
                .unwrap_or(false)
        });
        assert!(has_resize, "hit chain should contain a node with Resize gesture");
    }
```

- [ ] **Step 2.16: cargo check 验证测试可编译**

Run: `cargo check -p gui 2>&1 | tail -20`
Expected: 编译通过。可能会有若干 `unused_variable` 警告（Task 3 实现后会消失）。

如果有编译错误：
- `Desc` variants 的匹配模式：读 `gui/src/tree/desc.rs` 确认
- `LeafKind::Text` 的字段名：读 `gui/src/tree/layout/types.rs::LeafKind`
- `Color` 的字段（`r, g, b, a`）：读 `gui/src/renderer.rs` 或 `gui/src/renderer/color.rs`
- `BoxStyle::default()` 存在性：应该 derive 了 Default

- [ ] **Step 2.17: 运行测试，验证全部失败**

Run: `cargo test -p gui --lib widget::frameworks::panel::tests 2>&1 | tail -40`
Expected: **13 个测试全部失败**。典型失败原因：
- 结构测试：`build()` 返回空 WidgetBuild，`children.len() == 0`，各种 match 失败
- props_eq 测试：占位返回 `false`，`props_eq_identical` 断言 `true` 失败
- 集成测试：`reconcile` 把空的 WidgetBuild 展开成空 Container，hit_test 命中空节点或不命中

如果某个测试意外 PASS，检查是不是断言方向写反（比如 `!props_eq_different_*` 系列：占位 false 恰好让这些通过）。

**注意可能意外 PASS 的 2 个测试**：
- `props_eq_different_position`：占位返回 false，`assert!(!false)` → PASS
- `props_eq_different_title`：同上 → PASS

如果只有这 2 个意外 PASS（其余 11 个 FAIL），符合预期，继续 Task 3。

- [ ] **Step 2.18: 不 commit，继续 Task 3**

---

### Task 3: 实现 build() 和 props_eq

**Files:**
- Modify: `gui/src/widget/frameworks/panel.rs`（替换 `build()` 和 `props_eq` 的占位实现）

**Goal:** 把 Task 1 的占位 `build()` 替换为真实实现，让 Task 2 的 13 个测试全部 PASS。

- [ ] **Step 3.1: 在 panel.rs 顶部添加缺失的 import**

Read `gui/src/widget/frameworks/panel.rs` 当前顶部 import 区域。

用 Edit 把当前顶部：
```rust
use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::tree::Desc;
use crate::tree::layout::BoxStyle;
use crate::widget::props::{WidgetBuild, WidgetProps};
```

替换为：
```rust
use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::gesture::Gesture;
use crate::renderer::{Border, Color};
use crate::tree::Desc;
use crate::tree::layout::{BoxStyle, Decoration, Direction, Edges, LeafKind, Position, Size};
use crate::widget::props::{WidgetBuild, WidgetProps};
```

**注意**：如果 `Edges` / `Direction` 等的实际路径不同，根据 C.1 里 `arrange.rs` 的 use 语句做微调。

- [ ] **Step 3.2: 在 PanelProps 上面添加装饰常量**

找到 `pub const TITLE_BAR_HEIGHT: f32 = 32.0;` 这行。用 Edit 在这一行之后追加：

```rust

// ── 装饰占位常量（C.7 会接主题系统） ──

/// 外框背景色（base-900）
const FRAME_BG: Color = Color { r: 0.133, g: 0.145, b: 0.196, a: 1.0 };
/// 外框边框色（base-700）
const FRAME_BORDER: Color = Color { r: 0.263, g: 0.278, b: 0.333, a: 1.0 };
/// 标题栏背景色（base-800）
const TITLEBAR_BG: Color = Color { r: 0.180, g: 0.196, b: 0.255, a: 1.0 };
/// 标题文字色（base-200）
const TITLE_COLOR: Color = Color { r: 0.902, g: 0.910, b: 0.941, a: 1.0 };
/// 外框圆角
const FRAME_RADIUS: f32 = 6.0;
/// 标题文字大小
const TITLE_FONT_SIZE: f32 = 13.0;
```

**注意**：`Color` 可能不是 `const`-可构造（如果它有非 const 的方法）。如果 const 声明报错，改为 `fn frame_bg() -> Color { ... }` 等辅助函数。

- [ ] **Step 3.3: 替换 props_eq 的占位实现**

找到：
```rust
    fn props_eq(&self, _other: &dyn WidgetProps) -> bool {
        // TODO(Task 3): 实际实现
        false
    }
```

用 Edit 替换为：
```rust
    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        // 按业界惯例（React），不深比较 content，
        // children 变化由 reconcile 层的 child diff 处理。
        other
            .as_any()
            .downcast_ref::<Self>()
            .map_or(false, |o| {
                self.id == o.id
                    && self.title == o.title
                    && self.x == o.x
                    && self.y == o.y
                    && self.w == o.w
                    && self.h == o.h
            })
    }
```

- [ ] **Step 3.4: 替换 build() 的占位实现**

找到：
```rust
    fn build(&self, _id: &str) -> WidgetBuild {
        // TODO(Task 3): 实际实现
        WidgetBuild {
            style: BoxStyle::default(),
            decoration: None,
            children: Vec::new(),
        }
    }
```

用 Edit 替换为：
```rust
    fn build(&self, id: &str) -> WidgetBuild {
        // ── 外框 ──
        let outer_style = BoxStyle {
            position: Position::Absolute { x: self.x, y: self.y },
            width: Size::Fixed(self.w),
            height: Size::Fixed(self.h),
            direction: Direction::Column,
            gestures: vec![Gesture::Resize],
            ..BoxStyle::default()
        };

        let outer_decoration = Decoration {
            background: Some(FRAME_BG),
            border: Some(Border {
                width: 1.0,
                color: FRAME_BORDER,
            }),
            radius: [FRAME_RADIUS; 4],
            shadow: None,
        };

        // ── 标题栏 ──
        let titlebar = Desc::Container {
            id: Cow::Owned(format!("{id}::titlebar")),
            style: BoxStyle {
                height: Size::Fixed(TITLE_BAR_HEIGHT),
                padding: Edges::symmetric(6.0, 10.0),
                direction: Direction::Row,
                gestures: vec![Gesture::Drag],
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(TITLEBAR_BG),
                border: None,
                radius: [FRAME_RADIUS, FRAME_RADIUS, 0.0, 0.0],
                shadow: None,
            }),
            children: vec![
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::title")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: self.title.to_string(),
                        font_size: TITLE_FONT_SIZE,
                        color: TITLE_COLOR,
                    },
                },
            ],
        };

        // ── 内容区（透明穿透） ──
        let content = Desc::Container {
            id: Cow::Owned(format!("{id}::content")),
            style: BoxStyle {
                flex_grow: 1.0,
                padding: Edges::all(8.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: self.content.clone(),
        };

        WidgetBuild {
            style: outer_style,
            decoration: Some(outer_decoration),
            children: vec![titlebar, content],
        }
    }
```

**注意**：
- `Edges::symmetric(6.0, 10.0)` 的参数顺序按 C.1 arrange.rs 里的使用方式确认（应该是 `(vertical, horizontal)`）。如果签名不同，根据实际调整
- `Edges::all(8.0)` 在 C.1 已证实存在
- `self.content.clone()` 需要 `Desc` 实现 Clone——查看 `gui/src/tree/desc.rs`，如果 Desc 没 Clone（`Box<dyn WidgetProps>` 可能影响），会编译报错。若报错，改为 `std::mem::take(&mut self.content)` 方案不可行（build 只有 `&self`）。实际上查看 Box<dyn WidgetProps> impl Clone 区块：`widget/props.rs` 有 `impl Clone for Box<dyn WidgetProps>`，所以 Desc::Widget 应该能 Clone。如果 Desc 本身没 derive Clone，需要手动实现或改为 `pub fn clone_desc(d: &Desc) -> Desc` 辅助函数

- [ ] **Step 3.5: cargo check 验证编译**

Run: `cargo check -p gui 2>&1 | tail -20`
Expected: 编译通过（可能还有一些 dead_code 警告，都是预存的）。

常见问题：
- `Desc` 没实现 `Clone`：检查 `gui/src/tree/desc.rs`，如果没有，手动在 panel.rs 写一个 `fn clone_descs(children: &[Desc]) -> Vec<Desc>` 辅助函数，使用 `match` 递归克隆
- `Color` 不能做 `const`：把常量改为 `fn frame_bg() -> Color { ... }` 风格
- `Edges::symmetric` 签名不匹配：查看定义，可能是 `(h, v)` 顺序

- [ ] **Step 3.6: 运行 13 个测试，验证全部 PASS**

Run: `cargo test -p gui --lib widget::frameworks::panel::tests 2>&1 | tail -30`
Expected: **13 passed, 0 failed**

如果有失败：

- `widget_type_is_panel`：检查 `widget_type()` 返回值拼写（应为 `"Panel"`）
- `build_outer_is_absolute`：检查 `Position::Absolute` 的字段顺序和 `Size::Fixed` 的包装
- `build_outer_has_resize_gesture`：确认 `gestures: vec![Gesture::Resize]`
- `build_outer_has_frame_decoration`：确认 outer_decoration 的 background/border 都是 Some
- `build_children_count_is_two`：确认返回的 `children.len() == 2`（titlebar + content）
- `build_titlebar_has_drag_gesture`：`children[0]` 是 Container 且 gestures == [Drag]，height == Fixed(32.0)
- `build_titlebar_contains_title_text`：标题栏的第一个子是 Leaf Text，content == "Title"
- `build_content_flex_grow_holds_user_children`：`children[1]` 的 flex_grow=1, decoration=None, children.len()=1
- `props_eq_identical`：`a.props_eq(&b)` 返回 true（当 a, b 字段全相等时）
- `props_eq_different_position`：x 或 y 不等时返回 false
- `props_eq_different_title`：title 不等时返回 false
- `hit_on_titlebar_reaches_drag_node`：HitChain 非空，某节点 gestures 含 Drag
- `hit_on_corner_reaches_resize_node`：HitChain 非空，某节点 gestures 含 Resize

**集成测试常见失败原因**：
- 命中点 (100, 30) 没落在标题栏：确认 reconcile + layout 之后 titlebar 的 rect 正确——可能是 Row direction 错误、padding 影响等
- 命中点 (310, 220) 没落在外框：确认外框 rect 是 (10, 20, 300, 200)，右下角是 (310, 220)

如果集成测试失败，用 `println!` 临时调试：
```rust
for id in chain.iter() {
    if let Some(n) = tree.get(id) {
        println!("chain node id={:?} rect={:?} gestures={:?}", n.id, n.rect, n.style.gestures);
    }
}
```

- [ ] **Step 3.7: 确认 C.1/C.2/C.3 测试没有回归**

Run: `cargo test -p gui 2>&1 | tail -15`
Expected: `test result: ok. 43 passed; 0 failed`（C.1+C.2+C.3 的 30 个 + C.4 的 13 个）

如果有回归，看具体失败的测试，反向排查 panel.rs 的 impl 是否无意中改了全局状态（不应该会，但以防万一）。

- [ ] **Step 3.8: 不 commit，继续 Task 4**

---

### Task 4: 最终验证 + commit

**Files:** 无改动

**Goal:** 运行 clippy、demo build，确认一切无回归，然后 commit 本次所有改动为一个 feat commit。

- [ ] **Step 4.1: cargo clippy 检查**

Run: `cargo clippy -p gui 2>&1 | tail -30`
Expected: 没有**本次 C.4 新引入的**错误或警告。Phase B/C.1-C.3 遗留的预存警告可以存在，但 panel.rs 不应引入新的。

如果发现新警告（比如未使用的 import、unused_mut、冗余的 clone 等），用 Edit 修复，然后再次 clippy 验证。

常见需要清理：
- `std::mem::*` 或 `std::borrow::*` 未用的 import
- `#[allow(dead_code)]` 需要加到某些测试 helper 上（例如 `desc_variant_name` 如果只在某些测试用）

- [ ] **Step 4.2: cargo build -p app --release 验证 demo 编译**

Run: `cargo build -p app --release 2>&1 | tail -10`
Expected: 编译通过

demo 行为没有任何变化（因为 PanelProps 未被 demo 引用），但必须确保编译依然成功。

- [ ] **Step 4.3: cargo test --workspace 全量回归**

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: 所有 workspace 下的 crate 测试通过。至少 gui crate 应该是 `ok. 43 passed; 0 failed`。

- [ ] **Step 4.4: 检查 git status**

Run: `git status`
Expected 只有三个改动：
```
modified:   gui/src/widget/mod.rs
new file:   gui/src/widget/frameworks/mod.rs
new file:   gui/src/widget/frameworks/panel.rs
```

如果有其他未预期的改动，停止并询问 coordinator。

- [ ] **Step 4.5: commit C.4**

Run:

```bash
cd /Users/aporicho/Desktop/nodeimg/.claude/worktrees/ui && git add gui/src/widget/mod.rs gui/src/widget/frameworks/mod.rs gui/src/widget/frameworks/panel.rs && git commit -m "$(cat <<'EOF'
feat(gui): C.4 PanelProps widget（复合控件框架）

新增 widget/frameworks/ 目录作为复合控件落点，与 widget/atoms/ 平级。
第一个成员是 PanelProps——第一个声明 gestures 且有子结构的复合 widget。

设计要点（详见 spec）：
- Controlled 状态模型：x/y/w/h 每帧由父组件传入，widget 无内部状态
- 字段最小化：id/title/x/y/w/h/content，装饰使用模块内硬编码常量
- build() 生成三层树：
  * 外框（widget 节点）：Absolute + Fixed size + gestures=[Resize]
  * 标题栏（独立 Container）：fixed 32px 高 + gestures=[Drag]
  * 内容区（独立 Container）：flex_grow=1 + decoration=None 透明穿透
- props_eq 不深比较 content（按 React 惯例，children 变化由 reconcile 处理）
- 子节点 id 使用 "::titlebar" / "::title" / "::content" 后缀

新增 13 个单元测试：
- 结构测试（8 个）：widget_type、outer Absolute/gestures/decoration、
  children 计数、titlebar drag/height/标题文本、content flex_grow/透明
- props_eq 测试（3 个）：identical / different_position / different_title
- 集成测试（2 个）：通过 reconcile + layout + hit_test 验证命中标题栏
  能达到 Drag 节点，命中外框角能达到 Resize 节点（C.1+C.2 联动）

零 demo 改动：app/src/demo.rs 未引用 PanelProps，demo 继续使用旧
PanelLayer/PanelFrame/PanelRenderer 系统。C.7 会处理 demo 迁移。

Refs:
- docs/superpowers/specs/2026-04-11-phase-c4-panel-widget-design.md
- docs/superpowers/plans/2026-04-11-phase-c4-panel-widget.md

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 4.6: 确认 commit 成功**

Run: `git log --oneline -5`
Expected: 最顶部是 `feat(gui): C.4 PanelProps widget（复合控件框架）`，下面是 C.3/C.2/C.1 的 commit。

- [ ] **Step 4.7: 最后一次完整测试运行**

Run: `cargo test -p gui 2>&1 | tail -10`
Expected: `test result: ok. 43 passed; 0 failed`

---

## 完成标志

- ✅ `cargo test -p gui`：43 passed（30 原有 + 13 新增）
- ✅ `cargo check --workspace`：无错误
- ✅ `cargo clippy -p gui`：无 C.4 新增警告
- ✅ `cargo build -p app --release`：编译通过，demo 行为零变化
- ✅ `git log` 最顶部是 `feat(gui): C.4 PanelProps widget（复合控件框架）`
- ✅ 三个文件改动：`widget/mod.rs` (modified)、`widget/frameworks/mod.rs` (new)、`widget/frameworks/panel.rs` (new)

## 风险点速查

**风险 1：`Desc` 不实现 `Clone`**
- 位置：Step 3.4 的 `self.content.clone()`
- 缓解：写一个 `clone_desc(d: &Desc) -> Desc` 辅助函数递归 match，或检查 Desc 的 derive 属性

**风险 2：`Position` / `Size` 不实现 `PartialEq`**
- 位置：Step 2.4、2.8 的断言
- 缓解：已经用 `match` 而非 `assert_eq!` 绕开

**风险 3：`Color` 不能做 `const`**
- 位置：Step 3.2 的装饰常量
- 缓解：改为 `fn frame_bg() -> Color { ... }` 等辅助函数

**风险 4：集成测试 12/13 命中点没落在预期区域**
- 位置：Step 3.6 验证
- 缓解：用 `println!` 打印 chain 节点的 rect 和 gestures 调试；若实在有问题，把 `build_tree_for_hit` 的 available Rect 加大、或微调命中点坐标

**风险 5：`Edges::symmetric` 参数顺序**
- 位置：Step 3.4 的标题栏 padding
- 缓解：参考 `widget/atoms/button.rs` 里 `Edges::symmetric(8.0, 16.0)` 的实际语义（水平/垂直）
