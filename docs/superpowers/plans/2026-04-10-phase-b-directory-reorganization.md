# 阶段 B 目录重组计划大纲

> **状态**：待实施。本文件是恢复工作上下文的大纲，不是分步执行计划。真正执行前需要用 superpowers:writing-plans 技能细化为 bite-sized tasks。

**Goal:** 按照 `docs/target/2.0.0-gui.md §5 目标文件结构` 对 `gui/src/` 做目录重组——把 `panel/tree/` 提升到顶层 `tree/`，把 `widget/layout/` 迁移到 `tree/layout/`，更新所有 import。行为零变化。

**Architecture:** 纯重构。不改变任何数据结构、trait 定义或控制流——只移动文件位置和调整模块路径。目的是让"树"作为核心基础设施从 `panel/` 下脱离出来，成为 widget 和 canvas 共享的独立模块。

**Tech Stack:** Rust module system，`mod`/`pub use` 语句，git mv 保留历史。

**Spec:** 暂无独立 spec。本阶段的设计决策已在 `docs/target/2.0.0-gui.md` 和 `docs/target/2.14.0-concepts.md` 中体现。

---

## 背景

阶段 A 完成后（commit `5c08791`），文档里描述的字段和变体都在代码中存在了。但**文件组织**还是旧的：
- `panel/tree/` 下面是树的实现（tree.rs、node.rs、desc.rs、diff.rs、paint.rs、hit.rs、layout.rs）
- `widget/layout/` 下面是布局引擎（types.rs、measure.rs、arrange.rs、layout.rs）

文档说目标结构是：
- `tree/` 在顶层，包含存储、diff、paint、hit 和 layout/
- `widget/` 只放控件定义（atoms/、composites/、frameworks/），不再包含 layout/
- `panel/` 瘦身，只放面板实例（toolbar、properties、preview）

### 为什么要做这个重组

1. **依赖方向澄清** —— 树是核心基础设施，widget 是填充树的内容。目前"widget 下有 layout"、"panel 下有 tree"的结构让依赖关系反向
2. **阶段 C 的前置条件** —— 阶段 C 要让 canvas 分支共享同一个 tree 实现，就必须让 tree 脱离 panel 模块
3. **代码搜索友好** —— "这个树是怎么实现的"的答案应该在 `tree/` 目录，而不是藏在 `panel/tree/`
4. **文档和代码对齐** —— 文档说"`tree/` 在顶层"，代码就应该有一个顶层的 `tree/`

## 非目标

- **不改行为** —— 任何数据结构、trait、算法都不变，仅仅是文件位置和路径变化
- **不改 demo.rs 的外部 API 使用方式** —— 只会因为 import 路径变化做相应调整
- **不接入新能力** —— Position::Absolute 的布局逻辑、Transform 的 paint 应用、hittable/gestures 的消费都不在本阶段
- **不消除 PanelLayer** —— 那是阶段 C
- **不实现 canvas 分支** —— 那是阶段 C

## 目标文件结构

### Before（当前）

```
gui/src/
├── panel/
│   ├── tree/
│   │   ├── tree.rs
│   │   ├── node.rs              (实际：widget/node.rs 下)
│   │   ├── desc.rs              (实际：widget/desc.rs 下)
│   │   ├── diff.rs
│   │   ├── paint.rs
│   │   ├── hit.rs
│   │   └── layout.rs            (PanelTree 的 LayoutTree trait impl)
│   └── ...
├── widget/
│   ├── layout/
│   │   ├── types.rs             (BoxStyle, LeafKind, Decoration 等)
│   │   ├── measure.rs
│   │   ├── arrange.rs
│   │   └── layout.rs
│   ├── node.rs                  (PanelNode, NodeKind)
│   ├── desc.rs                  (Desc enum)
│   ├── props.rs
│   ├── atoms/
│   └── ...
└── ...
```

注意：当前节点数据结构（`PanelNode`, `NodeKind`）和描述树（`Desc`）已经在 `widget/` 下，但树的存储/遍历算法（tree/diff/paint/hit）在 `panel/tree/` 下。这个分裂是历史遗留。

### After（目标）

```
gui/src/
├── tree/                        ← 核心基础设施
│   ├── mod.rs
│   ├── tree.rs                  ← arena 存储（从 panel/tree/tree.rs 移动）
│   ├── node.rs                  ← 节点数据（从 widget/node.rs 移动）
│   ├── desc.rs                  ← 声明式描述（从 widget/desc.rs 移动）
│   ├── diff.rs                  ← reconcile（从 panel/tree/diff.rs 移动）
│   ├── paint.rs                 ← 绘制遍历（从 panel/tree/paint.rs 移动）
│   ├── hit.rs                   ← 命中测试（从 panel/tree/hit.rs 移动）
│   ├── layout_impl.rs           ← Tree 的 LayoutTree trait impl（从 panel/tree/layout.rs 移动）
│   └── layout/                  ← 布局引擎（从 widget/layout/ 整体移动）
│       ├── mod.rs
│       ├── types.rs             ← BoxStyle, LeafKind, Decoration, 新类型
│       ├── measure.rs
│       ├── arrange.rs
│       └── layout.rs
│
├── widget/                      ← 控件定义
│   ├── mod.rs
│   ├── props.rs                 (不变)
│   ├── state.rs                 (不变)
│   ├── action.rs                (不变)
│   ├── focus.rs                 (不变)
│   ├── text_edit.rs             (不变)
│   ├── mapping.rs               (不变)
│   ├── atoms/                   (不变)
│   └── ... (Composites 和 frameworks 目录在阶段 C 时添加)
│
├── panel/                       ← 瘦身
│   ├── mod.rs
│   ├── frame.rs                 ← 暂时保留（阶段 C 消除）
│   ├── layer.rs                 ← 暂时保留（阶段 C 消除）
│   ├── renderer.rs              ← 暂时保留（阶段 C 消除）
│   ├── drag.rs                  ← 暂时保留
│   ├── resize.rs                ← 暂时保留
│   ├── hit.rs                   ← 暂时保留
│   ├── event.rs                 (不变)
│   └── instances/               (不变)
│
├── canvas/                      ← 基本不动
├── gesture/                     ← 基本不动
├── shell/                       (不变)
├── renderer/                    (不变)
├── theme/                       (不变)
└── context.rs                   ← import 路径调整
```

**关键决定**：`panel/tree/` 这个子目录在阶段 B 结束后会消失，但 `panel/` 本身还在（阶段 C 才会清理 PanelLayer/PanelFrame 等）。阶段 B 的目标不是消灭 `panel/`，只是搬移里面的树实现。

## 迁移清单

按依赖顺序迁移：

### B.1 创建 `gui/src/tree/` 目录和 mod.rs 骨架

```rust
// gui/src/tree/mod.rs
pub mod layout;
pub mod tree;
pub mod node;
pub mod desc;
pub mod diff;
pub mod paint;
pub mod hit;
pub mod layout_impl;

pub use tree::Tree;
pub use node::{PanelNode, NodeKind, NodeId};
pub use desc::Desc;
pub use layout::{BoxStyle, Decoration, LeafKind, /* ... */};
```

（具体的 pub use 清单需要从现有的 widget/mod.rs 和 panel/tree/mod.rs 中提取）

### B.2 迁移 layout 引擎

- `git mv gui/src/widget/layout gui/src/tree/layout`
- 更新 `gui/src/widget/mod.rs` 移除 `pub mod layout;`
- 更新 `gui/src/tree/mod.rs` 加 `pub mod layout;`
- 搜索所有 `crate::widget::layout::` 替换为 `crate::tree::layout::`
  - 估计涉及文件：`gui/src/widget/atoms/*.rs`、`gui/src/widget/props.rs`、`gui/src/panel/tree/*.rs`、`gui/src/canvas/*.rs`
- 验证 `cargo check -p gui` 通过

### B.3 迁移 node 和 desc

- `git mv gui/src/widget/node.rs gui/src/tree/node.rs`
- `git mv gui/src/widget/desc.rs gui/src/tree/desc.rs`
- 更新 `widget/mod.rs` 和 `tree/mod.rs`
- 搜索所有 `crate::widget::node::` 替换为 `crate::tree::node::`
- 搜索所有 `crate::widget::desc::` 替换为 `crate::tree::desc::`
- 搜索 `use crate::widget::{node, desc}` 这类组合 use 也要更新
- 验证编译

### B.4 迁移树存储和算法

- `git mv gui/src/panel/tree gui/src/tree-panel-temp`（临时名）
- 从临时目录把文件挪到 `gui/src/tree/` 下（因为 `tree/` 已经存在）：
  - `tree.rs` → `gui/src/tree/tree.rs`（可能需要重命名或覆盖）
  - `diff.rs` → `gui/src/tree/diff.rs`
  - `paint.rs` → `gui/src/tree/paint.rs`
  - `hit.rs` → `gui/src/tree/hit.rs`
  - `layout.rs` → `gui/src/tree/layout_impl.rs`（避免和 layout/ 目录冲突）
  - `mod.rs` 内容合并到 `gui/src/tree/mod.rs`
- 删除临时目录和 `gui/src/panel/tree/`
- 搜索所有 `crate::panel::tree::` 替换为 `crate::tree::`
- 搜索 `use super::tree::` 这类相对路径，根据文件新位置调整

**难点**：`panel/tree/tree.rs` 定义了 `PanelTree` 结构体。迁移到 `tree/tree.rs` 后，它可以改名为 `Tree`（去掉 Panel 前缀），因为它不再是面板专属的。但这会导致**所有使用 PanelTree 的地方都要重命名**。

**建议**：阶段 B 保持 `PanelTree` 名字不变，留到阶段 C 重命名。

### B.5 清理 panel/tree 引用

- 更新 `gui/src/panel/mod.rs` 移除 `pub mod tree;`
- 更新 `gui/src/panel/renderer.rs` 里的 import（它目前 `use super::tree::...`）
- 更新 `gui/src/context.rs`
- 更新 `gui/src/panel/hit.rs`、`panel/layer.rs` 等引用

### B.6 更新 lib.rs 导出

`gui/src/lib.rs` 需要导出新的 `tree` 模块：

```rust
pub mod tree;
pub mod widget;
pub mod panel;
pub mod canvas;
pub mod gesture;
pub mod shell;
pub mod renderer;
pub mod theme;
pub mod context;
```

### B.7 验证

- `cargo check -p gui` —— 零错，无新 warning
- `cargo clippy -p gui` —— 无新 warning
- `cargo build -p app --release` —— 成功
- 启动 demo，行为与阶段 A 结束时一致

### B.8 commit

单个 commit：`refactor(gui): 阶段 B 目录重组 —— tree/ 提升到顶层`

## 风险和陷阱

### 🔴 Import 路径大面积更新

几乎所有 gui crate 的源文件都会受影响。单个文件改坏会导致整个 crate 编译失败。

**缓解**：
- 按依赖顺序分步执行（B.2 → B.3 → B.4），每步之后独立 `cargo check`
- 使用 `rg`/`grep -r` 找所有引用点，逐一检查
- 不要 search-replace 整个项目，容易误伤注释和字符串

### 🟡 模块名冲突

新的 `tree/` 模块下会有 `tree.rs` 文件，路径是 `crate::tree::tree::Tree`。这看起来重复。可以选择：

- 把 `tree/tree.rs` 的内容合并到 `tree/mod.rs`（更常见的做法）
- 或保持 `tree/tree.rs`，然后在 `tree/mod.rs` 里 `pub use tree::Tree;` 让外部用 `crate::tree::Tree`

**建议**：合并到 `tree/mod.rs`，避免 `tree::tree` 这种重复。

### 🟡 panel/tree/ 和 tree/ 同时存在的时刻

在 B.4 执行到一半时会有两个 tree/ 目录（原 `panel/tree/` 和新建的 `tree/`）。这时代码可能编译失败。

**缓解**：B.4 要么一次完成（所有文件挪完），要么把中间状态严格限制在一个文件操作内（用 stash 暂存问题）。

### 🟢 历史保留

git mv 会保留文件历史。但跨目录的文件移动有时会让 `git log --follow` 跟踪困难。

**缓解**：commit message 里明确记录"原路径 → 新路径"映射。

### 🟡 `panel/tree/layout.rs` 和 `widget/layout/layout.rs` 命名冲突

两个文件都叫 `layout.rs`，职责不同：
- `panel/tree/layout.rs` 是 `PanelTree` 对 `LayoutTree` trait 的实现
- `widget/layout/layout.rs` 是布局入口函数 `layout()`

迁移到 `tree/` 下时冲突：
- `panel/tree/layout.rs` → `tree/layout_impl.rs`（本计划的命名）
- `widget/layout/layout.rs` → `tree/layout/layout.rs`（保持原名）

### 🟢 文档同步

`docs/target/2.0.0-gui.md` 的文件结构图已经描述的是目标状态（阶段 B 完成后），不需要改。

## 验证方法

1. **交叉检查** —— 用 `find gui/src -type f -name '*.rs' | sort` 对比目标结构
2. **编译验证** —— `cargo check -p gui` 和 `cargo build -p app --release` 都通过
3. **grep 验证** —— `grep -r "crate::widget::layout" gui/` 应该零结果（除了 widget/layout 自己内部，但那个目录已经不存在了）；`grep -r "panel::tree" gui/` 应该零结果
4. **Demo 手测** —— 启动 demo，行为与阶段 A 完全一致
5. **git log --follow** —— 对关键文件验证历史可追溯

## 预估工作量

**4-7 天**。大部分时间花在 import 路径更新和编译错误修复上。工作相对机械但琐碎。

建议一次 session 完成，避免工作树长期处于半迁移状态。

## 阶段 B 完成后

- `tree/` 目录在顶层，包含树的全部基础设施
- `widget/` 只有控件定义
- `panel/` 还在，但不再包含 tree 实现
- Canvas 分支的实现（阶段 C）可以直接复用 `crate::tree::*`
- 阶段 C 的"消除 PanelLayer"有了技术前置条件
