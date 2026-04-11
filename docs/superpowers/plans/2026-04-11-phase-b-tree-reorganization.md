# 阶段 B 目录重组实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `panel/tree/`、`widget/layout/`、`widget/node.rs`、`widget/desc.rs` 迁移到新的 `gui/src/tree/` 顶层模块,并把 `PanelTree` 重命名为 `Tree`,让"树"作为独立基础设施脱离 panel。纯重构,行为零变化。

**Architecture:** 按依赖顺序分 7 个 task。每个 task 结束后 `cargo check -p gui` 验证。Task 4 是原子大任务(搬 panel/tree/ 内部,14 个子步骤中间不验证),其他 task 可以独立验证。**整个阶段 B 最终一次性 commit**(见 Task 7),符合 spec 对"纯重构原子性"的要求。

**Tech Stack:** Rust workspace、`git mv` 保留历史、`cargo check -p gui` 为主要验证手段、demo 手测为最终验证。

**Spec:** `docs/superpowers/specs/2026-04-11-phase-b-tree-reorganization-design.md`

---

## 前置知识

### 已知反向依赖(阶段 B 不解决)

`widget/node.rs` 和 `widget/desc.rs` 目前都依赖 `widget::props::WidgetProps`:

```rust
// widget/node.rs:3
use crate::widget::props::WidgetProps;
// widget/desc.rs:3
use super::props::WidgetProps;
```

搬到 `tree/node.rs` 和 `tree/desc.rs` 后,这会变成 `tree → widget::props` 的反向依赖(目标文档 §5 要求 `tree` 不依赖 `widget`)。**阶段 B 不解决这个反向依赖**,保留现状作为阶段 C 的 TODO。

Plan 里 Task 3 的 import 更新会把 `super::props::WidgetProps` 改为 `crate::widget::props::WidgetProps`(因为 `super::` 在 tree/ 下指向 tree::,不再指向 widget::)。

### commit 策略

Spec 决定整个阶段 B **只产生一个 commit**(Task 7 的最后一步)。中间的 Task 1~6 完成后各自 `cargo check` 验证,但**不单独 commit**。如果某个 task 中途出错,使用 `git stash` 暂存已改动的代码回到 Task 起点,重新开始。

这和 "frequent commits" 的一般原则相悖,但阶段 B 是纯重构,原子性比中间快照更重要。

### 关键重命名规则

| 旧名 | 新名 | 适用范围 |
|------|------|---------|
| `PanelTree` | `Tree` | 结构体名(Task 4 做) |
| `panel_tree.rs` | `tree.rs` | 无,文件路径通过 git mv 处理 |
| `PanelNode` | `PanelNode`(**不变**) | 节点数据结构,目标文档保留此名 |
| `NodeId` | `NodeId`(不变) | |
| `NodeKind` | `NodeKind`(不变) | |

Task 4 的 `PanelTree → Tree` rename 要同步更新**所有**引用点:`panel/renderer.rs` 以及所有搬到 `tree/` 下的文件(tree.rs、scroll.rs、layout_adapter.rs、diff.rs、paint.rs、hit.rs)的结构体名、函数签名、变量类型。靠编译器兜底:漏一个就编译失败。

### 路径替换总表

所有在 import 语句里出现的旧路径和新路径:

| 旧路径 | 新路径 | 处理的 task |
|--------|--------|-------------|
| `crate::widget::layout::` | `crate::tree::layout::` | Task 2 |
| `crate::widget::node::` | `crate::tree::node::` 或 `crate::tree::` | Task 3 |
| `crate::widget::desc::` | `crate::tree::desc::` 或 `crate::tree::` | Task 3 |
| `super::props::WidgetProps`(在 tree/desc.rs) | `crate::widget::props::WidgetProps` | Task 3 |
| `crate::panel::tree::` | `crate::tree::` | Task 6 |
| `super::tree::` (在 panel/renderer.rs) | `crate::tree::` | Task 4 |
| `super::tree::PanelTree` (在 diff/hit/paint.rs) | `super::tree::Tree` | Task 4 |
| `use crate::widget::node::` (在 tree/ 下的 diff/hit/paint/tree.rs) | `use super::node::` | Task 4(可选简化) |
| `use crate::widget::layout::` (在 tree/tree.rs 和 layout_adapter.rs) | `use super::layout::` | Task 4 |

---

## File Structure

### 本阶段创建的文件

| 路径 | 职责 | 来源 |
|------|------|------|
| `gui/src/tree/mod.rs` | 模块入口 + 对外 API 的 pub use | 新建 |
| `gui/src/tree/tree.rs` | `Tree` 结构 + arena 方法 | `panel/tree/tree.rs` git mv + PanelTree→Tree |
| `gui/src/tree/scroll.rs` | `impl Tree { scroll }` | 新建,内容来自 `panel/tree/layout.rs` 的 scroll 函数 |
| `gui/src/tree/layout_adapter.rs` | `impl LayoutTree for Tree` | `panel/tree/layout.rs` git mv + 清理 |
| `gui/src/tree/node.rs` | `PanelNode / NodeId / NodeKind` | `widget/node.rs` git mv |
| `gui/src/tree/desc.rs` | `Desc` 枚举 | `widget/desc.rs` git mv |
| `gui/src/tree/diff.rs` | `reconcile` | `panel/tree/diff.rs` git mv |
| `gui/src/tree/paint.rs` | `paint` 遍历 | `panel/tree/paint.rs` git mv |
| `gui/src/tree/hit.rs` | `hit_test` | `panel/tree/hit.rs` git mv |
| `gui/src/tree/layout/` 整目录 | 泛型布局引擎 | `widget/layout/` git mv |

### 本阶段删除的文件

- `gui/src/canvas/tree/`(6 个空注释文件)
- `gui/src/panel/tree/`(所有文件搬走后的空目录)

### 本阶段修改的文件

| 路径 | 改动 |
|------|------|
| `gui/src/lib.rs` | 加 `pub mod tree;` |
| `gui/src/widget/mod.rs` | 删除 `pub mod layout;`、`pub mod node;`、`pub mod desc;`、`pub use layout::{...}` |
| `gui/src/panel/mod.rs` | 删除 `mod tree;` |
| `gui/src/panel/renderer.rs` | import 路径、`PanelTree→Tree`、`scroll` 改为 method |
| `gui/src/widget/atoms/button.rs` | import 路径 |
| `gui/src/widget/atoms/slider.rs` | import 路径 |
| `gui/src/widget/atoms/text_input.rs` | import 路径 |
| `gui/src/widget/atoms/toggle.rs` | import 路径 |
| `gui/src/widget/atoms/dropdown.rs` | import 路径 |
| `app/src/demo.rs` | import 路径 |

---

## Tasks

### Task 1: 创建 tree/ 骨架

**Files:**
- Create: `gui/src/tree/mod.rs`
- Modify: `gui/src/lib.rs`

- [ ] **Step 1.1: 创建空的 tree/mod.rs**

使用 Write 工具创建 `gui/src/tree/mod.rs`,内容:

```rust
// 阶段 B 占位:后续 task 会逐步填入 mod 声明和 pub use
```

- [ ] **Step 1.2: 在 lib.rs 中声明 tree 模块**

读 `gui/src/lib.rs`,当前内容:

```rust
pub mod canvas;
pub mod context;
pub mod gesture;
pub mod renderer;
pub mod shell;
pub mod panel;
pub mod widget;
```

用 Edit 工具把这段改为:

```rust
pub mod canvas;
pub mod context;
pub mod gesture;
pub mod renderer;
pub mod shell;
pub mod panel;
pub mod tree;
pub mod widget;
```

- [ ] **Step 1.3: cargo check 验证**

Run: `cargo check -p gui`
Expected: 编译通过,零警告(新 tree 模块已挂进 lib.rs,虽然空壳但合法)

- [ ] **Step 1.4: 不 commit,继续 Task 2**

---

### Task 2: 迁移 widget/layout → tree/layout

**Files:**
- Move: `gui/src/widget/layout/` → `gui/src/tree/layout/`(整目录)
- Modify: `gui/src/widget/mod.rs`
- Modify: `gui/src/tree/mod.rs`
- Modify: 9 个引用 `crate::widget::layout::` 的文件

- [ ] **Step 2.1: git mv widget/layout → tree/layout**

Run: `git mv gui/src/widget/layout gui/src/tree/layout`
Expected: 无输出(git mv 成功时静默)

验证:

Run: `ls gui/src/tree/layout/`
Expected: `arrange.rs  layout.rs  measure.rs  mod.rs  types.rs`

- [ ] **Step 2.2: 更新 widget/mod.rs,删除 layout 的 mod 和 pub use**

读 `gui/src/widget/mod.rs`,当前内容:

```rust
pub mod atoms;
pub mod desc;
pub mod layout;
pub mod node;
pub mod props;

pub mod action;
mod focus;
mod mapping;
pub mod state;
mod text_edit;

pub use layout::{BoxStyle, Decoration, LeafKind, LayoutTree, Size};
```

用 Edit 把 `pub mod layout;` 删除,同时把最后的 `pub use layout::{...}` 整行删除。结果:

```rust
pub mod atoms;
pub mod desc;
pub mod node;
pub mod props;

pub mod action;
mod focus;
mod mapping;
pub mod state;
mod text_edit;
```

- [ ] **Step 2.3: 更新 tree/mod.rs,添加 layout 的 mod 和 pub use**

用 Edit 把 `gui/src/tree/mod.rs` 的占位注释替换为:

```rust
pub mod layout;

pub use layout::layout;
```

- [ ] **Step 2.4: 更新 widget/atoms/button.rs 的 import**

读 `gui/src/widget/atoms/button.rs` 的 L25 附近。用 Edit 把:

```rust
use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Align, Justify, Edges, LeafKind};
```

改为:

```rust
use crate::tree::layout::{BoxStyle, Decoration, Size, Direction, Align, Justify, Edges, LeafKind};
```

- [ ] **Step 2.5: 更新 widget/atoms/slider.rs 的 import**

读 `gui/src/widget/atoms/slider.rs` 的 L28 附近。用 Edit 把:

```rust
use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Align, LeafKind};
```

改为:

```rust
use crate::tree::layout::{BoxStyle, Decoration, Size, Direction, Align, LeafKind};
```

- [ ] **Step 2.6: 更新 widget/atoms/text_input.rs 的 import**

读 `gui/src/widget/atoms/text_input.rs` 的 L25 附近。用 Edit 把:

```rust
use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Edges, LeafKind};
```

改为:

```rust
use crate::tree::layout::{BoxStyle, Decoration, Size, Direction, Edges, LeafKind};
```

- [ ] **Step 2.7: 更新 widget/atoms/toggle.rs 的 import**

读 `gui/src/widget/atoms/toggle.rs` 的 L25 附近。用 Edit 把:

```rust
use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Align, Justify, Edges, LeafKind};
```

改为:

```rust
use crate::tree::layout::{BoxStyle, Decoration, Size, Direction, Align, Justify, Edges, LeafKind};
```

- [ ] **Step 2.8: 更新 widget/atoms/dropdown.rs 的 import**

读 `gui/src/widget/atoms/dropdown.rs` 的 L26 附近。用 Edit 把:

```rust
use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Align, Edges, LeafKind};
```

改为:

```rust
use crate::tree::layout::{BoxStyle, Decoration, Size, Direction, Align, Edges, LeafKind};
```

- [ ] **Step 2.9: 更新 widget/node.rs 的 import**

读 `gui/src/widget/node.rs` 的 L2。用 Edit 把:

```rust
use crate::widget::layout::{BoxStyle, Decoration, LeafKind};
```

改为:

```rust
use crate::tree::layout::{BoxStyle, Decoration, LeafKind};
```

- [ ] **Step 2.10: 更新 widget/desc.rs 的 import**

读 `gui/src/widget/desc.rs` 的 L2。用 Edit 把:

```rust
use crate::widget::layout::{BoxStyle, Decoration, LeafKind};
```

改为:

```rust
use crate::tree::layout::{BoxStyle, Decoration, LeafKind};
```

- [ ] **Step 2.11: 更新 panel/tree/paint.rs 的 import**

读 `gui/src/panel/tree/paint.rs` 的 L3。用 Edit 把:

```rust
use crate::widget::layout::LeafKind;
```

改为:

```rust
use crate::tree::layout::LeafKind;
```

- [ ] **Step 2.12: 更新 panel/tree/layout.rs 的 import**

读 `gui/src/panel/tree/layout.rs` 的 L3。用 Edit 把:

```rust
use crate::widget::layout::{self, BoxStyle, LeafKind, LayoutTree};
```

改为:

```rust
use crate::tree::layout::{self, BoxStyle, LeafKind, LayoutTree};
```

- [ ] **Step 2.13: 确认没有遗漏的 widget::layout 引用**

Run: `rg 'crate::widget::layout' gui/ app/`
Expected: 零结果

- [ ] **Step 2.14: cargo check 验证**

Run: `cargo check -p gui`
Expected: 编译通过

- [ ] **Step 2.15: 不 commit,继续 Task 3**

---

### Task 3: 迁移 widget/node.rs 和 widget/desc.rs → tree/

**Files:**
- Move: `gui/src/widget/node.rs` → `gui/src/tree/node.rs`
- Move: `gui/src/widget/desc.rs` → `gui/src/tree/desc.rs`
- Modify: `gui/src/widget/mod.rs`
- Modify: `gui/src/tree/mod.rs`
- Modify: `gui/src/tree/desc.rs`(搬迁后修正 `super::props` 相对路径)
- Modify: 所有引用 `crate::widget::node::` 和 `crate::widget::desc::` 的文件

- [ ] **Step 3.1: git mv widget/node.rs → tree/node.rs**

Run: `git mv gui/src/widget/node.rs gui/src/tree/node.rs`

- [ ] **Step 3.2: git mv widget/desc.rs → tree/desc.rs**

Run: `git mv gui/src/widget/desc.rs gui/src/tree/desc.rs`

- [ ] **Step 3.3: 更新 widget/mod.rs**

读 `gui/src/widget/mod.rs`,当前内容:

```rust
pub mod atoms;
pub mod desc;
pub mod node;
pub mod props;

pub mod action;
mod focus;
mod mapping;
pub mod state;
mod text_edit;
```

用 Edit 删除 `pub mod desc;` 和 `pub mod node;` 两行。结果:

```rust
pub mod atoms;
pub mod props;

pub mod action;
mod focus;
mod mapping;
pub mod state;
mod text_edit;
```

- [ ] **Step 3.4: 更新 tree/mod.rs**

读 `gui/src/tree/mod.rs`,当前内容:

```rust
pub mod layout;

pub use layout::layout;
```

用 Edit 改为:

```rust
mod desc;
pub mod layout;
mod node;

pub use desc::Desc;
pub use layout::layout;
pub use node::{NodeId, NodeKind, PanelNode};
```

- [ ] **Step 3.5: 修正 tree/desc.rs 的 super::props 相对路径**

读 `gui/src/tree/desc.rs` 的 L3。当前内容:

```rust
use super::props::WidgetProps;
```

用 Edit 改为(`super::` 现在指向 `tree::`,tree 没有 props,必须改成绝对路径):

```rust
use crate::widget::props::WidgetProps;
```

注意:`tree/node.rs` 的 L3 是 `use crate::widget::props::WidgetProps;`,已经是绝对路径,不需要改。

- [ ] **Step 3.6: 更新 widget/atoms/button.rs 的 desc import**

读 `gui/src/widget/atoms/button.rs` 的 L24 附近(就在 layout 的 use 上一行)。用 Edit 把:

```rust
use crate::widget::desc::Desc;
```

改为:

```rust
use crate::tree::desc::Desc;
```

- [ ] **Step 3.7: 更新 widget/atoms/slider.rs 的 desc import**

读 `gui/src/widget/atoms/slider.rs` 的 L27 附近。用 Edit 把:

```rust
use crate::widget::desc::Desc;
```

改为:

```rust
use crate::tree::desc::Desc;
```

- [ ] **Step 3.8: 更新 widget/atoms/text_input.rs 的 desc import**

读 `gui/src/widget/atoms/text_input.rs` 的 L24 附近。用 Edit 把:

```rust
use crate::widget::desc::Desc;
```

改为:

```rust
use crate::tree::desc::Desc;
```

- [ ] **Step 3.9: 更新 widget/atoms/toggle.rs 的 desc import**

读 `gui/src/widget/atoms/toggle.rs` 的 L24 附近。用 Edit 把:

```rust
use crate::widget::desc::Desc;
```

改为:

```rust
use crate::tree::desc::Desc;
```

- [ ] **Step 3.10: 更新 widget/atoms/dropdown.rs 的 desc import**

读 `gui/src/widget/atoms/dropdown.rs` 的 L25 附近。用 Edit 把:

```rust
use crate::widget::desc::Desc;
```

改为:

```rust
use crate::tree::desc::Desc;
```

- [ ] **Step 3.11: 更新 panel/tree/paint.rs 的 node import**

读 `gui/src/panel/tree/paint.rs` 的 L1。用 Edit 把:

```rust
use crate::widget::node::{NodeId, NodeKind};
```

改为:

```rust
use crate::tree::node::{NodeId, NodeKind};
```

- [ ] **Step 3.12: 更新 panel/tree/layout.rs 的 node import**

读 `gui/src/panel/tree/layout.rs` 的 L1。用 Edit 把:

```rust
use crate::widget::node::{NodeId, NodeKind};
```

改为:

```rust
use crate::tree::node::{NodeId, NodeKind};
```

- [ ] **Step 3.13: 更新 panel/tree/tree.rs 的 node import**

读 `gui/src/panel/tree/tree.rs` 的 L1。用 Edit 把:

```rust
use crate::widget::node::{NodeId, PanelNode};
```

改为:

```rust
use crate::tree::node::{NodeId, PanelNode};
```

- [ ] **Step 3.14: 更新 panel/tree/mod.rs 的 desc 和 node pub use**

读 `gui/src/panel/tree/mod.rs` 的 L7-L8。当前内容:

```rust
pub use crate::widget::desc::Desc;
pub use crate::widget::node::NodeId;
```

用 Edit 改为:

```rust
pub use crate::tree::desc::Desc;
pub use crate::tree::node::NodeId;
```

- [ ] **Step 3.15: 更新 panel/tree/hit.rs 的 node import**

读 `gui/src/panel/tree/hit.rs` 的 L1。用 Edit 把:

```rust
use crate::widget::node::NodeId;
```

改为:

```rust
use crate::tree::node::NodeId;
```

- [ ] **Step 3.16: 更新 panel/tree/diff.rs 的 desc 和 node import**

读 `gui/src/panel/tree/diff.rs` 的 L1-L2。当前内容:

```rust
use crate::widget::desc::Desc;
use crate::widget::node::{NodeId, NodeKind, PanelNode};
```

用 Edit 改为:

```rust
use crate::tree::desc::Desc;
use crate::tree::node::{NodeId, NodeKind, PanelNode};
```

- [ ] **Step 3.17: 确认没有遗漏的 widget::{node,desc} 引用**

Run: `rg 'crate::widget::(node|desc)' gui/ app/`
Expected: 零结果

注:`app/src/demo.rs:11` 的 `use gui::panel::tree::Desc;` 不匹配这个 pattern(它走 `panel::tree::` 间接路径),Task 6 专门处理。

- [ ] **Step 3.18: cargo check 验证**

Run: `cargo check -p gui`
Expected: 编译通过

注:此时 tree/ 下有 `layout/`、`node.rs`、`desc.rs` 三个子项,`panel/tree/` 还完整,两者共存,都能编译。

- [ ] **Step 3.19: 不 commit,继续 Task 4**

---

### Task 4: 搬迁 panel/tree/ 到 tree/ 并重命名 PanelTree → Tree(原子大任务)

**⚠ 原子操作**:这个 task 内部的 14 个子步骤**不独立运行 cargo check**。全部子步骤按顺序完成后,在 Step 4.15 运行 `cargo check`。中间如果需要暂停,用 `git stash` 保存进度,恢复后从被打断的子步骤继续。

**Files:**
- Move: `gui/src/panel/tree/tree.rs` → `gui/src/tree/tree.rs`
- Move: `gui/src/panel/tree/layout.rs` → `gui/src/tree/layout_adapter.rs`
- Move: `gui/src/panel/tree/diff.rs` → `gui/src/tree/diff.rs`
- Move: `gui/src/panel/tree/paint.rs` → `gui/src/tree/paint.rs`
- Move: `gui/src/panel/tree/hit.rs` → `gui/src/tree/hit.rs`
- Create: `gui/src/tree/scroll.rs`
- Delete: `gui/src/panel/tree/mod.rs`
- Delete: `gui/src/panel/tree/`(空目录)
- Modify: `gui/src/tree/mod.rs`
- Modify: `gui/src/panel/mod.rs`
- Modify: `gui/src/panel/renderer.rs`
- Modify: 搬到 tree/ 下的各文件(PanelTree → Tree、import 路径调整)

- [ ] **Step 4.1: git mv panel/tree/tree.rs → tree/tree.rs**

Run: `git mv gui/src/panel/tree/tree.rs gui/src/tree/tree.rs`

- [ ] **Step 4.2: 编辑 tree/tree.rs:PanelTree → Tree,import 改为 super::**

读 `gui/src/tree/tree.rs`,当前内容:

```rust
use crate::tree::node::{NodeId, PanelNode};

/// 面板树存储。用 Vec<Option<>> 做 arena，索引访问。
pub struct PanelTree {
    nodes: Vec<Option<PanelNode>>,
    root: Option<NodeId>,
    free: Vec<NodeId>,
}

impl PanelTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
            free: Vec::new(),
        }
    }

    pub fn insert(&mut self, node: PanelNode) -> NodeId {
        ...
    }
    ...
}
```

用 Edit 工具做两处修改:

1. 把 `use crate::tree::node::{NodeId, PanelNode};` 改为 `use super::node::{NodeId, PanelNode};`
2. 把 doc comment `面板树存储。用 Vec<Option<>> 做 arena，索引访问。` 改为 `全局控件树存储。用 Vec<Option<>> 做 arena，索引访问。`
3. 用 `replace_all: true` 把 `PanelTree` 全部替换为 `Tree`(这会改 struct 定义 + impl block + new 方法里的 `Self` 前后)

验证:

Run: `rg 'PanelTree' gui/src/tree/tree.rs`
Expected: 零结果

- [ ] **Step 4.3: git mv panel/tree/layout.rs → tree/layout_adapter.rs**

Run: `git mv gui/src/panel/tree/layout.rs gui/src/tree/layout_adapter.rs`

- [ ] **Step 4.4: 从 layout_adapter.rs 拆出 scroll 函数,新建 tree/scroll.rs**

先创建 `gui/src/tree/scroll.rs`,用 Write 工具写入完整内容:

```rust
use super::node::NodeId;
use super::tree::Tree;

impl Tree {
    /// 更新滚动偏移。delta 为正数向下滚。
    pub fn scroll(&mut self, node_id: NodeId, delta: f32) {
        if let Some(node) = self.get_mut(node_id) {
            let max = (node.content_height - node.rect.h).max(0.0);
            node.scroll_offset = (node.scroll_offset + delta).clamp(0.0, max);
        }
    }
}
```

- [ ] **Step 4.5: 清理 tree/layout_adapter.rs**

读 `gui/src/tree/layout_adapter.rs`(即原来的 panel/tree/layout.rs),当前内容:

```rust
use crate::tree::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::tree::layout::{self, BoxStyle, LeafKind, LayoutTree};
use crate::renderer::Rect;

impl LayoutTree for PanelTree {
    type NodeId = NodeId;

    fn style(&self, node: NodeId) -> &BoxStyle {
        &self.get(node).unwrap().style
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

    fn text_content(&self, node: NodeId) -> Option<(&str, f32)> {
        match &self.get(node)?.kind {
            NodeKind::Leaf(LeafKind::Text { content, font_size, .. }) => Some((content, *font_size)),
            _ => None,
        }
    }
}

pub fn layout(
    tree: &mut PanelTree,
    root: NodeId,
    available: Rect,
    measure_text: &mut dyn FnMut(&str, f32) -> (f32, f32),
) {
    layout::layout(tree, root, available, measure_text);
}

/// 更新滚动偏移。delta 为正数向下滚。
pub fn scroll(tree: &mut PanelTree, node_id: NodeId, delta: f32) {
    if let Some(node) = tree.get_mut(node_id) {
        let max = (node.content_height - node.rect.h).max(0.0);
        node.scroll_offset = (node.scroll_offset + delta).clamp(0.0, max);
    }
}
```

用 Write 工具**完全重写**这个文件为:

```rust
use super::layout::{BoxStyle, LayoutTree, LeafKind};
use super::node::{NodeId, NodeKind};
use super::tree::Tree;
use crate::renderer::Rect;

impl LayoutTree for Tree {
    type NodeId = NodeId;

    fn style(&self, node: NodeId) -> &BoxStyle {
        &self.get(node).unwrap().style
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

    fn text_content(&self, node: NodeId) -> Option<(&str, f32)> {
        match &self.get(node)?.kind {
            NodeKind::Leaf(LeafKind::Text { content, font_size, .. }) => Some((content, *font_size)),
            _ => None,
        }
    }
}
```

**删除内容**:
1. `pub fn layout(tree, root, ...)` wrapper(阶段 B 决策:外部直接用 `crate::tree::layout(...)`)
2. `pub fn scroll(tree, node_id, delta)`(已在 Step 4.4 变为 `impl Tree::scroll` method)

**重命名**:`PanelTree` → `Tree`

**import 调整**:
- `use crate::tree::node::{NodeId, NodeKind};` → `use super::node::{NodeId, NodeKind};`
- `use super::tree::PanelTree;` → `use super::tree::Tree;`
- `use crate::tree::layout::{self, BoxStyle, LeafKind, LayoutTree};` → `use super::layout::{BoxStyle, LayoutTree, LeafKind};`(注:没有 `self::layout` 别名,因为 wrapper 已删除)

- [ ] **Step 4.6: git mv panel/tree/diff.rs → tree/diff.rs**

Run: `git mv gui/src/panel/tree/diff.rs gui/src/tree/diff.rs`

- [ ] **Step 4.7: 修复 tree/diff.rs 的 PanelTree → Tree 和 import**

读 `gui/src/tree/diff.rs`,当前前 4 行:

```rust
use crate::tree::desc::Desc;
use crate::tree::node::{NodeId, NodeKind, PanelNode};
use super::tree::PanelTree;
use crate::renderer::Rect;
```

用 Edit 把前 3 行改为:

```rust
use super::desc::Desc;
use super::node::{NodeId, NodeKind, PanelNode};
use super::tree::Tree;
```

然后用 Edit 的 `replace_all: true` 把这个文件里所有 `PanelTree` 替换为 `Tree`(涉及 `reconcile`、`reconcile_node`、`reconcile_children`、`create_from_desc` 这些函数的签名里的 `tree: &mut PanelTree`,大约 5-7 处)。

验证:

Run: `rg 'PanelTree' gui/src/tree/diff.rs`
Expected: 零结果

- [ ] **Step 4.8: git mv panel/tree/paint.rs → tree/paint.rs**

Run: `git mv gui/src/panel/tree/paint.rs gui/src/tree/paint.rs`

- [ ] **Step 4.9: 修复 tree/paint.rs 的 PanelTree → Tree 和 import**

读 `gui/src/tree/paint.rs`,当前前 4 行:

```rust
use crate::tree::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::tree::layout::LeafKind;
use crate::renderer::{Color, Point, Renderer, RectStyle, TextStyle};
```

用 Edit 把前 3 行改为:

```rust
use super::layout::LeafKind;
use super::node::{NodeId, NodeKind};
use super::tree::Tree;
```

然后用 Edit 的 `replace_all: true` 把这个文件里所有 `PanelTree` 替换为 `Tree`(涉及 `paint`、`paint_node` 函数的签名,约 2-3 处)。

验证:

Run: `rg 'PanelTree' gui/src/tree/paint.rs`
Expected: 零结果

- [ ] **Step 4.10: git mv panel/tree/hit.rs → tree/hit.rs**

Run: `git mv gui/src/panel/tree/hit.rs gui/src/tree/hit.rs`

- [ ] **Step 4.11: 修复 tree/hit.rs 的 PanelTree → Tree 和 import**

读 `gui/src/tree/hit.rs`,当前前 2 行:

```rust
use crate::tree::node::NodeId;
use super::tree::PanelTree;
```

用 Edit 改为:

```rust
use super::node::NodeId;
use super::tree::Tree;
```

然后用 Edit 的 `replace_all: true` 把这个文件里所有 `PanelTree` 替换为 `Tree`(涉及 `hit_test`、`hit_test_node` 函数的签名,约 2-3 处)。

验证:

Run: `rg 'PanelTree' gui/src/tree/hit.rs`
Expected: 零结果

- [ ] **Step 4.12: 更新 tree/mod.rs 添加所有新模块和 pub use**

读 `gui/src/tree/mod.rs`,当前内容:

```rust
mod desc;
pub mod layout;
mod node;

pub use desc::Desc;
pub use layout::layout;
pub use node::{NodeId, NodeKind, PanelNode};
```

用 Write 工具**完全重写**为:

```rust
mod desc;
mod diff;
mod hit;
pub mod layout;
mod layout_adapter;
mod node;
mod paint;
mod scroll;
mod tree;

pub use desc::Desc;
pub use diff::reconcile;
pub use hit::hit_test;
pub use layout::layout;
pub use node::{NodeId, NodeKind, PanelNode};
pub use paint::paint;
pub use tree::Tree;
```

说明:
- `layout_adapter` 和 `scroll` 没有 `pub use`,因为它们只定义 `impl` block。trait impl 和 inherent impl 在编译时自动对 `Tree` 类型生效,外部无需显式 re-export。
- `layout::layout` 既是 mod 又是 fn——这是 Rust 允许的"同名 mod 和 value"。外部可以写 `crate::tree::layout::BoxStyle`(走 mod)或 `crate::tree::layout(&mut tree, ...)`(走 fn)。

- [ ] **Step 4.13: 删除 panel/tree/mod.rs 和空目录**

Run: `rm -f gui/src/panel/tree/mod.rs && rmdir gui/src/panel/tree`
Expected: 两个命令都无输出(成功时静默)

验证:

Run: `ls gui/src/panel/tree 2>&1 || echo "DELETED"`
Expected: `DELETED`(目录不存在)

- [ ] **Step 4.14: 更新 panel/mod.rs 删除 `mod tree;`**

读 `gui/src/panel/mod.rs`,当前内容:

```rust
mod drag;
mod frame;
mod hit;
mod layer;
mod renderer;
mod resize;
pub mod tree;

pub use drag::apply_drag_move;
pub use frame::PanelFrame;
pub use hit::hit_test_panel;
pub use layer::PanelLayer;
pub use renderer::PanelRenderer;
pub use resize::{ResizeEdge, apply_resize, detect_edge};
```

用 Edit 把 `pub mod tree;` 整行删除。结果:

```rust
mod drag;
mod frame;
mod hit;
mod layer;
mod renderer;
mod resize;

pub use drag::apply_drag_move;
pub use frame::PanelFrame;
pub use hit::hit_test_panel;
pub use layer::PanelLayer;
pub use renderer::PanelRenderer;
pub use resize::{ResizeEdge, apply_resize, detect_edge};
```

- [ ] **Step 4.15: 更新 panel/renderer.rs,改 import + rename + scroll method 化**

读 `gui/src/panel/renderer.rs`,完整内容(53 行):

```rust
use super::tree::{reconcile, layout, paint, hit_test, scroll, Desc, PanelTree, NodeId};
use crate::renderer::{Rect, Renderer, TextMeasurer};

/// 面板渲染器。封装 reconcile → layout → paint 流水线。
pub struct PanelRenderer {
    tree: PanelTree,
}

impl PanelRenderer {
    pub fn new() -> Self {
        Self { tree: PanelTree::new() }
    }

    /// 更新面板内容并重新布局。
    pub fn update(&mut self, desc: Desc, content_rect: Rect, measurer: &mut TextMeasurer) {
        reconcile(&mut self.tree, desc);
        if let Some(root) = self.tree.root() {
            layout(
                &mut self.tree,
                root,
                content_rect,
                &mut |text, size| measurer.measure(text, size),
            );
        }
    }

    /// 绘制面板内容。
    pub fn paint(&self, renderer: &mut Renderer) {
        if let Some(root) = self.tree.root() {
            paint(&self.tree, root, renderer);
        }
    }

    /// 命中测试，返回被点击的 widget id。
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&str> {
        let root = self.tree.root()?;
        hit_test(&self.tree, root, x, y)
    }

    /// 滚动指定节点。
    pub fn scroll(&mut self, node_id: NodeId, delta: f32) {
        scroll(&mut self.tree, node_id, delta);
    }

    pub fn root(&self) -> Option<NodeId> {
        self.tree.root()
    }

    pub fn tree(&self) -> &PanelTree {
        &self.tree
    }
}
```

用 Write 工具**完全重写**为:

```rust
use crate::tree::{reconcile, layout, paint, hit_test, Desc, NodeId, Tree};
use crate::renderer::{Rect, Renderer, TextMeasurer};

/// 面板渲染器。封装 reconcile → layout → paint 流水线。
pub struct PanelRenderer {
    tree: Tree,
}

impl PanelRenderer {
    pub fn new() -> Self {
        Self { tree: Tree::new() }
    }

    /// 更新面板内容并重新布局。
    pub fn update(&mut self, desc: Desc, content_rect: Rect, measurer: &mut TextMeasurer) {
        reconcile(&mut self.tree, desc);
        if let Some(root) = self.tree.root() {
            layout(
                &mut self.tree,
                root,
                content_rect,
                &mut |text, size| measurer.measure(text, size),
            );
        }
    }

    /// 绘制面板内容。
    pub fn paint(&self, renderer: &mut Renderer) {
        if let Some(root) = self.tree.root() {
            paint(&self.tree, root, renderer);
        }
    }

    /// 命中测试，返回被点击的 widget id。
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&str> {
        let root = self.tree.root()?;
        hit_test(&self.tree, root, x, y)
    }

    /// 滚动指定节点。
    pub fn scroll(&mut self, node_id: NodeId, delta: f32) {
        self.tree.scroll(node_id, delta);
    }

    pub fn root(&self) -> Option<NodeId> {
        self.tree.root()
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }
}
```

**变化总结**:
1. `use super::tree::{...}` → `use crate::tree::{...}`
2. `scroll` 从 use 列表移除(变成 method,不需要 free function import)
3. `PanelTree` → `Tree`(共 5 处:use、struct 字段、new 方法里的 `PanelTree::new()`、返回类型、`tree()` 方法返回类型)
4. 第 42 行 `scroll(&mut self.tree, node_id, delta);` → `self.tree.scroll(node_id, delta);`
5. 注意 use 顺序里 `layout` 位置不变,Rust 会同时识别为 fn import(因为 `crate::tree::layout` 通过 `pub use layout::layout` 作为 fn 暴露)

- [ ] **Step 4.16: cargo check 验证**

Run: `cargo check -p gui`
Expected: 编译通过,零警告

如果失败,按错误信息定位问题。常见问题:
- `PanelTree` 还有遗漏的引用 — 运行 `rg 'PanelTree' gui/` 定位
- `super::tree::` 指向错误 — 检查 panel/renderer.rs 的 use 路径
- `scroll` 找不到 — 检查 tree/mod.rs 是否声明了 `mod scroll;`,检查 tree/scroll.rs 里的 `impl Tree { scroll }`

- [ ] **Step 4.17: 不 commit,继续 Task 5**

---

### Task 5: 删除 canvas/tree/ 空骨架

**Files:**
- Delete: `gui/src/canvas/tree/`(整个目录)

- [ ] **Step 5.1: 确认 canvas/tree/ 内容都是空注释**

Run: `cat gui/src/canvas/tree/*.rs`
Expected: 每行都是 `//` 开头的注释,无实际代码

- [ ] **Step 5.2: 确认 canvas/mod.rs 没挂这个子目录**

Run: `cat gui/src/canvas/mod.rs`
Expected: 内容中**没有** `mod tree;` 或 `pub mod tree;`

- [ ] **Step 5.3: 删除 canvas/tree/ 整目录**

Run: `rm -rf gui/src/canvas/tree`
Expected: 无输出

验证:

Run: `ls gui/src/canvas/tree 2>&1 || echo "DELETED"`
Expected: `DELETED`

- [ ] **Step 5.4: cargo check 验证**

Run: `cargo check -p gui`
Expected: 编译通过

- [ ] **Step 5.5: 不 commit,继续 Task 6**

---

### Task 6: 更新外部引用 app/src/demo.rs

**Files:**
- Modify: `app/src/demo.rs`

- [ ] **Step 6.1: 更新 demo.rs 的 panel::tree::Desc import**

读 `app/src/demo.rs` 的 L11。当前内容:

```rust
use gui::panel::tree::Desc;
```

用 Edit 改为:

```rust
use gui::tree::Desc;
```

- [ ] **Step 6.2: 确认没有其他 gui::panel::tree 引用**

Run: `rg 'gui::panel::tree' app/`
Expected: 零结果

- [ ] **Step 6.3: cargo build 验证**

Run: `cargo build -p nodeimg-app --release`
Expected: 编译通过,零新警告

注:如果 app crate 名不是 `nodeimg-app`,查看 `app/Cargo.toml` 确认实际名字,或者用 `cargo build --workspace --release` 构建整个 workspace。

- [ ] **Step 6.4: 不 commit,继续 Task 7**

---

### Task 7: 最终验证和 commit

**Files:** 无(只是验证和 commit)

- [ ] **Step 7.1: cargo check 整个 gui crate**

Run: `cargo check -p gui`
Expected: 编译通过,零新 warning

- [ ] **Step 7.2: cargo clippy**

Run: `cargo clippy -p gui -- -D warnings`
Expected: 无新警告

如果 clippy 抱怨新产生的 warning(不是搬迁带来的,可能是重命名触发的风格建议),按错误信息修复再继续。

- [ ] **Step 7.3: cargo build release**

Run: `cargo build -p nodeimg-app --release`
Expected: 编译通过

- [ ] **Step 7.4: cargo test 全 workspace**

Run: `cargo test --workspace`
Expected: 所有测试通过

如果有测试因为路径变化失败,按错误修复(通常是 test 文件里的 `use` 语句需要更新)。

- [ ] **Step 7.5: 验证没有遗留的 widget::layout/node/desc 引用**

Run: `rg 'crate::widget::(layout|node|desc)' gui/ app/`
Expected: 零结果

- [ ] **Step 7.6: 验证没有遗留的 panel::tree 引用**

Run: `rg 'panel::tree' gui/ app/`
Expected: 零结果

- [ ] **Step 7.7: 验证没有遗留的 PanelTree 标识符**

Run: `rg 'PanelTree' gui/ app/`
Expected: 零结果

- [ ] **Step 7.8: 确认 panel/tree/ 目录已删除**

Run: `ls gui/src/panel/tree 2>&1 || echo "DELETED"`
Expected: `DELETED`

- [ ] **Step 7.9: 确认 canvas/tree/ 目录已删除**

Run: `ls gui/src/canvas/tree 2>&1 || echo "DELETED"`
Expected: `DELETED`

- [ ] **Step 7.10: 确认新 tree/ 结构正确**

Run: `ls gui/src/tree/`
Expected:

```
desc.rs  diff.rs  hit.rs  layout  layout_adapter.rs  mod.rs  node.rs  paint.rs  scroll.rs  tree.rs
```

Run: `ls gui/src/tree/layout/`
Expected:

```
arrange.rs  layout.rs  measure.rs  mod.rs  types.rs
```

- [ ] **Step 7.11: Demo 手测**

Run: `cargo run -p nodeimg-app --release`

人工验证(实际点击):
- [ ] 面板(toolbar、properties、preview)正常显示
- [ ] 按钮点击反应正常
- [ ] 滑块拖拽正常
- [ ] 面板标题栏拖拽移动正常
- [ ] 面板边缘 resize 正常
- [ ] 画布背景渲染正常
- [ ] 行为与阶段 A 结束时(commit `5c08791`)完全一致

如果发现行为差异,**不要 commit**。回到出问题的 task 排查,修复后重新验证。

- [ ] **Step 7.12: git status 确认改动清单**

Run: `git status`
Expected: 看到:
- deleted: `gui/src/canvas/tree/*.rs`(6 个文件)
- renamed: `widget/layout/*` → `tree/layout/*`
- renamed: `widget/node.rs` → `tree/node.rs`
- renamed: `widget/desc.rs` → `tree/desc.rs`
- renamed: `panel/tree/tree.rs` → `tree/tree.rs`
- renamed: `panel/tree/layout.rs` → `tree/layout_adapter.rs`
- renamed: `panel/tree/diff.rs` → `tree/diff.rs`
- renamed: `panel/tree/paint.rs` → `tree/paint.rs`
- renamed: `panel/tree/hit.rs` → `tree/hit.rs`
- deleted: `panel/tree/mod.rs`
- new: `tree/mod.rs`
- new: `tree/scroll.rs`
- modified: `gui/src/lib.rs`
- modified: `gui/src/widget/mod.rs`
- modified: `gui/src/panel/mod.rs`
- modified: `gui/src/panel/renderer.rs`
- modified: `gui/src/widget/atoms/button.rs`, `slider.rs`, `text_input.rs`, `toggle.rs`, `dropdown.rs`
- modified: `app/src/demo.rs`

- [ ] **Step 7.13: commit 整个阶段 B**

Run:

```bash
git add gui/src/tree gui/src/lib.rs gui/src/widget/mod.rs gui/src/panel/mod.rs gui/src/panel/renderer.rs gui/src/widget/atoms/ gui/src/widget/node.rs gui/src/widget/desc.rs gui/src/widget/layout gui/src/panel/tree gui/src/canvas/tree app/src/demo.rs
```

注:`git add` 会同时处理新建、删除、重命名和修改。如果路径引用已删除文件 `git add` 会报错,可以用 `git add -A` 或 `git add .`(但注意只在干净的工作树下用 `-A`/`.`,以防加入无关变更)。

Run:

```bash
git commit -m "$(cat <<'EOF'
refactor(gui): 阶段 B 目录重组 —— tree/ 提升到顶层

把 panel/tree/、widget/layout/、widget/node.rs、widget/desc.rs 迁移
到新的 gui/src/tree/ 顶层模块,并把 PanelTree 重命名为 Tree,让"树"
作为独立基础设施脱离 panel,为阶段 C(canvas 分支共享同一棵树、消除
PanelLayer)做技术前置。

文件迁移映射:
- widget/layout/ 整目录 → tree/layout/
- widget/node.rs → tree/node.rs(PanelNode 结构名不变)
- widget/desc.rs → tree/desc.rs
- panel/tree/tree.rs → tree/tree.rs(PanelTree → Tree 重命名)
- panel/tree/layout.rs → tree/layout_adapter.rs(拆分:layout wrapper
  删除、scroll 独立为 tree/scroll.rs、LayoutTree impl 留在
  layout_adapter.rs)
- panel/tree/diff.rs → tree/diff.rs
- panel/tree/paint.rs → tree/paint.rs
- panel/tree/hit.rs → tree/hit.rs
- panel/tree/mod.rs 内容合并到 tree/mod.rs 后删除

新建文件:
- tree/mod.rs(模块入口,薄风格 re-export)
- tree/scroll.rs(impl Tree { scroll } method 形式)

删除:
- canvas/tree/ 整目录(6 个空注释文件,阶段 C 设计已改为"一棵树两个
  分支",tree 在顶层)
- panel/tree/ 整目录(所有文件搬走后)
- widget/mod.rs 里的 pub use layout::{...}(死代码,无引用)
- panel/tree/layout.rs 里的 pub fn layout wrapper(纯冗余代理)

API 变化:
- PanelTree → Tree(结构体重命名,所有 call site 同步)
- scroll 从 free function 变成 impl Tree method:
  scroll(&mut tree, id, delta) → tree.scroll(id, delta)
- pub mod layout + pub use layout::layout 共存,外部可用
  crate::tree::layout::BoxStyle(mod)和 crate::tree::layout(&mut tree)(fn)

已知 TODO(不在阶段 B 范围):
- tree/node.rs 和 tree/desc.rs 仍依赖 widget::props::WidgetProps,
  形成 tree → widget 的反向依赖,违反目标文档 §5 的"tree 不依赖
  widget"原则。阶段 C 将通过架构调整解决。

零行为变化:demo 视觉和交互与阶段 A 结束时(5c08791)完全一致。
所有变更纯为文件位置、模块路径和类型名称调整。

Refs:
- docs/target/2.0.0-gui.md §5 目标文件结构
- docs/superpowers/specs/2026-04-11-phase-b-tree-reorganization-design.md
- docs/superpowers/plans/2026-04-11-phase-b-tree-reorganization.md

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 7.14: 确认 commit 成功**

Run: `git log --oneline -3`
Expected: 最顶部是 `refactor(gui): 阶段 B 目录重组 —— tree/ 提升到顶层`

Run: `git status`
Expected: `nothing to commit, working tree clean`

---

## 回滚策略

如果整个阶段 B 执行到一半发现方向错误,需要回滚到起点:

```bash
git stash        # 暂存当前改动
git stash drop   # 丢弃暂存
```

注:因为所有中间 task 都没 commit,`git stash drop` 可以一次性回到 Task 1 开始前的状态。

如果执行完成后发现 demo 手测失败需要回滚已 commit 的阶段 B:

```bash
git reset --hard HEAD^
```

**警告**:这会丢失阶段 B 的所有工作。只在确认要完全放弃的情况下使用。更稳妥的做法是 `git revert HEAD`(产生一个反向 commit),保留历史追溯。

---

## 预估工作量

- Task 1:约 5 分钟(3 个 step)
- Task 2:约 30 分钟(15 个 step)
- Task 3:约 40 分钟(19 个 step)
- Task 4:约 60 分钟(17 个 step,包括 3 个 git mv 和大量 Edit)
- Task 5:约 5 分钟
- Task 6:约 5 分钟
- Task 7:约 15 分钟(验证 + demo 手测 + commit)

**总计约 2.5 小时**,比 spec 估计的 "一次 session" 更具体。建议一次连续完成,避免半迁移状态。
