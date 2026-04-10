# 阶段 B 目录重组设计:tree/ 提升到顶层

## 背景

阶段 A(commit `5c08791`)完成后,文档里描述的字段和变体都已在代码中存在,但**文件组织**还是旧的:树的实现分散在 `panel/tree/` 和 `widget/` 下,布局引擎住在 `widget/layout/`。

`docs/target/2.0.0-gui.md` §5 定义的目标结构里,`tree/` 是顶层独立模块,`widget/` 只放控件定义,`panel/` 瘦身成只放面板实例。本阶段的任务是把 `gui/src/` 的实际结构对齐到这个目标。

阶段 B 还是阶段 C(消除 PanelLayer、实现 canvas 分支)的**技术前置条件**——阶段 C 要让 canvas 分支和 panel 分支共享同一棵 Tree,必须先让 Tree 脱离 panel 模块。

当前代码状态:

- `panel/tree/` 下 6 个文件,是 panel 专用的树实现(`PanelTree` 结构、reconcile、paint、hit、layout adapter)
- `widget/layout/` 下 5 个文件,是泛型布局引擎(`LayoutTree` trait、measure、arrange)
- `widget/node.rs`、`widget/desc.rs` 定义节点数据和声明式描述
- `canvas/tree/` 存在但全是 6 个空注释文件,没挂进 `canvas/mod.rs`——是更早阶段的废弃预留
- `widget/mod.rs` 导出一行 `pub use layout::{BoxStyle, Decoration, LeafKind, LayoutTree, Size};`——grep 证明是死代码,无任何引用点

## 目标与非目标

### 目标

1. **创建 `gui/src/tree/` 顶层模块**,把 `panel/tree/`、`widget/layout/`、`widget/node.rs`、`widget/desc.rs` 的内容迁移进去
2. **`PanelTree` 重命名为 `Tree`**,所有引用点同步更新
3. **删除 `canvas/tree/` 空骨架**(6 个空注释文件)
4. **删除 `widget/mod.rs` 里的 `pub use layout::{...}`**(死代码)
5. **更新所有 import 路径**:`crate::widget::{layout, node, desc}::*` → `crate::tree::*`、`crate::panel::tree::*` → `crate::tree::*`、`app/src/demo.rs:11` 的 use 语句
6. **`panel/tree/layout.rs` 的三块内容重新分配**:`layout()` wrapper 删除,`scroll()` 独立成 `tree/scroll.rs`,`impl LayoutTree for Tree` 独立成 `tree/layout_adapter.rs`
7. **demo 和所有现有行为零变化**——cargo check/build/clippy 全过,demo 视觉和交互与阶段 A 结束时完全一致

### 非目标

- 不改 Tree 以外的任何数据结构、trait 定义或控制流
- 不实现 Position::Absolute 的布局逻辑(留给未来阶段)
- 不让 `hittable`、`gestures`、`transform` 被真正消费
- 不消除 PanelLayer / PanelFrame / PanelRenderer(那是阶段 C)
- 不实现 canvas 分支的 reconcile(那是阶段 C)
- 不触碰 `renderer/`、`shell/`、`gesture/`、`theme/`、`canvas/*.rs`(除 canvas/tree/ 空骨架外)
- 不添加新功能、新类型、新测试——纯重构

## 指导原则

1. **纯重构,零行为变化** —— 所有变更是文件位置、模块路径和类型名称。运行时行为严格守恒。任何"顺手改一点业务逻辑"的冲动都要压住。
2. **一次 session 完成** —— 避免工作树长期处于半迁移状态。阶段 B 本身小,一次性推完最安全。
3. **一个文件一个职责,文件名忠实反映用途** —— `tree/` 内部拆分遵循项目的文件结构哲学。即使 7 行的 `scroll()` 也值得独立成 `scroll.rs`,因为"滚动状态管理"是独立语义。
4. **对外 API 收敛到 `tree/mod.rs`** —— 外部代码通过 `crate::tree::*` 使用,不直接 reach into `tree::scroll` 或 `tree::layout_adapter` 这种内部文件。

## 文件迁移映射

### 整体搬迁

| 旧路径 | 新路径 | 方式 |
|--------|--------|------|
| `gui/src/widget/layout/` 整目录 | `gui/src/tree/layout/` | `git mv` 整目录 |
| `gui/src/widget/node.rs` | `gui/src/tree/node.rs` | `git mv` 单文件 |
| `gui/src/widget/desc.rs` | `gui/src/tree/desc.rs` | `git mv` 单文件 |
| `gui/src/panel/tree/diff.rs` | `gui/src/tree/diff.rs` | `git mv` 单文件 |
| `gui/src/panel/tree/paint.rs` | `gui/src/tree/paint.rs` | `git mv` 单文件 |
| `gui/src/panel/tree/hit.rs` | `gui/src/tree/hit.rs` | `git mv` 单文件 |

### 拆分和合并

`gui/src/panel/tree/tree.rs`(62 行,`PanelTree` 结构 + arena 方法)变为:

- `gui/src/tree/tree.rs` — `Tree` 结构(`PanelTree` → `Tree` 重命名)+ 所有 arena 方法(`new`/`insert`/`remove`/`get`/`get_mut`/`root`/`set_root`/`iter_mut`)

`gui/src/panel/tree/layout.rs`(56 行)的三块内容分别去向:

| 原内容 | 行数 | 新位置 |
|--------|------|--------|
| `impl LayoutTree for PanelTree { ... }` | 34 行 | `gui/src/tree/layout_adapter.rs`(`PanelTree` → `Tree`) |
| `pub fn scroll(tree, node_id, delta)` | 7 行 | `gui/src/tree/scroll.rs`,改为 `impl Tree { pub fn scroll(&mut self, ...) }` method |
| `pub fn layout(tree, root, available, measure_text)` | 8 行 | **删除**,调用方改用 `crate::tree::layout(...)`(通过 `tree/mod.rs` 的 `pub use layout::layout;` 访问) |

`gui/src/panel/tree/mod.rs`(13 行,仅 `mod` 声明和 `pub use` 列表)的内容合并到新的 `gui/src/tree/mod.rs`。

### 删除

- `gui/src/canvas/tree/` 整目录(6 个空注释文件:`mod.rs`、`tree.rs`、`node.rs`、`diff.rs`、`paint.rs`、`hit.rs`)
- `gui/src/panel/tree/` 整目录(在所有文件搬走后)
- `gui/src/widget/mod.rs` 的 `pub use layout::{BoxStyle, Decoration, LeafKind, LayoutTree, Size};` 这一行
- `gui/src/widget/mod.rs` 的 `pub mod layout;`、`pub mod node;`、`pub mod desc;` 三个声明

## 目标文件结构

```plaintext
gui/src/
│
├── tree/                        ← 新增的顶层模块
│   ├── mod.rs                   ← 模块声明 + pub use(对外 API 入口)
│   ├── tree.rs                  ← Tree struct + arena 方法
│   ├── scroll.rs                ← impl Tree { scroll } 滚动状态管理
│   ├── layout_adapter.rs        ← impl LayoutTree for Tree 布局引擎适配
│   ├── node.rs                  ← PanelNode、NodeId、NodeKind(原 widget/node.rs)
│   ├── desc.rs                  ← Desc 声明式描述(原 widget/desc.rs)
│   ├── diff.rs                  ← reconcile(原 panel/tree/diff.rs)
│   ├── paint.rs                 ← 绘制遍历(原 panel/tree/paint.rs)
│   ├── hit.rs                   ← 命中测试(原 panel/tree/hit.rs)
│   └── layout/                  ← 泛型布局引擎(原 widget/layout/ 整目录)
│       ├── mod.rs
│       ├── types.rs             ← BoxStyle、LeafKind、Decoration、LayoutTree trait 等
│       ├── measure.rs
│       ├── arrange.rs
│       └── layout.rs            ← fn layout<T: LayoutTree>
│
├── widget/                      ← 瘦身:只剩控件定义和状态
│   ├── mod.rs                   ← 删除 layout/node/desc 的 mod 和 pub use
│   ├── props.rs                 (不变)
│   ├── state.rs                 (不变)
│   ├── action.rs                (不变)
│   ├── focus.rs                 (不变)
│   ├── text_edit.rs             (不变)
│   ├── mapping.rs               (不变)
│   └── atoms/                   (不变,内部 import 更新)
│       ├── button.rs
│       ├── slider.rs
│       ├── text_input.rs
│       ├── toggle.rs
│       └── dropdown.rs
│
├── panel/                       ← 删掉 panel/tree/,其余暂留给阶段 C
│   ├── mod.rs                   ← 删除 `mod tree;`
│   ├── frame.rs                 (不变,阶段 C 消除)
│   ├── layer.rs                 (不变,阶段 C 消除)
│   ├── renderer.rs              ← import 更新:super::tree::* → crate::tree::*
│   ├── drag.rs                  (不变)
│   ├── resize.rs                (不变)
│   ├── hit.rs                   (不变)
│   ├── event.rs                 (不变)
│   └── instances/               (不变)
│
├── canvas/                      ← 删掉 canvas/tree/ 空骨架
│   ├── mod.rs                   (不变)
│   ├── background.rs            (不变)
│   ├── camera.rs                (不变)
│   ├── pan.rs                   (不变)
│   ├── node_card.rs             (不变)
│   ├── popup.rs                 (不变)
│   └── renderer.rs              (不变)
│
├── context.rs                   (不变,未引用 tree 相关类型)
├── lib.rs                       ← 新增 `pub mod tree;` 声明
│
└── (其余 gesture/、shell/、renderer/、theme/ 不变)
```

## tree/mod.rs 公开 API

采用**薄风格** re-export——只导出"树作为整体的公开 API",布局细节类型(BoxStyle、LeafKind 等)留在 `crate::tree::layout::` 下,保持分层清晰。

```rust
// gui/src/tree/mod.rs
mod desc;
mod diff;
mod hit;
pub mod layout;                  // 子模块保持 pub,让外部访问 layout::BoxStyle 等
mod layout_adapter;              // 纯实现文件,对外不暴露
mod node;
mod paint;
mod scroll;                      // 纯实现文件,对外不暴露
mod tree;

pub use desc::Desc;
pub use diff::reconcile;
pub use hit::hit_test;
pub use layout::layout;          // fn,不是 mod——通过 `pub mod layout` 和这个 `pub use` 可以同时用 `tree::layout` 作为 mod 和 fn(Rust 允许类型和值同名)
pub use node::{NodeId, NodeKind, PanelNode};
pub use paint::paint;
pub use tree::Tree;
```

外部调用风格:

```rust
use crate::tree::{Tree, Desc, NodeId, reconcile, paint, hit_test};
use crate::tree::layout::{BoxStyle, LeafKind, Decoration, Size, Direction, Align, Justify, Edges};

let mut tree = Tree::new();
reconcile(&mut tree, desc);
crate::tree::layout(&mut tree, root, available, &mut measure_text);  // 布局 pass
paint(&mut tree, root, renderer);
tree.scroll(node_id, delta);     // scroll 作为 method 访问
```

**说明 · 同名 mod + fn**:`layout` 既是 `tree/layout/` 子模块(外部写 `crate::tree::layout::BoxStyle`),又是 `fn layout()`(外部写 `crate::tree::layout(&mut tree, ...)`)。Rust 的类型命名空间(mod)和值命名空间(fn)是分开的,这两种用法不冲突。现有代码里的 `panel/tree/layout.rs` 已经是这种用法,阶段 B 保持不变。

## 关键决策记录

### 决策 1 · `PanelTree` 立即重命名为 `Tree`

**决定**:阶段 B 同时完成目录搬迁和结构体重命名,不推迟到阶段 C。

**替代方案**:阶段 B 只搬目录,保留 `PanelTree` 名字,阶段 C 再 rename。

**理由**:

- 目录搬到 `tree/` 后,如果结构体仍叫 `PanelTree`,会产生"目录名说这是顶层 Tree,结构体名说这是 Panel 专属"的语义裂缝。阶段 B 到 C 之间任何读代码的人(包括未来的自己)都会困惑。
- 阶段 C 已经是大工程(20-32 天、高风险),不应叠加"全局 rename"的机械负担。
- Rust 编译器会兜住 rename 的所有遗漏——漏一个 `PanelTree` 引用就编译失败,风险极低。
- 重命名的额外改动集中在 `panel/renderer.rs` 和 `panel/tree/` 内部文件(后者本来就要搬),边际成本小。
- 项目的文件结构哲学(一个文件一个职责、文件名忠实反映用途)同样适用于类型名:类型名应该忠实反映用途,`PanelTree` 不再专属 panel 就不该再叫这个名字。

### 决策 2 · `scroll.rs` 和 `layout_adapter.rs` 独立成文件

**决定**:`impl LayoutTree for Tree`(34 行)和 `scroll()`(7 行)各自独立成文件,而非合并进 `tree/tree.rs`。

**替代方案 A**:两者都合并进 `tree/tree.rs`——Rust 惯例允许 struct 定义和主要 trait impl 同文件,合并后 tree.rs 约 100 行。

**替代方案 B**:只 scroll 独立,LayoutTree impl 合并进 tree.rs。

**理由**:

- 项目的文件结构哲学("一个文件一个职责,文件名忠实反映用途")要求即使小到 7 行的独立职责也值得独立文件。`scroll()` 是"滚动状态管理",`impl LayoutTree for Tree` 是"Tree 对布局引擎的适配器",两者和"Tree 的存储"是三个不同职责。
- Rust 的 `impl Tree { ... }` 可以散在多个文件里,method 调用点(`tree.scroll(...)`)感知不到文件分布。
- 独立文件让 `tree/` 的结构更"自解释"——读目录清单就能看到每个职责的位置,不需要打开 tree.rs 翻注释分段。
- 代价是偏离 `docs/target/2.0.0-gui.md` §5 的文件清单(§5 没有 `scroll.rs` 和 `layout_adapter.rs`)。但 §5 是 high-level 素描,实现层合理拆分不受其束缚。阶段 B 完成后需要微调 §5 加入这两个追加文件。

**替代方案 C 为什么被排除**:把 `impl LayoutTree for Tree` 放进 `tree/layout/adapter.rs`。这会让 `tree/layout/` 从"纯泛型引擎"变成"知道具体 Tree 类型",破坏"布局引擎对 Tree 类型零依赖"的设计,并引入 layout → tree 的反向依赖。

### 决策 3 · 删除 `widget/mod.rs` 的 `pub use layout::{...}`,tree/mod.rs 薄风格 re-export

**决定**:

1. `widget/mod.rs` 的 `pub use layout::{BoxStyle, Decoration, LeafKind, LayoutTree, Size};` 整行删除,不保留兼容层。
2. `tree/mod.rs` 顶层只 re-export 树级 API(`Tree`、`Desc`、`NodeId`、`NodeKind`、`PanelNode`、`reconcile`、`paint`、`hit_test`、`layout` fn),**不**提升布局细节类型(`BoxStyle`、`LeafKind`、`Decoration`、`Size` 等)到顶层。

**理由(删除 widget pub use)**:

- grep 证明这行是死代码:`widget/atoms/*.rs` 5 个文件全部直接用 `use crate::widget::layout::{...}` 完整路径,没用 widget 顶层的 pub use;`app/src/demo.rs` 也没用。
- 保留这种"没人用的兼容层"是纯粹的技术债——每个看到它的人都要 grep 确认"真的没人用",然后困惑"为什么留着"。
- 架构边界:目标文档 `2.0.0-gui.md` §5 的 widget 模块下没有任何布局相关的公开 API,widget 的角色是"控件定义",不该对外暴露布局类型。

**理由(tree 薄风格)**:

- `crate::tree::Tree` 是"树的核心 API",`BoxStyle` 是"布局引擎的细节类型",两者属于不同层级,不该挤在同一命名空间。
- 完整路径 `crate::tree::layout::BoxStyle` 比 `crate::tree::BoxStyle` **自带分类信息**——路径本身就告诉读者"这是布局引擎的概念",信息量更大。
- atoms/ 的 import 多 8 个字符,打字成本可忽略。
- tree 顶层保持 8 个符号左右,扫一眼就能看完,不会膨胀成 20+ 行的 re-export 列表。

## 影响面统计

### `use crate::widget::layout::` 引用点(9 处)

需要改为 `use crate::tree::layout::`:

- `gui/src/panel/tree/paint.rs:3`(文件搬走后内部路径也要改)
- `gui/src/panel/tree/layout.rs:3`(整个文件被拆分)
- `gui/src/widget/atoms/toggle.rs:25`
- `gui/src/widget/atoms/text_input.rs:25`
- `gui/src/widget/atoms/dropdown.rs:26`
- `gui/src/widget/atoms/slider.rs:28`
- `gui/src/widget/atoms/button.rs:25`
- `gui/src/widget/node.rs:2`(文件搬走时内部 import 也改)
- `gui/src/widget/desc.rs:2`(同上)

### `use crate::widget::node::` 引用点(6 处)

全在 `panel/tree/` 内部,文件整体搬到 `tree/` 后改为 `use crate::tree::node::`(或通过 `tree/mod.rs` 的 `pub use` 简化为 `use crate::tree::{NodeId, ...}`):

- `gui/src/panel/tree/paint.rs:1`
- `gui/src/panel/tree/layout.rs:1`
- `gui/src/panel/tree/tree.rs:1`
- `gui/src/panel/tree/mod.rs:8`
- `gui/src/panel/tree/hit.rs:1`
- `gui/src/panel/tree/diff.rs:2`

### `use crate::widget::desc::` 引用点(7 处)

- `gui/src/panel/tree/mod.rs:7`
- `gui/src/panel/tree/diff.rs:1`
- `gui/src/widget/atoms/toggle.rs:24`
- `gui/src/widget/atoms/text_input.rs:24`
- `gui/src/widget/atoms/dropdown.rs:25`
- `gui/src/widget/atoms/slider.rs:27`
- `gui/src/widget/atoms/button.rs:24`

### `use super::tree` / `use self::tree` 相对路径(5 处)

panel 内部对 tree 子模块的相对引用,文件搬走后改为 `use crate::tree::`:

- `gui/src/panel/renderer.rs:1` ← 外部 call site,真改
- `gui/src/panel/tree/diff.rs:3`
- `gui/src/panel/tree/hit.rs:2`
- `gui/src/panel/tree/layout.rs:2`
- `gui/src/panel/tree/paint.rs:2`

### `use crate::panel::tree::` 外部引用(1 处)

- `app/src/demo.rs:11` `use gui::panel::tree::Desc;` → `use gui::tree::Desc;`

### `PanelTree` 标识符引用(rename 影响)

- `gui/src/panel/tree/tree.rs` — struct 定义(搬到 `tree/tree.rs` 时 rename)
- `gui/src/panel/tree/layout.rs` — `impl LayoutTree for PanelTree`(拆分到 `layout_adapter.rs` 时 rename)
- `gui/src/panel/tree/diff.rs`、`hit.rs`、`paint.rs` — use + 函数签名(搬走时 rename)
- `gui/src/panel/tree/mod.rs` — `pub use tree::PanelTree;`(合并到新 mod.rs 时 rename)
- `gui/src/panel/renderer.rs:1` — use + 签名(真 call site,直接 rename)

合计约 **10-15 个文件**需要改动 use 语句或 `PanelTree` 引用。

## 执行步骤

按依赖顺序分步,每步独立 `cargo check -p gui` 验证。

### B.1 创建 `tree/` 目录骨架

- 新建 `gui/src/tree/mod.rs` 只含空 mod 声明(占位,逐步填)
- 更新 `gui/src/lib.rs` 添加 `pub mod tree;`
- `cargo check -p gui` — 应通过(新模块还没用)

### B.2 迁移 `widget/layout/` → `tree/layout/`

- `git mv gui/src/widget/layout gui/src/tree/layout`
- 更新 `gui/src/widget/mod.rs`:删除 `pub mod layout;` 和 `pub use layout::{...};`
- 更新 `gui/src/tree/mod.rs`:添加 `pub mod layout;` 和 `pub use layout::layout;`
- 全局替换 `use crate::widget::layout::` → `use crate::tree::layout::`(9 处)
- `cargo check -p gui` — 应通过

### B.3 迁移 `widget/node.rs` 和 `widget/desc.rs` → `tree/node.rs` 和 `tree/desc.rs`

- `git mv gui/src/widget/node.rs gui/src/tree/node.rs`
- `git mv gui/src/widget/desc.rs gui/src/tree/desc.rs`
- 更新 `gui/src/widget/mod.rs`:删除 `pub mod node;` 和 `pub mod desc;`
- 更新 `gui/src/tree/mod.rs`:添加 `mod node;`、`mod desc;`、`pub use node::{NodeId, NodeKind, PanelNode};`、`pub use desc::Desc;`
- 全局替换 `crate::widget::node::` → `crate::tree::node::`(6 处)、`crate::widget::desc::` → `crate::tree::desc::`(7 处)
- 注:`tree/node.rs` 和 `tree/desc.rs` 内部引用 `crate::widget::layout::` 已经在 B.2 的全局替换里被改为 `crate::tree::layout::`,无需额外处理
- `cargo check -p gui` — 应通过

### B.4 搬迁 `panel/tree/` 下所有文件到 `tree/`(原子操作)

B.4 是一次原子操作,内部有多个子步骤。**中间不独立 `cargo check`**,全部完成后再验证。原因:`panel/tree/tree.rs` 定义的 `PanelTree` 是 `panel/tree/diff.rs`、`hit.rs`、`paint.rs`、`layout.rs` 的共同依赖,中间任何一个文件先搬都会临时断开引用。

**子步骤**(按顺序执行,全部完成后再 `cargo check`):

1. `git mv gui/src/panel/tree/tree.rs gui/src/tree/tree.rs` — 保留 git 历史
2. 编辑 `gui/src/tree/tree.rs`:`PanelTree` → `Tree`,删除 `use crate::widget::node::{NodeId, PanelNode};` 改为 `use super::node::{NodeId, PanelNode};`
3. `git mv gui/src/panel/tree/layout.rs gui/src/tree/layout_adapter.rs` — 保留 git 历史
4. 编辑 `gui/src/tree/layout_adapter.rs`:
   - `PanelTree` → `Tree`
   - `use crate::widget::node::{NodeId, NodeKind};` → `use super::node::{NodeId, NodeKind};`
   - `use super::tree::PanelTree;` → `use super::tree::Tree;`
   - `use crate::widget::layout::{self, BoxStyle, LeafKind, LayoutTree};` → `use super::layout::{BoxStyle, LeafKind, LayoutTree};`
   - **删除** `pub fn layout(tree, root, ...)` wrapper(整段,约 8 行)
   - **删除** `pub fn scroll(tree, node_id, delta)` 函数(整段,约 7 行,下一步新建 scroll.rs 时用)
5. 新建 `gui/src/tree/scroll.rs`(`Write` 工具),内容:
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
6. `git mv gui/src/panel/tree/diff.rs gui/src/tree/diff.rs`
7. `git mv gui/src/panel/tree/paint.rs gui/src/tree/paint.rs`
8. `git mv gui/src/panel/tree/hit.rs gui/src/tree/hit.rs`
9. 编辑这三个文件内部的 import:
   - `use super::tree::PanelTree;` → `use super::tree::Tree;`
   - 所有函数签名里的 `PanelTree` → `Tree`
   - `use crate::widget::node::*` 已在 B.3 改为 `use crate::tree::node::*`,这里进一步简化为 `use super::node::*`(可选)
10. 更新 `gui/src/tree/mod.rs`,添加所有新模块声明和 re-export(薄风格):
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
11. 删除空的 `gui/src/panel/tree/mod.rs` 和 `gui/src/panel/tree/` 目录:`rm -f gui/src/panel/tree/mod.rs && rmdir gui/src/panel/tree`
12. 更新 `gui/src/panel/mod.rs`:删除 `mod tree;`(或 `pub mod tree;`)
13. 更新 `gui/src/panel/renderer.rs:1`:
    - `use super::tree::{reconcile, layout, paint, hit_test, scroll, Desc, PanelTree, NodeId};` → `use crate::tree::{reconcile, layout, paint, hit_test, Desc, Tree, NodeId};`
    - 所有 `PanelTree` 引用改为 `Tree`
    - `scroll(&mut tree, id, delta)` 改为 `tree.scroll(id, delta)`(method 形式)
    - `layout(&mut tree, ...)` 调用点改为 `crate::tree::layout(&mut tree, ...)`(因为原来的 `scroll` 和 `layout` 都不在同一个 use 里了)
14. `cargo check -p gui` — 应通过

### B.5 删除 `canvas/tree/` 空骨架

- `rm -rf gui/src/canvas/tree`
- `gui/src/canvas/mod.rs` 本来就没挂这个目录,无需改动
- `cargo check -p gui` — 应通过

### B.6 更新外部引用 `app/src/demo.rs`

- `app/src/demo.rs:11`:`use gui::panel::tree::Desc;` → `use gui::tree::Desc;`
- `cargo build -p nodeimg-app --release` — 应通过

### B.7 最终验证

- `cargo check -p gui` — 零错,无新 warning
- `cargo clippy -p gui -- -D warnings` — 无新 warning
- `cargo build -p nodeimg-app --release` — 成功
- `cargo test --workspace` — 通过(如果有相关测试)
- 启动 demo 手测:按钮、滑块、面板拖拽、面板 resize 行为与阶段 A 结束时完全一致
- `rg 'crate::widget::(layout|node|desc)' gui/` — 零结果
- `rg 'panel::tree' gui/ app/` — 零结果
- `rg 'PanelTree' gui/ app/` — 零结果

### B.8 commit

单个 commit,消息包含:

- 标题:`refactor(gui): 阶段 B 目录重组 —— tree/ 提升到顶层`
- Body 包含迁移映射表、`PanelTree → Tree` rename 说明、scroll.rs / layout_adapter.rs 独立的决策理由、零行为变化声明

## 风险和缓解

### 🔴 Import 路径大面积更新导致编译错误

几乎所有 `gui` crate 的源文件都会受影响。单个文件改坏会导致整个 crate 编译失败。

**缓解**:

- 按依赖顺序分步(B.1 → B.2 → B.3 → B.4 → ...),每步独立 `cargo check`
- B.4 是原子操作,内部 14 个子步骤中间不独立验证,但 B.4 完成后 `cargo check` 必须通过
- 用 `rg --type rust 'crate::widget::(layout|node|desc)'` 精确找引用点,避免误伤注释和字符串
- `PanelTree → Tree` 的 rename 靠编译器兜底:漏一个就编译失败

### 🟡 `tree/tree.rs` 的路径重复看起来怪

外部写 `crate::tree::Tree` 没问题,但 `tree/tree.rs` 这种文件名在文件浏览器里容易让人困惑"为什么目录和文件同名"。

**缓解**:

- 这是 Rust 模块系统的合法用法(类似 `std::io::stdin` 住在 `std/src/io/stdio.rs`)
- `tree/mod.rs` 的 `pub use tree::Tree;` 让外部看到的是 `crate::tree::Tree`,不暴露 `tree::tree::Tree` 的中间路径
- spec 文档和 commit message 明确说明这个命名

### 🟡 B.4 内部的编译失败窗口

B.4 的 14 个子步骤中间,代码会处于不可编译状态。例如子步骤 1 搬走 `panel/tree/tree.rs` 后,`panel/tree/diff.rs` 等还引用 `super::tree::PanelTree`,而 `super::tree` 已经不存在(文件被搬走);子步骤 2 修改 `tree/tree.rs` 的 `PanelTree → Tree` 后,`panel/renderer.rs` 的 `use super::tree::PanelTree` 也会失效。

**缓解**:

- B.4 作为一次原子操作执行,14 个子步骤按执行步骤里列出的顺序一次完成,中间不运行 `cargo check` 也不 commit
- B.4 完成后立即 `cargo check -p gui`,一旦通过就进入 B.5
- 如果 B.4 中间因任何原因需要暂停,**不要 commit**,使用 `git stash` 保存进度,恢复后继续按子步骤顺序完成 B.4
- plan 阶段(writing-plans)会把 B.4 进一步拆细成可逐步执行的 bite-sized tasks,使用 TodoWrite 追踪,但 spec 这里保持 B.4 的原子性

### 🟢 历史保留

`git mv` 会保留文件历史。跨目录的移动有时让 `git log --follow` 跟踪困难。

**缓解**:

- 在 commit message 里明确记录"原路径 → 新路径"映射,方便未来 `git log --follow` 失效时人工追溯
- `tree.rs` 的内容从 `panel/tree/tree.rs` + `panel/tree/layout.rs` 合并而来,其中只有前者是主体,`git log --follow` 会跟踪 `panel/tree/tree.rs` 的历史;`scroll.rs` 和 `layout_adapter.rs` 的历史可以通过 commit message 指向 `panel/tree/layout.rs`

## 验证方法

1. **编译验证**:`cargo check -p gui`、`cargo clippy -p gui`、`cargo build -p nodeimg-app --release` 全部通过,零新 warning
2. **grep 验证**:
   - `rg 'crate::widget::(layout|node|desc)' gui/` 零结果
   - `rg 'panel::tree' gui/ app/` 零结果
   - `rg 'PanelTree' gui/ app/` 零结果
3. **结构验证**:`find gui/src -type f -name '*.rs' | sort` 与目标文件结构逐项对照
4. **Demo 手测**:启动 `cargo run -p nodeimg-app --release`,验证:
   - 面板(toolbar、properties、preview)正常显示
   - 按钮点击、滑块拖拽、toggle 切换、dropdown 展开交互与阶段 A 结束时一致
   - 面板拖拽、面板 resize 交互与阶段 A 结束时一致
   - 画布背景、节点卡片渲染正常
5. **git 历史验证**:`git log --follow gui/src/tree/tree.rs` 能追到 `panel/tree/tree.rs` 的历史

## 预估工作量和交付形态

- **一次 session 完成**(避免半迁移状态)
- **单个 commit**(纯重构,原子性更重要)
- 实际工作量比大纲原估的 4-7 天小很多,因为影响面只有约 10-15 个文件

## 阶段 B 完成后

- `gui/src/tree/` 在顶层,包含树的全部基础设施
- `widget/` 只有控件定义(props、atoms/、state、action、focus、text_edit、mapping)
- `panel/` 还在,但不再包含 tree 实现(`panel/tree/` 已删除)
- `canvas/` 不再有空骨架 `canvas/tree/`
- `PanelTree` 名字消失,统一为 `Tree`
- Canvas 分支(阶段 C)的实现可以直接复用 `crate::tree::*`,无需再做模块搬迁
- 阶段 C 的"消除 PanelLayer"有了完整的技术前置条件

## 后续阶段衔接

阶段 C(消除 PanelLayer / 实现 canvas 分支)基于本阶段的成果开始。阶段 C 会:

1. 在同一个 `Tree` 上用 CanvasRoot / PanelRoot 两个分支承载画布和面板内容
2. 把 PanelLayer/PanelFrame 消除,面板变成 `tree::Tree` 里的特定框架控件节点
3. 引入 `widget/frameworks/panel.rs` 作为面板模板

阶段 C 的细节不在本 spec 范围内,见 `docs/superpowers/plans/2026-04-10-phase-c-panel-system-unification.md`。
