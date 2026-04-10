# 阶段 C 面板系统整合计划大纲

> **状态**：待实施。本文件是恢复工作上下文的大纲，不是分步执行计划。真正执行前需要用 superpowers:brainstorming + superpowers:writing-plans 技能细化（阶段 C 涉及多个设计决策，必须先走完整的 brainstorm）。

**Goal:** 消除 `PanelLayer` / `PanelFrame` / `PanelRenderer` 独立面板系统，让面板变成树里的特定框架控件（Framework Widget）。同时实现 canvas 分支，让 `Context` 持有一棵统一的树。

**Architecture:** 架构级整合。把当前"面板系统 + 控件树"的双重架构合并为"一棵树 + 两个分支（PanelRoot / CanvasRoot）"。面板位置、大小、z-order 成为 widget 节点的属性，通过手势系统响应拖拽/resize。camera 作为 CanvasRoot 节点的 transform 属性。

**Tech Stack:** Rust widget 系统，gesture arena，Flexbox + Absolute 布局，trait 对象。

**Spec:** 本阶段**尚无 spec**。开始执行前必须先通过 `superpowers:brainstorming` 技能走完整的设计讨论。这份大纲只是先记录已有的讨论结论，让未来的 brainstorm 不必从零开始。

---

## 前置条件

- 阶段 A 完成 ✓（commit `5c08791`）
- 阶段 B 完成（`tree/` 在顶层，`widget/layout/` 迁移到 `tree/layout/`）

阶段 C 必须等阶段 B 完成后再做，因为 canvas 分支的实现要直接复用 `crate::tree::*`。

## 背景

本 session 的讨论过程中，多轮分析得出的结论：

### 问题：面板不是控件，是独立系统

当前代码里：

- `PanelFrame` 存面板几何（x, y, w, h, visible），定义在 `gui/src/panel/frame.rs`
- `PanelLayer` 存所有面板的几何列表，管理 z-order
- `PanelRenderer` 存**一棵**控件树（PanelTree），负责 reconcile/layout/paint
- `Context` 持有三个独立字段：`canvas: CanvasRenderer`, `layer: PanelLayer`, `panel: PanelRenderer`

这个设计的**根本问题**是：`PanelRenderer` 只有**一棵**树。如果有两个面板（Toolbar + Properties），它们的控件内容没办法分别放进两棵树里。当前 demo 只用一个面板就是这个限制的直接后果。

拖拽面板调用 `panel/drag.rs::apply_drag_move`，它修改 `PanelFrame.x/y`，和控件树完全无关。resize 也是类似的独立代码路径（`panel/resize.rs`）。hit test 分两步：先 `hit_test_panel` 查面板层，再 `panel.hit_test` 查控件树。

### 目标：一棵树 + 两个分支

新架构：

```
Context {
    tree: Tree,   // 一棵统一的树
    camera: Camera,
    gesture_arena: Option<GestureArena>,
}

Tree:
Root — Container
├── CanvasRoot — Container(Transform: camera)    先绘制，底层
│   ├── background — Leaf(Grid)
│   ├── Nodes — Container
│   │   ├── NodeView A — Widget(NodeCardProps, Absolute)
│   │   └── NodeView B — Widget(NodeCardProps, Absolute)
│   └── Connections — Container
│       └── Leaf(Connection { from, to })
│
└── PanelRoot — Container                         后绘制，上层
    ├── toolbar — Widget(PanelProps, Absolute)
    ├── properties — Widget(PanelProps, Absolute)
    └── preview — Widget(PanelProps, Absolute)
```

**关键点**：

- 面板是 `Widget(PanelProps)` 节点，和 Button/Slider 同层级
- 面板的位置挂在 `PanelNode.style.position: Position::Absolute { x, y }`
- 面板的大小挂在 `PanelNode.style.width/height: Size::Fixed(_)`
- 面板的拖拽/resize 由 `gestures: vec![Drag, Resize]` 声明
- z-order 由 `PanelRoot` 的 children 顺序决定，点击面板把它移到末尾
- Camera 是 `CanvasRoot` 节点的 `style.transform` 属性

## 非目标

- **不引入 Absolute 定位之外的新布局模式**
- **不实现动画系统** —— 虽然 Transform 字段在阶段 A 已加，但动画需要单独的插值引擎
- **不实现 Path 图元的绘制** —— 画布的自定义路径留到另一个阶段
- **不接入 IME** —— 文字输入的 IME 支持是独立的大工程
- **不改 Renderer 下层代码** —— 所有 GPU 管线都不动

## 拆解：C 阶段的子任务

阶段 C 太大，不能作为单个 plan 执行。需要拆成若干子阶段：

### C.1 布局引擎支持 Position::Absolute

**工作**：`tree/layout/arrange.rs` 加入对 `Position::Absolute { x, y }` 的处理。遇到 Absolute 子节点时，跳过 Flexbox 流程，直接把 `rect` 设为 `(parent_rect.x + x, parent_rect.y + y, ...)`。Absolute 子节点不占用父容器的 flex 空间。

**文件**：
- `gui/src/tree/layout/arrange.rs`

**预估**：1-2 天。需要正确处理嵌套 Absolute、min/max 约束、margin 等。写单元测试验证基本情况。

**依赖**：无（可以独立做）。

### C.2 hit test 支持 hittable / gestures / hit chain

**工作**：
- `tree/hit.rs` 从返回 `Option<&str>`（单个 widget id）改为返回 `HitChain`（从叶子到根的 NodeId 列表）
- 消费 `BoxStyle.hittable` 三态（None 沿用旧判据，Some(true) 强制可命中，Some(false) 强制不可命中）
- 反向遍历子节点（后渲染的先检查），实现正确的 z-order 命中
- 考虑 scroll 容器的裁剪（阶段 A 未处理）
- 考虑 Transform 节点的坐标逆变换（为 C.5 canvas 服务）

**文件**：
- `gui/src/tree/hit.rs`
- 可能新增 `gui/src/tree/hit_chain.rs` 定义 HitChain 类型

**预估**：2-3 天。hit chain 是后续 gesture 重构的基础。

**依赖**：C.1（Absolute 节点需要被 hit test 正确处理）。

### C.3 Resize 手势识别器

**工作**：
- 新增 `gui/src/gesture/resize.rs` 实现 `ResizeRecognizer`
- 在 `gui/src/gesture/kind.rs` 的 `Gesture` 枚举加 `Resize` 变体
- 实现逻辑：命中节点边缘（6px 内）→ 进入 resize 模式 → 按拖拽距离修改父节点（Panel widget）的 `width`/`height`/`position.x`/`position.y`

**文件**：
- `gui/src/gesture/resize.rs`（新）
- `gui/src/gesture/kind.rs`
- `gui/src/gesture/mod.rs`

**预估**：2-3 天。需要支持 8 个方向的 resize（Top/Bottom/Left/Right/TopLeft/TopRight/BottomLeft/BottomRight），参考现有的 `panel/resize.rs`。

**依赖**：C.2（需要知道命中了哪个节点的边缘）。

### C.4 Panel 框架控件模板

**工作**：
- 新建 `gui/src/widget/frameworks/panel.rs` 定义 `PanelProps`
- 实现 `WidgetProps` trait
- `build()` 返回：
  - 外层 Container（Absolute 定位 + shadow/border/radius decoration）
  - TitleBar 子树（composite widget，暂时可以直接写 Container + Text + Button）
  - Content 容器（接受外部注入的 `Vec<Desc>`）
- Props 结构：
  ```rust
  pub struct PanelProps {
      pub title: Cow<'static, str>,
      pub x: f32,  // 初始位置
      pub y: f32,
      pub w: f32,  // 初始大小
      pub h: f32,
      pub visible: bool,
      pub content: Vec<Desc>,  // 内容槽
  }
  ```

**关键设计问题**：面板的**运行时状态**（当前 x, y, w, h）要跨帧保留，但 reconcile 会根据 Props 重新构建节点。这里有一个状态语义问题：

- 选项 A：reconcile 时如果节点已存在，**不覆盖 rect 字段**（只更新 Props 相关部分）。面板初次创建用 Props.x/y/w/h，之后由拖拽/resize 修改 rect，再 reconcile 时保留。
- 选项 B：面板状态在 UIState 里，每帧 view() 从 UIState 读 x/y/w/h 构造 Props。
- 选项 C：面板 widget 在 build() 里保留对自身节点 rect 的引用（通过某种间接方式）。

**本 session 中用户已决定**：面板几何状态**挂在 widget tree 节点上**（选项 A）。所以 reconcile 必须有机制"不覆盖某些运行时字段"。

**文件**：
- `gui/src/widget/frameworks/panel.rs`（新）
- `gui/src/widget/frameworks/mod.rs`（新）
- `gui/src/widget/mod.rs`
- `gui/src/tree/diff.rs`（reconcile 逻辑加上"保留运行时字段"的处理）

**预估**：3-5 天。设计 reconcile 保留字段的机制是难点。

**依赖**：C.1（Absolute 定位）、C.3（Resize 手势）。

### C.5 Canvas 分支基础实现

**工作**：
- `gui/src/canvas/tree/` 目录目前几乎空白（只有几个占位文件）。阶段 C.5 要实现 canvas 分支的具体内容。
- 实际上，在新架构里 canvas 不需要独立的 tree 实现——它用同一棵 tree，只是在 CanvasRoot 节点上挂 Transform 属性。
- 需要做的：
  1. 实现 `LeafKind::Grid` 的 paint（从 `canvas/background.rs` 迁移逻辑）
  2. 实现 `LeafKind::Connection` 的 paint（贝塞尔曲线 + 端口位置查找）
  3. 实现 `Transform` 在 paint/hit test 中的应用（CanvasRoot 节点的 camera）
  4. 新建 `NodeCardProps` 特定框架控件（类似 PanelProps，但在世界坐标系）

**文件**：
- `gui/src/tree/paint.rs`（扩展 LeafKind 分发）
- `gui/src/canvas/background.rs`（函数被 paint.rs 调用）
- `gui/src/widget/frameworks/node_card.rs`（新）

**预估**：5-8 天。Connection 的端口位置查找是难点（需要从 tree 里按 id 查找节点）。Transform 在 hit test 里的坐标逆变换是另一个难点。

**依赖**：C.1、C.2、C.4。

### C.6 Context 重构

**工作**：
- 当前 `Context { canvas: CanvasRenderer, layer: PanelLayer, panel: PanelRenderer }`
- 改为 `Context { tree: Tree, gesture_arena: Option<GestureArena>, /* 没有 canvas/layer/panel */ }`
- 相机 (`Camera`) 作为 `CanvasRoot` 节点的 `style.transform` 字段存储。`Context` 本身不再持有 `Camera`——它是树里某个节点的属性。
- 添加 `Context::update(desc: Desc)` 方法，驱动整棵树的 reconcile
- 添加 `Context::hit_test(x, y) -> HitResult` 方法，统一入口
- 添加 `Context::render(renderer, viewport)` 方法，统一绘制入口

**文件**：
- `gui/src/context.rs`
- `gui/src/canvas/renderer.rs`（整合到 Context，最终删除）

**预估**：2-3 天。结构改动不大，但是 API 破坏性改动，需要跟 demo.rs 的重写同步。

**依赖**：C.5（canvas 分支能工作）。

### C.7 demo.rs 重写

**工作**：
- 当前 `app/src/demo.rs` 用命令式 API：`gui.layer.add(PanelFrame::new(...))`、`hit_test_panel`、`detect_edge`、`apply_drag_move` 等
- 改为声明式：
  - `init()` 不再 add panel frame，而是初始化 `Context::new()`
  - 每帧 `render()` 构造整棵 `Desc` 树（包含 PanelRoot 下的 toolbar/properties/preview widgets），调 `gui.update(desc)`
  - `event()` 不再手动做面板 hit test 和 resize 边缘检测，只调 `gui.hit_test(x, y)` 和 gesture arena
- 视觉结果：与当前 demo 一致（一个面板，内部有 Button A/B + Slider）

**文件**：
- `app/src/demo.rs`
- `app/src/main.rs`（可能）

**预估**：3-5 天。demo.rs 当前 229 行，基本全要重写。

**依赖**：C.6。

### C.8 删除 PanelLayer / PanelFrame / panel 独立系统

**工作**：

删除以下文件：
- `gui/src/panel/frame.rs`
- `gui/src/panel/layer.rs`
- `gui/src/panel/drag.rs`
- `gui/src/panel/resize.rs`
- `gui/src/panel/hit.rs`
- `gui/src/panel/renderer.rs`

保留/改造：
- `gui/src/panel/mod.rs` 变成只声明面板实例子模块
- `gui/src/panel/instances/` 变成 `gui/src/panel/` 下的平铺文件（toolbar.rs、properties.rs、preview.rs）
- 每个面板实例是一个 `fn toolbar(state: &ToolbarState) -> Desc { panel("工具栏", vec![...]) }` 形式的函数

**register_panel! 宏**（用户已决定保留）：
- 新位置：`gui/src/panel/mod.rs` 或独立宏 crate
- 展开为面板实例构造函数 + 状态初始化 + 事件路由

**文件**：
- 删除：上面列的 6 个文件
- 改造：`gui/src/panel/mod.rs`
- 新建/移动：面板实例文件

**预估**：2-3 天。大部分是删除工作，需要确认没有代码还在引用这些文件。

**依赖**：C.7（demo 不再使用这些 API）。

## 总工作量估算

| 子阶段 | 工作内容 | 预估 |
|---|---|---|
| C.1 | Absolute 布局 | 1-2 天 |
| C.2 | hit chain + hittable/gestures | 2-3 天 |
| C.3 | Resize 手势识别器 | 2-3 天 |
| C.4 | PanelProps 框架控件 | 3-5 天 |
| C.5 | Canvas 分支实现 | 5-8 天 |
| C.6 | Context 重构 | 2-3 天 |
| C.7 | demo.rs 重写 | 3-5 天 |
| C.8 | 删除 PanelLayer 系统 | 2-3 天 |
| **总计** | | **20-32 天** |

这是个大工程。建议：

- 每个子阶段走完整的 brainstorm → spec → plan 流程
- 每个子阶段独立 commit，不合并成一个大 commit
- C.1 和 C.2 可以并行（它们依赖不同）
- C.5 最大，可能需要拆成 C.5.a (Grid paint)、C.5.b (Connection paint)、C.5.c (NodeCard widget) 三个子项
- C.7 和 C.8 紧密耦合，应该放在最后连续做

## 高风险点

### 🔴 reconcile 保留运行时字段

这是阶段 C 最微妙的设计问题。面板被拖动后，它的 `rect.x` 变了。下一帧 view() 返回的 Desc 里 PanelProps 还是初始值（比如 Props.x = 100），reconcile 看到 Props 改变会不会重置 rect？

**正确语义应该是**：
- Props 里的 x/y 只用于节点首次创建时的初始位置
- 节点存在后，Props.x/y 的变化不影响现有 rect（或者需要显式标记 "initial_only"）
- 拖拽修改的是 node 的 `style.position` 字段，直接绕过 Props

这要求 reconcile 有"首次创建"和"更新"两种语义的区分，或者 Props 分"初始值"和"受控值"。**设计上待定**。

### 🔴 demo.rs API 破坏不可逆

demo.rs 的改写没法增量进行——要么是旧 API 全套，要么是新 API 全套。建议在独立 feature 分支做，不要一边迁移一边保持两套 API。

### 🟡 canvas Grid 的 camera 依赖

`LeafKind::Grid` 的 paint 需要知道当前视口在世界坐标中的范围（才能决定画哪些点）。这要求 paint 遍历时能拿到当前的 Transform 矩阵。

**方案**：paint 遍历时维护一个 "current transform" 栈。进入 CanvasRoot 时 push camera，离开时 pop。Grid 的 paint 函数读取栈顶的 transform 推导出世界坐标可视区域。

### 🟡 Connection 的跨节点引用

`LeafKind::Connection { from_port, to_port }` 的 paint 需要查找那两个 port 节点的 rect。这要求 paint 函数能访问整棵 tree。

**方案**：paint 函数签名改成 `fn paint(tree: &Tree, node_id: NodeId, renderer: &mut Renderer, ...)`，这样 paint 内可以 `tree.lookup_by_id(port_id)` 查找任意节点。

### 🟡 hit test 的坐标变换

统一 hit test 进入 CanvasRoot 时需要对 (x, y) 做相机的逆变换。如果 canvas 内部再嵌套 transform（比如某个节点卡片有自己的 rotation），需要正确的矩阵乘法。

**方案**：hit test 函数维护一个 "current transform inverse" 栈，逐层下降时应用逆变换。

### 🟢 Resize 手势的边缘检测

参考现有 `panel/resize.rs::detect_edge` 的 6px 阈值，集成到新的 ResizeRecognizer 里。命中测试先返回最深的 widget，再由 resize recognizer 检查是否在边缘 6px 内。

## brainstorm 和 plan 文件命名建议

执行阶段 C 时，每个子阶段独立 brainstorm/spec/plan：

- `docs/superpowers/specs/2026-XX-XX-c1-absolute-layout-design.md`
- `docs/superpowers/specs/2026-XX-XX-c2-hit-chain-design.md`
- `docs/superpowers/specs/2026-XX-XX-c3-resize-gesture-design.md`
- `docs/superpowers/specs/2026-XX-XX-c4-panel-widget-design.md`
- `docs/superpowers/specs/2026-XX-XX-c5-canvas-branch-design.md`
- `docs/superpowers/specs/2026-XX-XX-c6-context-refactor-design.md`
- `docs/superpowers/specs/2026-XX-XX-c7-demo-rewrite-design.md`
- `docs/superpowers/specs/2026-XX-XX-c8-panellayer-removal-design.md`

对应的 plan 文件放在 `docs/superpowers/plans/` 下同日期前缀。

## 成功判据（阶段 C 全部完成后）

- [ ] `PanelLayer`、`PanelFrame`、`PanelRenderer`、`panel/drag.rs`、`panel/resize.rs`、`panel/hit.rs` 全部删除
- [ ] `Context` 结构体只有 `tree` 和 `gesture_arena`（camera 作为节点属性）
- [ ] `Position::Absolute` 布局生效
- [ ] `Transform` 在 paint/hit test 中生效（camera）
- [ ] hit test 返回 hit chain，gesture arena 按链式分发
- [ ] Resize 作为 Gesture 枚举变体存在，有对应 ResizeRecognizer
- [ ] `widget/frameworks/panel.rs` 定义 PanelProps
- [ ] `widget/frameworks/node_card.rs` 定义 NodeCardProps
- [ ] `LeafKind::Grid` 能实际渲染
- [ ] `LeafKind::Connection` 能实际渲染
- [ ] `demo.rs` 用声明式 API，功能与阶段 A 结束时一致
- [ ] `cargo clippy -p gui -- -D warnings` 通过
- [ ] `cargo build -p app --release` 成功
- [ ] 手测 demo 所有交互正常

## 关键决策记录（本 session 已确定）

1. **一棵树两分支**：CanvasRoot 和 PanelRoot 是 Root 的两个直接子节点
2. **CanvasRoot 先 PanelRoot 后**：保证面板画在画布之上
3. **面板几何状态挂在 widget tree 节点上**（非 UIState）—— 用户选择
4. **register_panel! 宏保留** —— 用户选择
5. **图元可以直接用，不经图元控件**（取消图元控件层级）—— 这是阶段 A 已经实施的决定
6. **Resize 不在阶段 A 的 Gesture 枚举里** —— 等 C.3 再加
7. **Connection port id 用 `Cow<'static, str>`** —— 阶段 A 已定
8. **TextureHandle 是不透明 u64 句柄** —— 阶段 A 已定
9. **CustomPaintFn 用 Arc<dyn CustomPainter> + ptr 比较** —— 阶段 A 已定
10. **Camera 是 Container 的 transform 属性，不是独立概念** —— 阶段 C 设计决定

这些决策避免在阶段 C 的 brainstorm 里重新讨论。

## 准备动作清单

回到这份文档时，按以下顺序开工：

1. [ ] 先确认阶段 B 已完成（`tree/` 在顶层）
2. [ ] 先做 C.1（Absolute 布局）——独立且风险低
3. [ ] 再做 C.2（hit chain）——给 C.3/C.4/C.5 打基础
4. [ ] 之后按顺序 C.3 → C.4 → C.5 → C.6 → C.7 → C.8

每个子阶段进入前用 `/brainstorming` 走完整流程。

## 备注

- 本文件不是可执行 plan，是**上下文恢复文档**。当你回来做阶段 C 时，先读这份文件，再读 `docs/target/2.14.0-concepts.md` 和 `docs/target/2.0.0-gui.md` §5，然后用 brainstorming 技能细化具体子阶段
- 本 session 讨论的完整过程在 git log 里有记录（搜索 `docs/superpowers/specs/` 和 `docs/superpowers/plans/` 的 commit）
- 阶段 A 完成的 commit 是 `5c08791`
