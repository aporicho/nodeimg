# 阶段 C.5 设计：paint 扩展 —— Grid / Connection / Transform

## 背景

阶段 C.1+C.2+C.3（commit `a0efe8d`）完成了布局、命中、手势基础设施，C.4（commit `33e3d3f`）建好了第一个复合 widget `PanelProps`。但是画布（canvas）功能目前还运行在独立的 `canvas/` 模块里（`canvas/background.rs` 直接调用 `Renderer::draw_circle` 画网格），没有纳入统一的 tree 架构。

`tree/paint.rs` 目前只有 39 行，只处理 `LeafKind::Text` + Container decoration（background/border/radius/shadow）。`types.rs` 里定义了 10 种 `LeafKind`，除 Text 外 9 种都还没有 paint 实现。`BoxStyle.transform` 字段在阶段 A 就引入了，但 paint 完全没消费它——C.2 只做了 hit test 侧的逆变换，paint 侧的正向变换到现在才做。

C.5 的任务是把画布需要的三项基础能力补到 tree 的 paint 子系统里：**`LeafKind::Grid` 绘制**、**`LeafKind::Connection` 绘制**、以及**Transform 在 paint 中的累积与应用**。做完之后，tree 能完整画出"一棵树 + Transform 相机 + 节点 + 连线"的 canvas 场景，为 C.6（Context 重构）提供画布端的绘制能力。

C.6（Context 重构）/ C.7（demo 重写）/ C.8（删除旧 panel）不在本 spec 范围。本 spec 只做"库能力"的扩展，demo 不动，`canvas/background.rs` 不动，`panel/renderer.rs` 只在 paint_node 签名改动时做最小适配。

## 目标与非目标

### 目标

1. **Transform 累积机制**：定义 `PaintTransform` 类型（封装累积的 translate + scale），改造 `tree/paint.rs::paint_node` 接收它作为递归参数。遇到 `node.style.transform: Some(_)` 时，复合出新的 `PaintTransform` 传给子节点。
2. **Grid 绘制**：`LeafKind::Grid { spacing, dot_color, dot_size }` 在 `paint_node` 中按 rect 边界遍历格点，应用累积变换得 screen 坐标，调 `Renderer::draw_circle` 逐点绘制。`dot_size` 也按 scale 缩放。
3. **Connection 绘制**：`LeafKind::Connection { from_port, to_port }` 在 `paint_node` 中线性遍历 tree 查找 id 匹配的 port 节点，取 `center_right` / `center_left` 锚点，计算三次贝塞尔 4 控制点（水平偏移启发式），调 `Renderer::draw_curve`。宽度和颜色硬编码为模块常量。
4. **纯函数提取**：把可测试的数学/查找逻辑提取到 `tree/paint_helpers.rs`（新建文件），实现 ~15 个单元测试。
5. **paint_node 签名改动**：从 `fn paint_node(tree, id, renderer)` 改为 `fn paint_node(tree, id, renderer, tf: PaintTransform)`。外部入口 `fn paint(tree, root, renderer)` 签名不变，内部用 `PaintTransform::identity()` 起始。
6. **零 demo 改动**：`app/src/demo.rs` 不修改。`canvas/background.rs` 不修改（demo 还靠它画网格）。`panel/renderer.rs::paint()` 继续走 `tree::paint(..)`，内部适配完 paint_node 新签名即可。

### 非目标

- **不做 NodeCardProps widget** — 类似 `PanelProps` 的"节点卡片"复合控件留给 C.5.5 或 C.7。
- **不做 Transform 的 rotate 应用** — `Transform.rotate` 字段保留（和 C.2 的逆变换对称），但 paint 不应用。`Renderer::draw_rect` 只支持轴对齐矩形，旋转需要额外的图元路径。
- **不清理 `canvas/background.rs`** — demo 还在用它画网格。C.7 重写 demo 时再删。
- **不扩展 `LeafKind`** — 不改 types.rs 里的 LeafKind 变体定义。Connection 的 width/color 用模块常量，不加字段。
- **不实现 Image / Icon / Path / CustomPaint 的 paint** — 本 spec 只做 Grid 和 Connection。其余 7 种 LeafKind 留给未来按需实现。
- **不改 `Renderer`** — 不动 `gui/src/renderer/**` 下任何文件。不引入 DrawSink trait。
- **不优化 Grid 大 rect 场景** — grid_cells 在"大 rect + 小 spacing"下会产生很多点。C.5 不做裁剪，只保证基本功能。裁剪优化留给实际遇到性能问题时。
- **不做 Connection 的 port 查找缓存** — 线性遍历每帧每连线一次。小 demo 可接受。未来可加索引。
- **不改 reconcile / layout / hit_test** — 只动 paint 子系统。C.1~C.4 的测试全部保留 PASS。

## 指导原则

1. **扩展 paint，不重构** — 只给 paint.rs 加能力（新的 LeafKind 分支、Transform 处理），不改 paint 的整体调用模式。paint_node 仍然是递归下行的。
2. **纯函数提取优先** — 所有数学计算（compose、apply、grid iteration、bezier）从 paint_node 主体剥离，放进 `paint_helpers.rs`。主文件只做"读 tree、分发 LeafKind、调 renderer"的胶水工作。
3. **硬编码常量代替配置** — Connection 宽度/颜色、bezier 水平偏移因子等都用模块常量。未来有需求再暴露为 props 或主题。
4. **与 C.2 对称** — C.2 已经实现了 `inverse_transform`（屏幕→local），本 spec 实现的 `apply_*` 是正向（local→屏幕）。两者数学对称，都只处理 translate + scale。
5. **零 demo 破坏** — demo 行为不变。panel/renderer.rs 的外部 API 保持兼容。
6. **测试优先** — 所有纯函数单元测试覆盖。paint_node 本身通过 dry run（构造各种 tree 调用 paint 不 panic）做 smoke test。

## 决策记录

### 决策 1 · PaintTransform 作为独立类型 vs 直接传 `&Transform`

**决定**：定义新类型 `PaintTransform { tx: f32, ty: f32, scale: f32 }` 作为 paint_node 的递归参数，不复用 `tree::layout::types::Transform`。

**替代方案**：
- 直接把 `Transform { translate: [f32;2], scale: f32, rotate: f32 }` 作为 paint_node 参数。
- 用三个 f32 参数（tx, ty, scale）传递，不封装。

**理由**：
- `Transform` 是节点声明的"自身变换"（e.g., CanvasRoot 的 camera）；`PaintTransform` 是"从 root 到当前节点的累积变换"。语义上是两个不同的东西，用不同类型能避免混淆。
- `PaintTransform` 明确不持 rotate 字段，提醒代码作者和读者"paint 只处理 translate + scale"。
- 三个 f32 参数虽然简单，但函数签名会变长且容易传错顺序（swap tx/ty）。封装成类型更安全。
- 类型有自己的方法（identity / compose / apply_rect / apply_point），API 集中，测试也集中。

### 决策 2 · Transform 复合的数学顺序

**决定**：`PaintTransform::compose(&self, child_tf: &Transform) -> PaintTransform` 的语义是"先应用父累积变换，再应用子节点自身的 transform"。具体公式：

```
new.tx = self.tx + self.scale * child_tf.translate[0]
new.ty = self.ty + self.scale * child_tf.translate[1]
new.scale = self.scale * child_tf.scale
```

`apply_point(p)` = `(self.tx + self.scale * p.x, self.ty + self.scale * p.y)`。

**替代方案**：
- 相反顺序：先应用子 transform，再应用父累积。语义变成"子节点在父的局部坐标系里先做 transform"。
- 使用 3×3 矩阵做完整仿射复合。

**理由**：
- 相反顺序会让直觉错乱——父节点的 scale 应该放大子节点 transform 的 translate 距离（想象相机 zoom in 后，子节点的"移动量"在屏幕上看起来更大）。
- 3×3 矩阵能表达 rotate，但本 spec 明确不做 rotate。矩阵带来的抽象成本得不偿失。
- 本 spec 的公式和 C.2 `inverse_transform` 是严格对称的逆运算：
  - Forward (C.5)：`screen = self.tx + self.scale * local`
  - Inverse (C.2)：`local = (screen - self.tx) / self.scale`
  - 两者复合后是 identity，对称性强，bug 少。

### 决策 3 · Connection 的端口锚点选择

**决定**：Connection 的 from 锚点是 from_port 节点 rect 的 **center_right**（右边中点），to 锚点是 to_port 节点 rect 的 **center_left**（左边中点）。硬编码，不可配置。

**替代方案**：
- 锚点居中（`rect.center()`）。
- 锚点由 Connection 额外的字段指定（e.g., `from_anchor: Anchor`）。
- 锚点由 port 节点自身的某种 meta 字段指定。

**理由**：
- 节点图 UI 的惯例：连线从源节点右侧出发，进入目标节点左侧。这是 100% 的场景覆盖。
- `rect.center()` 会让连线"穿过"端口节点本身，视觉混乱。
- 暴露为字段需要 LeafKind::Connection 的 struct 变更 + props_eq 兼容考虑，scope 膨胀。
- 未来如果需要方向可配置（e.g., 垂直图），再加 anchor 字段。YAGNI。

### 决策 4 · 贝塞尔控制点的启发式

**决定**：三次贝塞尔的 4 个控制点按如下公式计算：

```
p0 = from
p3 = to
dx = (to.x - from.x).abs()
p1 = (from.x + dx * 0.5, from.y)
p2 = (to.x - dx * 0.5, to.y)
```

水平偏移量 = 两点横向距离的一半。这让连线呈现 S 形/C 形的自然曲线。

**替代方案**：
- 固定像素偏移（e.g., 60px）。两点距离过近时曲线会反弯。
- 斜率感知的复杂启发式。过度工程。
- 直线（不用贝塞尔）。视觉丑陋。

**理由**：
- 行业主流（Blender、Unreal Blueprints、各种节点编辑器）都用"距离一半"风格的启发式，用户审美符合预期。
- 纯函数好测：输入两点，输出 4 个 Point，断言即可。
- 用 abs() 保证 from.x > to.x（反向连线）也能产生合理曲线。

### 决策 5 · Connection 的 port 查找策略

**决定**：定义 `find_node_by_str_id(tree, id: &str) -> Option<Rect>`，从 root 开始深度优先遍历整棵树找 id 匹配的节点，返回其 rect（如果找到）。每次 paint Connection 都调用这个函数（O(n) per connection）。

**替代方案**：
- paint 入口先建一个 `HashMap<String, Rect>` 索引，paint_connection 时 O(1) 查。
- 在 reconcile / layout 阶段预写 port 位置到某个全局注册表。

**理由**：
- 小 demo（~30 节点 ~10 连线）下，`10 * 30 = 300` 次字符串比较/帧，远低于任何可感知的开销。
- HashMap 索引需要 paint 入口多走一次 tree 遍历，且需要传递新参数到 paint_node，增加复杂度。当前没有性能问题，YAGNI。
- 注册表需要生命周期管理（连线失效时清理条目），scope 蔓延。
- 如果未来遇到大规模节点图的性能问题，可以引入 `PaintContext` 结构体封装 tree + 索引，paint_node 接收 `&PaintContext` 而非裸 tree。那是"解决实际问题时"的事。

### 决策 6 · 找不到 port 时的行为

**决定**：`find_node_by_str_id` 返回 `None` 时，`paint_connection` **静默跳过**这条 connection，不画。可以在 debug 模式下打 log（非 spec 强制）。

**替代方案**：
- panic！（开发阶段显式失败）
- 画一条占位曲线（e.g., 红色从 (0,0) 到 (100,100)）提示错误。
- 上浮到调用方让 demo 处理。

**理由**：
- panic 在运行时图更新过程中很容易触发——比如删除节点后连线还没同步更新，画面直接崩溃不可接受。
- 占位曲线会让调试更困难（用户以为是一个真实连线出错了）。
- 静默跳过 + debug log 是最稳妥的选择，符合 GUI 代码的防御性原则。demo 层面应该保证不会构造悬挂连线，但 paint 层不强制这个约束。

### 决策 7 · Grid 的 dot_size 也按 scale 缩放

**决定**：`paint_grid` 在调 `Renderer::draw_circle` 时传入的 radius 是 `dot_size * tf.scale`。也就是说，相机 zoom in 后，grid 点也会跟着变大。

**替代方案**：
- 保持 dot_size 不变（屏幕像素，不随 zoom 变）。
- 提供一个 flag 让调用方选择。

**理由**：
- grid 点和 spacing 在同一个 local 坐标系里，spacing 随 zoom 缩放是必然的（否则点之间距离不对）。dot_size 也随 zoom 缩放保持视觉比例一致。
- 若 dot_size 不变，zoom in 时点会显得越来越小（相对于变大的 spacing），视觉不协调。
- 不需要提供 flag——这是 C.5 最小范围的硬编码行为。未来有需求再扩展。

### 决策 8 · 把纯函数放在独立文件 `paint_helpers.rs`

**决定**：新建 `gui/src/tree/paint_helpers.rs`，里面定义 `PaintTransform` 类型、所有纯函数（`grid_cells`、`bezier_control_points`、`find_node_by_str_id`、`rect_anchor_*`）和它们的单元测试。`paint.rs` 只保留"读 tree、分发 LeafKind、调 renderer"的胶水代码。

**替代方案**：
- 全部塞进 `paint.rs`，保持单文件。
- 把 `PaintTransform` 放进 `tree/layout/types.rs`（和 `Transform` 同文件）。

**理由**：
- 单文件会让 paint.rs 变得很长（>300 行），违反"一个文件一个职责"的约定。paint.rs 的职责是"从 tree 到 renderer 调用序列"的胶水，helpers 的职责是"可独立验证的数学/查找"，两个职责分文件更清晰。
- `PaintTransform` 是 paint 子系统的内部类型（不是 tree/layout 公用的），放 layout/types.rs 语义不对。
- `paint_helpers.rs` 是 `pub(super)` 或 crate-private 模块——外部不直接用，但 paint.rs 和测试都能访问。

## 架构

### 本 spec 变更的文件

| 路径 | 改动类型 | 说明 |
|------|---------|------|
| `gui/src/tree/paint_helpers.rs` | **新建** | `PaintTransform` 类型、纯函数、单元测试 |
| `gui/src/tree/paint.rs` | 修改 | 扩展 paint_node：Transform 累积、Grid/Connection 分支；签名加 `tf: PaintTransform` 参数 |
| `gui/src/tree/mod.rs` | 修改 | 加 `mod paint_helpers;` |

本 spec **零改动**的文件：

- `app/src/demo.rs`
- `gui/src/renderer/**`
- `gui/src/canvas/**`（包括 background.rs、renderer.rs、camera.rs 等）
- `gui/src/panel/**`（包括 panel/renderer.rs 如果它没有直接调 paint_node 的话）
- `gui/src/tree/layout/**`
- `gui/src/tree/hit.rs`、`desc.rs`、`diff.rs`、`tree.rs`、`node.rs`、`scroll.rs`、`paint.rs` 之外的 tree 子模块
- `gui/src/widget/**`
- `gui/src/gesture/**`

**注意**：如果 `panel/renderer.rs::paint()` 直接调 `tree::paint(&self.tree, root, renderer)`（外部 API 保持），那 panel/renderer.rs 不需要改。否则需要小改以适配 paint_node 新签名。实施者在 Task 1 开头读 panel/renderer.rs 确认。

### PaintTransform API

```rust
// gui/src/tree/paint_helpers.rs

use crate::renderer::{Point, Rect};
use crate::tree::layout::types::Transform;

/// 累积的 paint 变换。
///
/// 从 root 向下递归时累积的 translate + scale。rotate 不处理——
/// Renderer 的 draw_* 方法不支持旋转矩形，而 C.2 的 inverse_transform
/// 同样只处理 translate + scale，两者保持对称。
#[derive(Debug, Clone, Copy)]
pub struct PaintTransform {
    pub tx: f32,
    pub ty: f32,
    pub scale: f32,
}

impl PaintTransform {
    /// 恒等变换（paint 入口使用）。
    pub fn identity() -> Self {
        Self { tx: 0.0, ty: 0.0, scale: 1.0 }
    }

    /// 复合：先应用 self（父累积），再应用 child_tf（子节点自身的 Transform）。
    ///
    /// 公式：
    ///   new.tx = self.tx + self.scale * child_tf.translate[0]
    ///   new.ty = self.ty + self.scale * child_tf.translate[1]
    ///   new.scale = self.scale * child_tf.scale
    ///
    /// rotate 字段被忽略（C.5 不支持旋转）。
    pub fn compose(&self, child_tf: &Transform) -> Self {
        Self {
            tx: self.tx + self.scale * child_tf.translate[0],
            ty: self.ty + self.scale * child_tf.translate[1],
            scale: self.scale * child_tf.scale,
        }
    }

    /// 把 local 点变换到 screen 点：screen = tx + scale * local。
    pub fn apply_point(&self, p: Point) -> Point {
        Point {
            x: self.tx + self.scale * p.x,
            y: self.ty + self.scale * p.y,
        }
    }

    /// 把 local rect 变换到 screen rect。w/h 也按 scale 缩放。
    /// 注意：不处理旋转，返回的 rect 仍然是轴对齐的。
    pub fn apply_rect(&self, r: Rect) -> Rect {
        Rect {
            x: self.tx + self.scale * r.x,
            y: self.ty + self.scale * r.y,
            w: self.scale * r.w,
            h: self.scale * r.h,
        }
    }
}
```

### 纯函数 API

```rust
// 同文件 paint_helpers.rs

/// 遍历 rect 内 spacing 为间距的所有格点。
///
/// 从 rect.x, rect.y 开始，每 spacing 像素一个点，直到 rect.x + rect.w。
/// spacing <= 0 时返回空迭代器（避免死循环）。
pub fn grid_cells(rect: Rect, spacing: f32) -> impl Iterator<Item = Point>;

/// 计算两点之间的三次贝塞尔 4 个控制点（水平偏移启发式）。
///
/// p0 = from, p3 = to
/// p1 = (from.x + |dx|/2, from.y)
/// p2 = (to.x - |dx|/2, to.y)
pub fn bezier_control_points(from: Point, to: Point) -> [Point; 4];

/// 深度优先遍历 tree，查找 id 字符串匹配的节点，返回其 rect。
///
/// 找不到返回 None。tree 很大时是 O(n)——C.5 范围内可接受。
pub fn find_node_by_str_id(tree: &Tree, id: &str) -> Option<Rect>;

/// 取 rect 右边中点（默认的 Connection 输出锚点）。
pub fn rect_center_right(r: Rect) -> Point {
    Point { x: r.x + r.w, y: r.y + r.h * 0.5 }
}

/// 取 rect 左边中点（默认的 Connection 输入锚点）。
pub fn rect_center_left(r: Rect) -> Point {
    Point { x: r.x, y: r.y + r.h * 0.5 }
}
```

### paint.rs 改动

```rust
// gui/src/tree/paint.rs（示意，非最终代码）

use crate::renderer::{Color, Point, Rect, Renderer};
use crate::tree::layout::{Decoration, LeafKind, NodeKind};
use crate::tree::node::NodeId;
use crate::tree::paint_helpers::{
    bezier_control_points, find_node_by_str_id, grid_cells, rect_center_left, rect_center_right,
    PaintTransform,
};
use crate::tree::tree::Tree;

// Connection 的硬编码样式（C.7 主题系统时可能替换）
const CONNECTION_WIDTH: f32 = 2.0;
const CONNECTION_COLOR: Color = Color { r: 0.55, g: 0.58, b: 0.65, a: 1.0 };

pub fn paint(tree: &Tree, root: NodeId, renderer: &mut Renderer) {
    paint_node(tree, root, renderer, PaintTransform::identity());
}

fn paint_node(tree: &Tree, id: NodeId, renderer: &mut Renderer, tf: PaintTransform) {
    let Some(node) = tree.get(id) else { return };

    let screen_rect = tf.apply_rect(node.rect);

    // 1. Container decoration
    if let Some(dec) = &node.decoration {
        // ...（复用当前 paint.rs 的 decoration 分支逻辑，把 rect 替换为 screen_rect）
    }

    // 2. Leaf 分发
    if let NodeKind::Leaf(kind) = &node.kind {
        match kind {
            LeafKind::Text { content, font_size, color } => {
                // 文字大小也按 scale 缩放
                let scaled_size = font_size * tf.scale;
                renderer.draw_text(
                    Point { x: screen_rect.x, y: screen_rect.y },
                    content,
                    &TextStyle { size: scaled_size, color: *color },
                );
            }
            LeafKind::Grid { spacing, dot_color, dot_size } => {
                // 用 local rect（node.rect）遍历格点，逐点变换
                for p in grid_cells(node.rect, *spacing) {
                    let sp = tf.apply_point(p);
                    renderer.draw_circle(sp, dot_size * tf.scale, *dot_color);
                }
            }
            LeafKind::Connection { from_port, to_port } => {
                let (Some(from_rect), Some(to_rect)) = (
                    find_node_by_str_id(tree, from_port.as_ref()),
                    find_node_by_str_id(tree, to_port.as_ref()),
                ) else {
                    return;  // 静默跳过
                };
                // 锚点在 local 空间，变换到 screen
                let from_p = tf.apply_point(rect_center_right(from_rect));
                let to_p = tf.apply_point(rect_center_left(to_rect));
                let ctrl = bezier_control_points(from_p, to_p);
                renderer.draw_curve(ctrl, CONNECTION_WIDTH * tf.scale, CONNECTION_COLOR);
            }
            _ => {
                // 其余 LeafKind 暂不实现（Image / Icon / Circle / Line / Curve / Path / CustomPaint）
            }
        }
    }

    // 3. 复合 Transform 并递归子节点
    let child_tf = match &node.style.transform {
        Some(tf_decl) => tf.compose(tf_decl),
        None => tf,
    };
    for &child in &node.children {
        paint_node(tree, child, renderer, child_tf);
    }
}
```

**注意**：
- 上面的代码是示意，具体的 decoration 绘制逻辑要沿用当前 paint.rs 的现有代码，只是把 `node.rect` 替换为 `screen_rect` 作为坐标。
- 当前 paint.rs 是什么样子（参数命名、helper 函数等）以实际代码为准。plan 阶段再精确到每一行。
- `Text` 的字体大小也按 scale 缩放（zoom in 文字变大），与 Grid 的 dot_size 处理一致。

### 命中语义不变

C.2 的 hit_test 已经处理了逆变换，本 spec 不动 hit_test。paint 的正向变换和 hit test 的逆变换是严格对称的，不会产生"点击位置不对齐"的 bug。

## 测试计划

### 测试组织

在 `gui/src/tree/paint_helpers.rs` 文件末尾追加 `#[cfg(test)] mod tests`，覆盖所有纯函数。约 15 个测试。

paint_node 本身不做单元测试（没有 mock renderer），通过 **dry run 测试**确保不 panic：构造几个不同形态的 tree（空 tree、纯 container、带 Grid、带 Connection、带 Transform），调用 `paint` 函数，断言没有 panic。这是 smoke test，不验证 renderer 调用序列。

### 测试清单（~15 个）

#### PaintTransform（5 个）

1. **`transform_identity_compose`** — `identity().compose(&Transform{translate:[10,20], scale:2.0, rotate:0})` 得 `{tx:10, ty:20, scale:2}`
2. **`transform_compose_nested`** — 两层嵌套 compose，验证数学：`identity → {translate:[10,20], scale:2} → {translate:[5,5], scale:3}` 得 `{tx: 10+2*5, ty: 20+2*5, scale: 2*3}` = `{tx:20, ty:30, scale:6}`
3. **`transform_apply_point`** — `PaintTransform{tx:10, ty:20, scale:2}.apply_point((3, 4))` 得 `(16, 28)`
4. **`transform_apply_rect`** — 同上但用 rect，验证 w/h 也按 scale 缩放
5. **`transform_ignores_rotate`** — 传入 rotate != 0 的 Transform compose，验证输出的 PaintTransform 不含 rotate，行为等价于 rotate=0

#### grid_cells（3 个）

6. **`grid_cells_single_point`** — rect=(0,0,0,0), spacing=10 → 应返回 1 个点 (0,0)
7. **`grid_cells_multi`** — rect=(0,0,30,30), spacing=10 → 应返回 16 个点（4×4 网格：0,10,20,30）
8. **`grid_cells_zero_spacing`** — spacing=0 或负数 → 返回空迭代器（避免死循环）

#### bezier_control_points（3 个）

9. **`bezier_horizontal`** — from=(0,0), to=(100,0) → p0=(0,0), p1=(50,0), p2=(50,0), p3=(100,0)
10. **`bezier_diagonal`** — from=(0,0), to=(100,50) → p1=(50,0), p2=(50,50)
11. **`bezier_reverse`** — from=(100,0), to=(0,0) → 验证 abs() 的效果，p1 和 p2 应在水平中点附近

#### find_node_by_str_id（3 个）

12. **`find_node_hit_root`** — tree 只有一个节点，id="foo"，查 "foo" 返回 Some(rect)
13. **`find_node_hit_nested`** — tree 有三层嵌套，叶子 id="target"，查返回 Some 且 rect 正确
14. **`find_node_miss`** — 查不存在的 id，返回 None

#### paint_node dry run（1 个）

15. **`paint_dry_run_variety`** — 构造包含 Container、Text Leaf、Grid Leaf、Connection Leaf、带 Transform 节点的 tree，调用 paint() 不 panic。用简单的 Renderer mock 或 feature flag 跳过实际 GPU 调用（如果可行）。**如果 Renderer 无法在测试中构造，此测试跳过，spec 注明为已知缺口。**

### 测试注意事项

- 测试 15 可能需要一个能在没有 GPU 的环境下构造的 Renderer。如果 Renderer 的 `new()` 必须要 wgpu Device，这个测试要么写不了，要么需要一个 feature gate。plan 阶段决定具体方案——若不可行，文档说明并跳过。
- 测试 1-14 都是纯函数，不依赖 Renderer/Tree 以外的基础设施，测试代码简洁。
- `find_node_by_str_id` 的测试需要构造 Tree 实例。参考 C.1 / C.4 的测试 helper（直接 insert PanelNode）。

## 风险与缓解

### 已知风险

1. **`Point` 类型可能不支持 `Copy` 或某些运算符** — `paint_helpers.rs` 里的数学计算假设 `Point` 有 `x, y: f32` 字段。**缓解**：实施者 Task 1 先读 `gui/src/renderer/types.rs`（或类似文件）确认 `Point` 定义。如果没有 `Point`，改用 `(f32, f32)` 元组。

2. **`Renderer::draw_curve` 签名可能与预期不符** — spec 假设它是 `draw_curve(points: [Point; 4], width: f32, color: Color)`。**缓解**：Task 1 读 renderer.rs 验证，如签名不同就按实际参数调整 paint_connection。

3. **`Transform` 类型可能没有 `PartialEq`** — PaintTransform::compose 需要读 Transform 字段。**缓解**：字段访问不需要 PartialEq，只需要 Copy。如果 Transform 没 Copy，改用 `&Transform` 引用。

4. **paint_node 签名改动波及 panel/renderer.rs** — 如果 panel/renderer.rs 直接调 paint_node 而不是 paint 入口，需要跟着改。**缓解**：Task 1 读 panel/renderer.rs 的 paint() 确认调用方式。如需改动，是一行改动（添加 `PaintTransform::identity()` 参数）。

5. **Grid 大范围 + 小 spacing 性能** — 10000×10000 的 rect + spacing=5 会生成 4,000,000 个点。**缓解**：C.5 不优化。demo 的 grid 是整个 viewport 范围（~1600×900），spacing >= 20，点数可控。未来实际出现问题再做裁剪。

6. **Connection 找不到 port 时的静默** — 可能让调试变困难。**缓解**：在 `paint_connection` 的 None 分支下，用 `#[cfg(debug_assertions)]` 条件 log 一条警告（不 panic）。plan 阶段决定是否需要。

### 非风险（明确不担心）

- **demo 破坏** — 不改 demo、不改 canvas/background.rs，demo 继续走旧路径画网格，行为与 C.4 结束完全一致。
- **C.1~C.4 测试回归** — 只动 paint 子系统，不影响 reconcile / layout / hit_test / widget。43 个单元测试继续 PASS。
- **`panel/renderer.rs::hit_test`** — 不改 hit_test，只可能改 paint 调用一行。

## 交付物

### 代码

- `gui/src/tree/paint_helpers.rs`（新建，~200 行：PaintTransform + 4 个纯函数 + ~15 个单元测试）
- `gui/src/tree/paint.rs`（修改，~100 行新增：Transform 累积、Grid 分支、Connection 分支、TextStyle 里的 scale 应用）
- `gui/src/tree/mod.rs`（修改，加一行 `mod paint_helpers;`）
- 可能：`gui/src/panel/renderer.rs`（如需适配 paint_node 签名，加一个参数）

### Commit

一次性 commit：`feat(gui): C.5 paint 扩展 — Grid / Connection / Transform 累积`。commit message 说明：
- 新增 `PaintTransform` 类型和累积机制（translate + scale，跳过 rotate）
- paint_node 签名增加 `tf` 参数（从 root 向下累积）
- LeafKind::Grid 和 LeafKind::Connection 的 paint 实现
- 纯函数提取到 paint_helpers.rs
- ~15 个单元测试（PaintTransform / grid_cells / bezier / find_node / dry run）
- 零 demo 改动，canvas/background.rs 不动

### 验证

- `cargo test -p gui --lib tree::paint_helpers::tests` — ~15 passed
- `cargo test -p gui` — 43（C.1~C.4）+ ~15（C.5）= ~58 passed
- `cargo check --workspace` — 无错误
- `cargo clippy -p gui` — 无新增警告
- `cargo build -p app --release` — demo 编译通过，行为与 C.4 结束时完全一致

## 参考

- 大纲：`docs/superpowers/plans/2026-04-10-phase-c-panel-system-unification.md`（C.5 节）
- 前置：`docs/superpowers/specs/2026-04-11-phase-c-layout-hit-gesture-design.md`（C.1+C.2+C.3）
- 前置：`docs/superpowers/specs/2026-04-11-phase-c4-panel-widget-design.md`（C.4）
- C.2 的逆变换：`gui/src/tree/hit.rs::inverse_transform`（C.5 的 apply_* 与之对称）
- Renderer API：`gui/src/renderer/renderer.rs`
- 现有 canvas/background.rs：将被 C.5 的 Grid paint 替代（但 C.5 不删，demo 还在用）
