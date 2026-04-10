# 布局引擎重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 消除 LayoutBox 中间转换层，让 Flexbox 引擎通过 LayoutTree trait 直接操作 PanelTree；叶子下沉到渲染图元（Text），Widget trait 删除。

**Architecture:** 分三阶段——先新增类型和泛型布局引擎（不破坏旧代码），再一次性切换 PanelTree 侧和 demo 到新系统，最后清理旧代码。每阶段结束 `cargo build -p gui` 必须通过。

**Tech Stack:** Rust, wgpu, 自研 GUI 框架（gui crate）

**Spec:** `docs/superpowers/specs/2026-04-09-layout-engine-refactor-design.md`

---

## File Structure

**新增：** 无新文件

**修改：**
| 文件 | 职责 |
|------|------|
| `gui/src/widget/layout/types.rs` | 新增 LayoutTree/LeafKind/Decoration，删除 LayoutBox/LayoutResult/ScrollArea |
| `gui/src/widget/layout/measure.rs` | 泛型化 `<T: LayoutTree>` |
| `gui/src/widget/layout/arrange.rs` | 泛型化，直接 set_rect |
| `gui/src/widget/layout/mod.rs` | 泛型入口 |
| `gui/src/widget/mod.rs` | 更新导出 |
| `gui/src/widget/atoms/button.rs` | Widget impl → 返回 Desc 的函数 |
| `gui/src/panel/tree/node.rs` | NodeKind 改造 + PanelNode 加字段 |
| `gui/src/panel/tree/desc.rs` | Desc 改造 |
| `gui/src/panel/tree/diff.rs` | 初始化新字段 |
| `gui/src/panel/tree/layout.rs` | impl LayoutTree for PanelTree |
| `gui/src/panel/tree/paint.rs` | 画图元 |
| `gui/src/panel/tree/hit.rs` | decoration 命中 |
| `gui/src/panel/tree/mod.rs` | 更新导出 |
| `gui/src/shell/app.rs` | update() 加 Renderer 参数 |
| `gui/src/shell/runner.rs` | 传 renderer 给 update() |
| `gui/src/demo.rs` | Button 函数化 |

**删除：**
| 文件 | 原因 |
|------|------|
| `gui/src/widget/trait.rs` | Widget trait 不再需要 |
| `gui/src/widget/layout/scroll.rs` | 滚动逻辑迁移到 panel/tree |

---

### Task 1: 新增类型（types.rs）

**Files:**
- Modify: `gui/src/widget/layout/types.rs`

- [ ] **Step 1: BoxStyle 加 Debug derive**

```rust
// types.rs:11 — 改 #[derive(Clone)] 为：
#[derive(Debug, Clone)]
pub struct BoxStyle {
```

- [ ] **Step 2: 新增 LayoutTree trait**

在 `types.rs` 文件末尾（ScrollArea 之后）追加：

```rust
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
}
```

- [ ] **Step 3: 新增 LeafKind 和 Decoration**

在 LayoutTree trait 之后追加：

```rust
use crate::renderer::style::Border;
use crate::renderer::Color;

/// 渲染图元类型。叶子节点的具体内容。
#[derive(Debug, Clone)]
pub enum LeafKind {
    /// 文本图元。尺寸在创建时由 measure_text() 预算，存入 BoxStyle 的 width/height。
    Text {
        content: String,
        font_size: f32,
        color: Color,
    },
}

/// 视觉装饰。附加在 Container 上，纯视觉属性，不影响布局。
/// paint 时 Decoration → RectStyle 转换（shadow 暂不支持）。
#[derive(Debug, Clone)]
pub struct Decoration {
    pub background: Option<Color>,
    pub border: Option<Border>,
    pub radius: [f32; 4],
}
```

注意：`Color` 和 `Border` 的导入路径。`Color` 在 `renderer::types`，通过 `renderer::Color` re-export。`Border` 在 `renderer::style`，通过 `renderer::Border` re-export。在 types.rs 顶部已有 `use crate::renderer::Rect;`，需要扩展为 `use crate::renderer::{Rect, Color, Border};`。

- [ ] **Step 4: 验证编译**

Run: `cargo build -p gui`
Expected: PASS（新类型未使用，可能有 dead_code 警告，无错误）

- [ ] **Step 5: Commit**

```bash
git add gui/src/widget/layout/types.rs
git commit -m "feat: 新增 LayoutTree trait、LeafKind、Decoration 类型"
```

---

### Task 2: 泛型布局引擎（measure + arrange + mod）

**Files:**
- Modify: `gui/src/widget/layout/measure.rs`
- Modify: `gui/src/widget/layout/arrange.rs`
- Modify: `gui/src/widget/layout/mod.rs`

- [ ] **Step 1: 重写 measure.rs**

完全替换 `measure.rs` 内容：

```rust
use super::types::*;

/// 自底向上计算每个节点的期望尺寸。
pub(crate) fn measure<T: LayoutTree>(tree: &T, node: T::NodeId) -> DesiredSize {
    let children = tree.children(node);
    let style = tree.style(node);

    if children.is_empty() {
        return DesiredSize {
            width: match style.width {
                Size::Fixed(w) => w,
                _ => 0.0,
            },
            height: match style.height {
                Size::Fixed(h) => h,
                _ => 0.0,
            },
        };
    }

    let child_sizes: Vec<DesiredSize> = children.iter().map(|&c| measure(tree, c)).collect();
    let n = child_sizes.len();
    let total_gap = if n > 1 { style.gap * (n as f32 - 1.0) } else { 0.0 };

    let (content_w, content_h) = match style.direction {
        Direction::Column => {
            let w = child_sizes.iter().map(|s| s.width).fold(0.0f32, f32::max);
            let h: f32 = child_sizes.iter().map(|s| s.height).sum::<f32>() + total_gap;
            (w, h)
        }
        Direction::Row => {
            let w: f32 = child_sizes.iter().map(|s| s.width).sum::<f32>() + total_gap;
            let h = child_sizes.iter().map(|s| s.height).fold(0.0f32, f32::max);
            (w, h)
        }
    };

    DesiredSize {
        width: match style.width {
            Size::Fixed(w) => w,
            _ => content_w + style.padding.horizontal() + style.margin.horizontal(),
        },
        height: match style.height {
            Size::Fixed(h) => h,
            _ => content_h + style.padding.vertical() + style.margin.vertical(),
        },
    }
}
```

- [ ] **Step 2: 重写 arrange.rs**

完全替换 `arrange.rs` 内容：

```rust
use crate::renderer::Rect;

use super::measure::measure;
use super::types::*;

/// 自顶向下分配位置，直接写入树节点。
pub(crate) fn arrange<T: LayoutTree>(tree: &mut T, node: T::NodeId, available: Rect) {
    let style = tree.style(node).clone();

    // 扣除 margin
    let after_margin = Rect {
        x: available.x + style.margin.left,
        y: available.y + style.margin.top,
        w: (available.w - style.margin.horizontal()).max(0.0),
        h: (available.h - style.margin.vertical()).max(0.0),
    };

    // 节点最终 rect（margin 内的区域）
    let node_rect = Rect {
        x: after_margin.x,
        y: after_margin.y,
        w: match style.width {
            Size::Fixed(w) => w,
            _ => after_margin.w,
        },
        h: match style.height {
            Size::Fixed(h) => h,
            _ => after_margin.h,
        },
    };

    tree.set_rect(node, node_rect);

    let children = tree.children(node);
    if children.is_empty() {
        return;
    }

    // 扣除 padding → 内容区域
    let content = Rect {
        x: node_rect.x + style.padding.left,
        y: node_rect.y + style.padding.top,
        w: (node_rect.w - style.padding.horizontal()).max(0.0),
        h: (node_rect.h - style.padding.vertical()).max(0.0),
    };

    // 处理滚动
    let scroll_offset = if style.overflow == Overflow::Scroll {
        let offset = tree.scroll_offset(node);

        // 计算内容总高度用于滚动
        let child_sizes: Vec<DesiredSize> = children.iter().map(|&c| measure(&*tree, c)).collect();
        let n = child_sizes.len();
        let total_gap = if n > 1 { style.gap * (n as f32 - 1.0) } else { 0.0 };
        let content_height = match style.direction {
            Direction::Column => child_sizes.iter().map(|s| s.height).sum::<f32>() + total_gap,
            Direction::Row => child_sizes.iter().map(|s| s.height).fold(0.0f32, f32::max),
        };

        tree.set_content_height(node, content_height);
        offset
    } else {
        0.0
    };

    // 度量子节点 + 预读子节点样式字段
    let child_sizes: Vec<DesiredSize> = children.iter().map(|&c| measure(&*tree, c)).collect();
    let child_styles: Vec<(f32, Size, Size)> = children.iter().map(|&c| {
        let s = tree.style(c);
        (s.flex_grow, s.width, s.height)
    }).collect();

    let n = child_sizes.len();
    let total_gap = if n > 1 { style.gap * (n as f32 - 1.0) } else { 0.0 };

    let is_column = style.direction == Direction::Column;

    // 主轴可用空间
    let main_available = if is_column { content.h } else { content.w };

    // 计算固定子节点占用 + 收集 flex_grow
    let mut fixed_main: f32 = total_gap;
    let mut total_grow: f32 = 0.0;
    for (i, &(grow, width, height)) in child_styles.iter().enumerate() {
        let child_main = if is_column { child_sizes[i].height } else { child_sizes[i].width };
        let is_fill = if is_column {
            matches!(height, Size::Fill)
        } else {
            matches!(width, Size::Fill)
        };

        if grow > 0.0 || is_fill {
            total_grow += if grow > 0.0 { grow } else { 1.0 };
        } else {
            fixed_main += child_main;
        }
    }

    let remaining = (main_available - fixed_main).max(0.0);

    // justify_content 偏移
    let (mut main_offset, extra_gap) = match style.justify_content {
        Justify::Start => (0.0, 0.0),
        Justify::End => (remaining.max(0.0), 0.0),
        Justify::Center => (remaining.max(0.0) / 2.0, 0.0),
        Justify::SpaceBetween => {
            if n > 1 && total_grow == 0.0 {
                (0.0, remaining / (n as f32 - 1.0))
            } else {
                (0.0, 0.0)
            }
        }
    };

    main_offset -= scroll_offset;

    // 排列子节点
    for (i, &child) in children.iter().enumerate() {
        let child_desired = &child_sizes[i];
        let (grow, width, height) = child_styles[i];
        let is_fill = if is_column {
            matches!(height, Size::Fill)
        } else {
            matches!(width, Size::Fill)
        };
        let effective_grow = if grow > 0.0 {
            grow
        } else if is_fill {
            1.0
        } else {
            0.0
        };

        // 主轴尺寸
        let child_main = if effective_grow > 0.0 {
            remaining * effective_grow / total_grow
        } else if is_column {
            child_desired.height
        } else {
            child_desired.width
        };

        // 交叉轴尺寸和偏移
        let cross_available = if is_column { content.w } else { content.h };
        let child_cross_desired = if is_column { child_desired.width } else { child_desired.height };
        let (cross_offset, child_cross) = match style.align_items {
            Align::Stretch => (0.0, cross_available),
            Align::Start => (0.0, child_cross_desired),
            Align::End => (cross_available - child_cross_desired, child_cross_desired),
            Align::Center => ((cross_available - child_cross_desired) / 2.0, child_cross_desired),
        };

        let child_rect = if is_column {
            Rect {
                x: content.x + cross_offset,
                y: content.y + main_offset,
                w: child_cross,
                h: child_main,
            }
        } else {
            Rect {
                x: content.x + main_offset,
                y: content.y + cross_offset,
                w: child_main,
                h: child_cross,
            }
        };

        arrange(tree, child, child_rect);

        main_offset += child_main + style.gap + extra_gap;
    }
}
```

- [ ] **Step 3: 重写 mod.rs**

完全替换 `widget/layout/mod.rs` 内容：

```rust
mod arrange;
mod measure;
mod types;

pub use types::*;

use crate::renderer::Rect;

/// 两遍布局：measure → arrange。直接写入树节点的 rect。
pub fn layout<T: LayoutTree>(tree: &mut T, root: T::NodeId, available: Rect) {
    arrange::arrange(tree, root, available);
}
```

注意：删除了 `pub mod scroll;` — scroll.rs 将在 Task 6 删除。先在这一步去掉引用。

- [ ] **Step 4: 不单独验证编译**

> Task 2 和 Task 3 必须一起完成才能编译——布局引擎签名变了，panel/tree/layout.rs 还在引用旧接口。直接继续 Task 3。

---

### Task 3: 切换 PanelTree 侧（node + desc + diff + layout）

**Files:**
- Modify: `gui/src/panel/tree/node.rs`
- Modify: `gui/src/panel/tree/desc.rs`
- Modify: `gui/src/panel/tree/diff.rs`
- Modify: `gui/src/panel/tree/layout.rs`

- [ ] **Step 1: 重写 node.rs**

```rust
use crate::widget::layout::{BoxStyle, Decoration, LeafKind};
use crate::renderer::Rect;

pub type NodeId = usize;

pub enum NodeKind {
    Container { style: BoxStyle, decoration: Option<Decoration> },
    Leaf { style: BoxStyle, kind: LeafKind },
}

pub struct PanelNode {
    pub id: &'static str,
    pub kind: NodeKind,
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub props_hash: u64,
    pub scroll_offset: f32,
    pub content_height: f32,
}
```

- [ ] **Step 2: 重写 desc.rs**

```rust
use std::hash::{Hash, Hasher};
use crate::widget::layout::{BoxStyle, Decoration, LeafKind};
use super::node::NodeKind;

/// 视图描述：view() 函数返回的轻量描述树。
pub enum Desc {
    Container {
        id: &'static str,
        style: BoxStyle,
        decoration: Option<Decoration>,
        children: Vec<Desc>,
    },
    Leaf {
        id: &'static str,
        style: BoxStyle,
        kind: LeafKind,
    },
}

impl Desc {
    pub fn id(&self) -> &'static str {
        match self {
            Desc::Container { id, .. } => id,
            Desc::Leaf { id, .. } => id,
        }
    }

    /// 消费 Desc，返回 (子描述列表, NodeKind)。
    pub(crate) fn into_parts(self) -> (Vec<Desc>, NodeKind) {
        match self {
            Desc::Container { style, decoration, children, .. } => {
                (children, NodeKind::Container { style, decoration })
            }
            Desc::Leaf { style, kind, .. } => {
                (Vec::new(), NodeKind::Leaf { style, kind })
            }
        }
    }

    pub fn props_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match self {
            Desc::Container { id, children, decoration, .. } => {
                0u8.hash(&mut hasher);
                id.hash(&mut hasher);
                children.len().hash(&mut hasher);
                // hash decoration 状态，确保背景色变化触发 reconcile
                if let Some(dec) = decoration {
                    1u8.hash(&mut hasher);
                    if let Some(bg) = &dec.background {
                        bg.r.to_bits().hash(&mut hasher);
                        bg.g.to_bits().hash(&mut hasher);
                        bg.b.to_bits().hash(&mut hasher);
                        bg.a.to_bits().hash(&mut hasher);
                    }
                }
            }
            Desc::Leaf { id, kind, .. } => {
                1u8.hash(&mut hasher);
                id.hash(&mut hasher);
                match kind {
                    LeafKind::Text { content, font_size, color } => {
                        content.hash(&mut hasher);
                        font_size.to_bits().hash(&mut hasher);
                        color.r.to_bits().hash(&mut hasher);
                        color.g.to_bits().hash(&mut hasher);
                        color.b.to_bits().hash(&mut hasher);
                        color.a.to_bits().hash(&mut hasher);
                    }
                }
            }
        }
        hasher.finish()
    }
}
```

- [ ] **Step 3: 更新 diff.rs — create_from_desc 加新字段**

在 `diff.rs:69` 的 `tree.insert(PanelNode { ... })` 中加两个字段：

```rust
let node_id = tree.insert(PanelNode {
    id,
    kind,
    rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
    children: Vec::new(),
    props_hash,
    scroll_offset: 0.0,
    content_height: 0.0,
});
```

同时删除 diff.rs:20 的注释 `// 提取子描述（Container 有 children，Widget 没有）`，改为 `// 提取子描述和节点类型`。

- [ ] **Step 4: 重写 layout.rs — impl LayoutTree for PanelTree**

```rust
use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::widget::layout::{self, BoxStyle, LayoutTree};
use crate::renderer::Rect;

impl LayoutTree for PanelTree {
    type NodeId = NodeId;

    fn style(&self, node: NodeId) -> &BoxStyle {
        match &self.get(node).unwrap().kind {
            NodeKind::Container { style, .. } | NodeKind::Leaf { style, .. } => style,
        }
    }

    fn children(&self, node: NodeId) -> Vec<NodeId> {
        self.get(node).map(|n| n.children.clone()).unwrap_or_default()
    }

    fn set_rect(&mut self, node: NodeId, rect: Rect) {
        if let Some(n) = self.get_mut(node) {
            n.rect = rect;
        }
    }

    fn scroll_offset(&self, node: NodeId) -> f32 {
        self.get(node).map(|n| n.scroll_offset).unwrap_or(0.0)
    }

    fn set_content_height(&mut self, node: NodeId, height: f32) {
        if let Some(n) = self.get_mut(node) {
            n.content_height = height;
        }
    }
}

pub fn layout(tree: &mut PanelTree, root: NodeId, available: Rect) {
    layout::layout(tree, root, available);
}

/// 更新滚动偏移。delta 为正数向下滚。
pub fn scroll(tree: &mut PanelTree, node_id: NodeId, delta: f32) {
    if let Some(node) = tree.get_mut(node_id) {
        let max = (node.content_height - node.rect.h).max(0.0);
        node.scroll_offset = (node.scroll_offset + delta).clamp(0.0, max);
    }
}
```

- [ ] **Step 5: 验证编译**

Run: `cargo build -p gui`
Expected: FAIL — `demo.rs` 还在引用 `Desc::Widget`、`Button`、旧的 `layout()` 签名。继续 Task 4 修复。

---

### Task 4: App trait + demo + paint + hit_test 切换

**Files:**
- Modify: `gui/src/shell/app.rs`
- Modify: `gui/src/shell/runner.rs`
- Modify: `gui/src/panel/tree/paint.rs`
- Modify: `gui/src/panel/tree/hit.rs`
- Modify: `gui/src/panel/tree/mod.rs`
- Modify: `gui/src/demo.rs`

- [ ] **Step 1: App trait — update() 加 Renderer 参数**

`shell/app.rs` 改为：

```rust
use super::context::AppContext;
use super::event::AppEvent;
use crate::renderer::Renderer;

pub trait App: Sized + 'static {
    fn init(ctx: &mut AppContext) -> Self;
    fn event(&mut self, event: AppEvent, ctx: &mut AppContext);
    fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext);
    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext);
}
```

- [ ] **Step 2: runner.rs — 传 renderer 给 update()**

`runner.rs:107` 改为：

```rust
state.app.update(&mut state.renderer, &mut state.ctx);
```

- [ ] **Step 3: 重写 paint.rs**

```rust
use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::widget::layout::LeafKind;
use crate::renderer::{Point, Renderer, RectStyle, TextStyle};

pub fn paint(tree: &PanelTree, root: NodeId, renderer: &mut Renderer) {
    paint_node(tree, root, renderer);
}

fn paint_node(tree: &PanelTree, node_id: NodeId, renderer: &mut Renderer) {
    let Some(node) = tree.get(node_id) else { return };
    let rect = node.rect;

    match &node.kind {
        NodeKind::Container { decoration, .. } => {
            if let Some(dec) = decoration {
                renderer.draw_rect(rect, &RectStyle {
                    color: dec.background.unwrap_or(crate::renderer::Color::TRANSPARENT),
                    border: dec.border,
                    radius: dec.radius,
                    shadow: None,
                });
            }
        }
        NodeKind::Leaf { kind, .. } => {
            match kind {
                LeafKind::Text { content, font_size, color } => {
                    renderer.draw_text(
                        Point { x: rect.x, y: rect.y },
                        content,
                        &TextStyle { color: *color, size: *font_size },
                    );
                }
            }
        }
    }

    let children: Vec<NodeId> = tree
        .get(node_id)
        .map(|n| n.children.clone())
        .unwrap_or_default();
    for child_id in children {
        paint_node(tree, child_id, renderer);
    }
}
```

注意：需要确认 `Color::TRANSPARENT` 是否存在。如果不存在，用 `Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }`。

- [ ] **Step 4: 重写 hit.rs**

```rust
use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;

pub fn hit_test(tree: &PanelTree, root: NodeId, x: f32, y: f32) -> Option<&'static str> {
    hit_test_node(tree, root, x, y)
}

fn hit_test_node(tree: &PanelTree, node_id: NodeId, x: f32, y: f32) -> Option<&'static str> {
    let Some(node) = tree.get(node_id) else { return None };

    let r = &node.rect;
    if x < r.x || x > r.x + r.w || y < r.y || y > r.y + r.h {
        return None;
    }

    // 子节点优先（深度优先）
    for &child_id in &node.children {
        if let Some(id) = hit_test_node(tree, child_id, x, y) {
            return Some(id);
        }
    }

    // 有 decoration 的容器可命中
    match &node.kind {
        NodeKind::Container { decoration, .. } if decoration.is_some() => Some(node.id),
        _ => None,
    }
}
```

- [ ] **Step 5: 更新 panel/tree/mod.rs**

添加 scroll 函数导出。把 layout 模块的 LayoutTree re-import 去掉（PanelTree 直接实现 trait，不需要 mod.rs 额外导出）：

```rust
mod desc;
mod diff;
mod hit;
mod layout;
mod node;
mod paint;
mod tree;

pub use desc::Desc;
pub use diff::reconcile;
pub use hit::hit_test;
pub use layout::{layout, scroll};
pub use node::NodeId;
pub use paint::paint;
pub use tree::PanelTree;
```

变化：`layout` 导出扩展为 `layout::{layout, scroll}`。

- [ ] **Step 6: 重写 demo.rs**

```rust
use crate::canvas::Canvas;
use crate::widget::layout::{Align, BoxStyle, Decoration, Direction, Justify, LeafKind, Size};
use crate::panel::{DragState, PanelFrame, PanelLayer, ResizeState, hit_test_panel};
use crate::panel::tree::{reconcile, layout, paint, hit_test, Desc, PanelTree};
use crate::renderer::{Border, Color, Rect, Renderer};
use crate::shell::{App, AppContext, AppEvent};

const PADDING: f32 = 16.0;
const FONT_SIZE: f32 = 12.0;
const PADDING_V: f32 = 8.0;
const RADIUS: f32 = 4.0;
const BORDER_COLOR: Color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };
const TEXT_COLOR: Color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };
const COLOR_ACTIVE: Color = Color { r: 0.2, g: 0.5, b: 0.9, a: 1.0 };
const COLOR_INACTIVE: Color = Color { r: 0.85, g: 0.85, b: 0.87, a: 1.0 };

fn button(
    id: &'static str,
    text_id: &'static str,
    label: &'static str,
    color: Color,
    renderer: &mut Renderer,
) -> Desc {
    let (text_w, text_h) = renderer.measure_text(label, FONT_SIZE);
    Desc::Container {
        id,
        style: BoxStyle {
            height: Size::Fixed(text_h + PADDING_V * 2.0),
            direction: Direction::Row,
            align_items: Align::Center,
            justify_content: Justify::Center,
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(color),
            border: Some(Border { width: 1.0, color: BORDER_COLOR }),
            radius: [RADIUS; 4],
        }),
        children: vec![Desc::Leaf {
            id: text_id,
            style: BoxStyle {
                width: Size::Fixed(text_w),
                height: Size::Fixed(text_h),
                ..BoxStyle::default()
            },
            kind: LeafKind::Text {
                content: label.to_string(),
                font_size: FONT_SIZE,
                color: TEXT_COLOR,
            },
        }],
    }
}

fn build_view(active: Option<&'static str>, renderer: &mut Renderer) -> Desc {
    Desc::Container {
        id: "__root",
        style: BoxStyle {
            direction: Direction::Column,
            gap: 8.0,
            ..BoxStyle::default()
        },
        decoration: None,
        children: vec![
            button(
                "btn_a", "btn_a_text", "Button A",
                if active == Some("btn_a") { COLOR_ACTIVE } else { COLOR_INACTIVE },
                renderer,
            ),
            button(
                "btn_b", "btn_b_text", "Button B",
                if active == Some("btn_b") { COLOR_ACTIVE } else { COLOR_INACTIVE },
                renderer,
            ),
        ],
    }
}

pub struct DemoApp {
    canvas: Canvas,
    layer: PanelLayer,
    drag: DragState,
    resize: ResizeState,
    tree: PanelTree,
    active_button: Option<&'static str>,
    mouse_x: f32,
    mouse_y: f32,
}

impl App for DemoApp {
    fn init(_ctx: &mut AppContext) -> Self {
        let mut layer = PanelLayer::new();
        layer.add(PanelFrame::new("demo", 100.0, 100.0, 300.0, 200.0));

        Self {
            canvas: Canvas::new(),
            layer,
            drag: DragState::new(),
            resize: ResizeState::new(),
            tree: PanelTree::new(),
            active_button: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    fn event(&mut self, event: AppEvent, _ctx: &mut AppContext) {
        if !matches!(event, AppEvent::MouseMove { .. }) {
            tracing::debug!("{:?}", event);
        }

        let panel_consumed = match &event {
            AppEvent::MousePress { x, y, button } => {
                if *button == crate::shell::MouseButton::Left {
                    if let Some(panel_id) = hit_test_panel(&self.layer, *x, *y) {
                        let frame = self.layer.get(panel_id).unwrap();

                        if let Some(edge) = ResizeState::detect_edge(frame, *x, *y) {
                            self.resize.start(panel_id, edge, *x, *y);
                        } else {
                            let mut button_hit = false;
                            if let Some(root) = self.tree.root() {
                                if let Some(id) = hit_test(&self.tree, root, *x, *y) {
                                    self.active_button = Some(id);
                                    button_hit = true;
                                }
                            }
                            if !button_hit {
                                self.drag.start(panel_id, *x, *y);
                            }
                        }

                        self.layer.bring_to_front(panel_id);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            AppEvent::MouseMove { x, y } => {
                self.mouse_x = *x;
                self.mouse_y = *y;
                let mut consumed = false;
                if self.drag.is_active() {
                    self.drag.update(*x, *y, &mut self.layer);
                    consumed = true;
                }
                if self.resize.is_active() {
                    self.resize.update(*x, *y, &mut self.layer);
                    consumed = true;
                }
                consumed
            }
            AppEvent::MouseRelease { .. } => {
                let was_active = self.drag.is_active() || self.resize.is_active();
                self.drag.end();
                self.resize.end();
                was_active
            }
            _ => false,
        };

        if !panel_consumed {
            self.canvas.event(&event);
        }
    }

    fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext) {
        // 光标样式
        if let Some(edge) = self.resize.current_edge() {
            ctx.cursor.set(edge.cursor_style());
        } else if let Some(panel_id) = hit_test_panel(&self.layer, self.mouse_x, self.mouse_y) {
            if let Some(frame) = self.layer.get(panel_id) {
                if let Some(edge) = ResizeState::detect_edge(frame, self.mouse_x, self.mouse_y) {
                    ctx.cursor.set(edge.cursor_style());
                }
            }
        }

        // 构建描述树 + reconcile（需要 renderer 做文字度量）
        let desc = build_view(self.active_button, renderer);
        reconcile(&mut self.tree, desc);
    }

    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext) {
        // layout 不再需要 renderer
        if let (Some(frame), Some(root)) = (self.layer.get("demo"), self.tree.root()) {
            let content_rect = Rect {
                x: frame.x + PADDING,
                y: frame.y + PADDING,
                w: frame.w - PADDING * 2.0,
                h: frame.h - PADDING * 2.0,
            };
            layout(&mut self.tree, root, content_rect);
        }
        let viewport_w = ctx.size.width as f32 / ctx.scale_factor as f32;
        let viewport_h = ctx.size.height as f32 / ctx.scale_factor as f32;

        self.canvas.render(renderer, viewport_w, viewport_h);

        let tree = &self.tree;
        self.layer.render(renderer, |_frame, renderer| {
            if let Some(root) = tree.root() {
                paint(tree, root, renderer);
            }
        });
    }
}
```

- [ ] **Step 7: 验证编译**

Run: `cargo build -p gui`
Expected: PASS（可能有 dead_code 警告——旧的 LayoutBox/LayoutResult/Widget 未使用）

- [ ] **Step 8: 验证运行**

Run: `cargo run -p gui`
Expected: 窗口显示无限画布 + 面板，面板内有两个按钮（背景色 + 居中文字 + 边框），点击可切换 active 颜色。

- [ ] **Step 9: Commit**

```bash
git add gui/src/
git commit -m "feat: 布局引擎泛型化 + 叶子下沉到图元

- LayoutTree trait 替代 LayoutBox 中间树
- measure/arrange 改为泛型 <T: LayoutTree>
- PanelTree 直接实现 LayoutTree
- Button 从 Widget 分解为 Container + Text 叶子
- App::update() 接收 Renderer 用于文字度量
- paint 直接画图元，hit_test 基于 decoration"
```

---

### Task 5: 清理旧代码

**Files:**
- Modify: `gui/src/widget/layout/types.rs`
- Delete: `gui/src/widget/trait.rs`
- Delete: `gui/src/widget/layout/scroll.rs`
- Modify: `gui/src/widget/atoms/button.rs`
- Modify: `gui/src/widget/mod.rs`

- [ ] **Step 1: 删除 types.rs 中的旧类型**

从 `types.rs` 中删除：

```rust
// 删除 LayoutBox（约 lines 4-8）
pub struct LayoutBox { ... }

// 删除 LayoutResult（约 lines 119-123）
pub struct LayoutResult { ... }

// 删除 ScrollArea（约 lines 125-131）
pub struct ScrollArea { ... }
```

- [ ] **Step 2: 删除 widget/trait.rs**

```bash
rm gui/src/widget/trait.rs
```

- [ ] **Step 3: 删除 widget/layout/scroll.rs**

```bash
rm gui/src/widget/layout/scroll.rs
```

- [ ] **Step 4: 清空 widget/atoms/button.rs — 改为 button 函数模块**

Button 的构建函数已经在 demo.rs 中。`button.rs` 可以清空或保留为 button 组件的 helper。

由于 button 函数目前只在 demo.rs 中使用，直接清空 button.rs：

```rust
// Button 控件已分解为 Container + Text 图元。
// 构建函数见 demo.rs 中的 button()。
```

- [ ] **Step 5: 更新 widget/mod.rs 导出**

```rust
pub mod atoms;
pub mod layout;

mod action;
mod focus;
mod mapping;
mod state;
mod text_edit;

pub use layout::{BoxStyle, Decoration, LeafKind, LayoutTree, Size};
```

删除 `pub mod r#trait;`、`pub use layout::LayoutBox;`、`pub use r#trait::Widget;`。

- [ ] **Step 6: 验证编译 + 运行**

Run: `cargo build -p gui && cargo run -p gui`
Expected: 编译通过，运行效果与 Task 4 相同。

- [ ] **Step 7: Commit**

```bash
git add -A gui/src/
git commit -m "chore: 清理旧代码（LayoutBox、Widget trait、scroll.rs）"
```

---

### Task 6: 更新设计文档

**Files:**
- Modify: `docs/target/2.11.0-layout-refactor.md`

- [ ] **Step 1: 更新文档状态**

将 `docs/target/2.11.0-layout-refactor.md` 的状态从"待实施"改为"已完成"，并更新内容反映最终实现（LayoutTree trait + 叶子下沉到图元）。

- [ ] **Step 2: Commit**

```bash
git add docs/target/2.11.0-layout-refactor.md
git commit -m "docs: 更新布局引擎重构文档状态为已完成"
```

---

## 验证清单

最终验证（Task 5 完成后）：

```bash
cargo build -p gui         # 编译通过，无错误
cargo run -p gui           # 运行 demo
cargo test --workspace     # 全量测试通过
```

目视验证：
- [ ] 面板显示两个按钮，文字居中
- [ ] 按钮有背景色 + 边框 + 圆角
- [ ] 点击按钮切换 active 颜色
- [ ] 无限画布拖拽/缩放正常
- [ ] 面板拖拽/缩放正常

代码验证：
- [ ] `LayoutBox` 类型不存在
- [ ] `LayoutResult` 类型不存在
- [ ] `Widget` trait 不存在
- [ ] `build_layout_box()` 不存在
- [ ] `find_node_by_id()` 不存在
- [ ] `layout()` 不接收 `Renderer` 参数
- [ ] 所有文字定位由布局引擎完成（无手动 `measure_text` + 偏移计算在 paint 中）
