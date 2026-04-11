# 阶段 C.5 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 扩展 `tree/paint.rs` 支持 `LeafKind::Grid`、`LeafKind::Connection` 绘制和 Transform 累积变换（translate + scale）。新增 `tree/paint_helpers.rs` 存放纯函数和单元测试。零 demo 改动。

**Architecture:** paint.rs 签名从 `paint_node(tree, id, renderer)` 改为 `paint_node(tree, id, renderer, tf: PaintTransform)`，入口 `paint(tree, root, renderer)` 用 `PaintTransform::identity()` 起始。递归时遇到 `node.style.transform: Some(_)` 就用 `compose` 复合出子 tf 传下去。所有节点的 `node.rect` 用 `tf.apply_rect` 变换为 screen rect 再传给 renderer。Grid/Connection 的绘制逻辑用纯函数（paint_helpers）辅助。

**Tech Stack:** Rust、`Renderer::draw_rect/draw_text/draw_circle/draw_curve`、`#[cfg(test)] mod tests`、`cargo test -p gui` 验证。

**Spec:** `docs/superpowers/specs/2026-04-11-phase-c5-paint-extensions-design.md`

---

## 前置知识

### commit 策略

本 plan 产出 **1 个 commit**：`feat(gui): C.5 paint 扩展 — Grid / Connection / Transform`。所有改动在同一次 commit 内。

### 测试基础设施

C.4 结束时 gui crate 有 43 个单元测试。本 plan 在 `tree/paint_helpers.rs` 末尾添加 ~15 个新测试，合计 ~58 个。

### 关键类型（已验证）

```rust
// gui/src/renderer/types.rs
#[derive(Debug, Clone, Copy)]
pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point { pub x: f32, pub y: f32 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }

// gui/src/tree/layout/types.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translate: [f32; 2],
    pub scale: f32,
    pub rotate: f32,  // 弧度（C.5 不应用）
}
```

### Renderer 方法（已验证）

```rust
// gui/src/renderer/renderer.rs
impl Renderer {
    pub fn draw_rect(&mut self, rect: Rect, style: &RectStyle);
    pub fn draw_text(&mut self, pos: Point, text: &str, style: &TextStyle);
    pub fn draw_circle(&mut self, center: Point, radius: f32, color: Color);
    pub fn draw_curve(&mut self, points: [Point; 4], width: f32, color: Color);
    // ...
}
```

### 现有 paint.rs 结构（已验证）

```rust
// gui/src/tree/paint.rs（当前 39 行）
pub fn paint(tree: &Tree, root: NodeId, renderer: &mut Renderer) {
    paint_node(tree, root, renderer);
}

fn paint_node(tree: &Tree, node_id: NodeId, renderer: &mut Renderer) {
    let Some(node) = tree.get(node_id) else { return };
    let rect = node.rect;

    if let Some(dec) = &node.decoration {
        renderer.draw_rect(rect, &RectStyle { ... });
    }

    if let NodeKind::Leaf(LeafKind::Text { content, font_size, color }) = &node.kind {
        renderer.draw_text(
            Point { x: rect.x, y: rect.y },
            content,
            &TextStyle { color: *color, size: *font_size },
        );
    }

    let children: Vec<NodeId> = tree.get(node_id).map(|n| n.children.clone()).unwrap_or_default();
    for child_id in children {
        paint_node(tree, child_id, renderer);
    }
}
```

### Connection 字段（已验证）

```rust
// gui/src/tree/layout/types.rs::LeafKind::Connection
Connection {
    from_port: Cow<'static, str>,
    to_port: Cow<'static, str>,
}
```

### 已确认设计决策（来自 spec）

1. **PaintTransform 独立类型**：`{ tx: f32, ty: f32, scale: f32 }`，不复用 `Transform`
2. **复合顺序**：`new.tx = self.tx + self.scale * child.translate[0]`（父先应用，子后应用）
3. **Connection 锚点**：from 用 `center_right`（右中），to 用 `center_left`（左中）
4. **贝塞尔启发式**：水平偏移 = `|dx|/2`
5. **Port 查找**：线性遍历树，O(n) per connection
6. **找不到 port**：静默跳过
7. **Grid dot_size**：也按 scale 缩放
8. **纯函数单独文件**：`paint_helpers.rs`

---

## File Structure

### 本 plan 创建或修改的文件

| 路径 | 改动 | 所属 Task |
|------|-----|----------|
| `gui/src/tree/paint_helpers.rs` | **新建** | Task 1-3 |
| `gui/src/tree/mod.rs` | 加 `mod paint_helpers;` | Task 1 |
| `gui/src/tree/paint.rs` | 扩展 paint_node：PaintTransform 参数 + Grid/Connection 分支 | Task 4 |

### 本 plan 不修改的文件

- `app/src/demo.rs`
- `gui/src/renderer/**`（不动 wgpu 层）
- `gui/src/canvas/**`（demo 还在用 canvas/background.rs 画网格）
- `gui/src/panel/renderer.rs`（调的是 `tree::paint(root, renderer)` 入口，入口签名不变）
- `gui/src/tree/layout/**`
- `gui/src/tree/hit.rs`、`diff.rs`、`tree.rs`、`node.rs`、`desc.rs` 等 tree 子模块
- `gui/src/widget/**`
- `gui/src/gesture/**`

---

## Tasks

### Task 1: 创建 paint_helpers.rs 骨架 + PaintTransform 类型

**Files:**
- Create: `gui/src/tree/paint_helpers.rs`
- Modify: `gui/src/tree/mod.rs`

**Goal:** 新建 `paint_helpers.rs`，定义 `PaintTransform` 类型和所有纯函数的占位实现（能编译，但行为错误）。注册到 `tree/mod.rs`。

- [ ] **Step 1.1: 读取当前 tree/mod.rs**

Read `gui/src/tree/mod.rs` 确认现有 mod 声明顺序。

- [ ] **Step 1.2: 修改 tree/mod.rs 加 paint_helpers 模块**

用 Edit 把 `mod paint;` 那一行附近改为（加一行 `mod paint_helpers;`）：

```rust
// 当前应该有 mod paint; 类似的行，在它前后加一行
mod paint_helpers;
```

具体位置以实际 tree/mod.rs 文件为准（按字母序或按 paint 相邻放置）。

- [ ] **Step 1.3: 创建 paint_helpers.rs 骨架**

用 Write 工具创建 `gui/src/tree/paint_helpers.rs`，内容如下：

```rust
//! Paint 子系统的纯函数辅助 + PaintTransform 类型。
//!
//! 职责：把 paint.rs 里可独立验证的数学/查找逻辑提取出来，
//! 让 paint.rs 本身只做"读 tree、分发 LeafKind、调 renderer"的胶水。

use crate::renderer::{Point, Rect};
use crate::tree::layout::types::Transform;
use crate::tree::node::NodeId;
use crate::tree::tree::Tree;

/// 从 paint 入口向下递归时累积的变换。
///
/// 只处理 translate + scale，不处理 rotate——Renderer 的 draw_* 方法
/// 不支持旋转矩形，而 C.2 的 inverse_transform 同样只处理 translate + scale，
/// 两者保持对称。
#[derive(Debug, Clone, Copy)]
pub struct PaintTransform {
    pub tx: f32,
    pub ty: f32,
    pub scale: f32,
}

impl PaintTransform {
    /// 恒等变换。paint 入口使用。
    pub fn identity() -> Self {
        // TODO(Task 3): 实际实现
        Self { tx: 0.0, ty: 0.0, scale: 1.0 }
    }

    /// 复合父累积变换和子节点自身的 transform。
    ///
    /// 公式：
    ///   new.tx = self.tx + self.scale * child.translate[0]
    ///   new.ty = self.ty + self.scale * child.translate[1]
    ///   new.scale = self.scale * child.scale
    ///
    /// rotate 字段被忽略。
    pub fn compose(&self, _child: &Transform) -> Self {
        // TODO(Task 3): 实际实现
        *self
    }

    /// 把 local 点变换到 screen 点。
    pub fn apply_point(&self, _p: Point) -> Point {
        // TODO(Task 3): 实际实现
        Point { x: 0.0, y: 0.0 }
    }

    /// 把 local rect 变换到 screen rect。w/h 也按 scale 缩放。
    pub fn apply_rect(&self, _r: Rect) -> Rect {
        // TODO(Task 3): 实际实现
        Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }
    }
}

/// 遍历 rect 内 spacing 为间距的格点。
///
/// 从 (rect.x, rect.y) 开始，每 spacing 像素一个点，直到 (rect.x + rect.w, rect.y + rect.h)。
/// spacing <= 0 时返回空 Vec（避免死循环）。
pub fn grid_cells(_rect: Rect, _spacing: f32) -> Vec<Point> {
    // TODO(Task 3): 实际实现
    Vec::new()
}

/// 计算两点之间的三次贝塞尔 4 个控制点（水平偏移启发式）。
///
/// p0 = from, p3 = to
/// p1 = (from.x + |dx|/2, from.y)
/// p2 = (to.x - |dx|/2, to.y)
pub fn bezier_control_points(_from: Point, _to: Point) -> [Point; 4] {
    // TODO(Task 3): 实际实现
    [Point { x: 0.0, y: 0.0 }; 4]
}

/// 深度优先遍历 tree，查找 id 字符串匹配的节点，返回其 rect。
///
/// 找不到返回 None。O(n)——C.5 范围内可接受。
pub fn find_node_by_str_id(_tree: &Tree, _id: &str) -> Option<Rect> {
    // TODO(Task 3): 实际实现
    None
}

/// 取 rect 右边中点（Connection 默认的输出锚点）。
pub fn rect_center_right(r: Rect) -> Point {
    // 这个函数足够简单，直接实现
    Point { x: r.x + r.w, y: r.y + r.h * 0.5 }
}

/// 取 rect 左边中点（Connection 默认的输入锚点）。
pub fn rect_center_left(r: Rect) -> Point {
    // 这个函数足够简单，直接实现
    Point { x: r.x, y: r.y + r.h * 0.5 }
}
```

**注意**：`rect_center_right` 和 `rect_center_left` 已经是最终实现（这些太简单了，没必要走 TDD 红灯流程）。其他函数用 TODO 占位。

- [ ] **Step 1.4: cargo check 验证编译**

Run: `cargo check -p gui 2>&1 | tail -15`

Expected: 无新增错误。可能有 `dead_code` / `unused_variable` 警告（占位实现未用参数），这是预期的。

如果有编译错误：
- `Point` / `Rect` 路径不对：验证 import 是 `use crate::renderer::{Point, Rect};`
- `Transform` 路径不对：读 `gui/src/tree/layout/types.rs` 或 `mod.rs` 确认实际 re-export 位置
- `NodeId` / `Tree` 路径不对：参考 `paint.rs` 当前的 use 语句

- [ ] **Step 1.5: 不 commit，继续 Task 2**

---

### Task 2: 写 15 个失败测试

**Files:**
- Modify: `gui/src/tree/paint_helpers.rs`（末尾追加 `#[cfg(test)] mod tests`）

**Goal:** 写全 15 个单元测试，运行后大部分失败（Task 1 的占位实现不正确）。

- [ ] **Step 2.1: 在 paint_helpers.rs 末尾追加 tests 模块骨架**

用 Edit 在文件末尾追加：

```rust

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::layout::types::{BoxStyle, LeafKind, Transform};
    use crate::tree::node::{NodeKind, PanelNode};
    use crate::renderer::Color;
    use std::borrow::Cow;

    /// 构造一个带指定 id 和 rect 的基础 Container 节点
    fn container_at(id: &'static str, rect: Rect) -> PanelNode {
        PanelNode {
            id: Cow::Borrowed(id),
            style: BoxStyle::default(),
            decoration: None,
            kind: NodeKind::Container,
            rect,
            children: Vec::new(),
            scroll_offset: 0.0,
            content_height: 0.0,
        }
    }
}
```

### Step 2.2: 添加 5 个 PaintTransform 测试

在 tests 模块里（`container_at` 之后）追加：

```rust
    // ── PaintTransform 测试 ──

    #[test]
    fn transform_identity_compose() {
        let id = PaintTransform::identity();
        let child = Transform {
            translate: [10.0, 20.0],
            scale: 2.0,
            rotate: 0.0,
        };
        let result = id.compose(&child);
        assert_eq!(result.tx, 10.0);
        assert_eq!(result.ty, 20.0);
        assert_eq!(result.scale, 2.0);
    }

    #[test]
    fn transform_compose_nested() {
        // identity → {translate:[10,20], scale:2} → {translate:[5,5], scale:3}
        // 公式：new.tx = self.tx + self.scale * child.translate[0]
        // 第一次 compose：{tx: 10, ty: 20, scale: 2}
        // 第二次 compose：{tx: 10 + 2*5, ty: 20 + 2*5, scale: 2*3}
        let first = PaintTransform::identity().compose(&Transform {
            translate: [10.0, 20.0],
            scale: 2.0,
            rotate: 0.0,
        });
        let second = first.compose(&Transform {
            translate: [5.0, 5.0],
            scale: 3.0,
            rotate: 0.0,
        });
        assert_eq!(second.tx, 20.0, "tx = 10 + 2*5 = 20");
        assert_eq!(second.ty, 30.0, "ty = 20 + 2*5 = 30");
        assert_eq!(second.scale, 6.0, "scale = 2*3 = 6");
    }

    #[test]
    fn transform_apply_point() {
        let tf = PaintTransform { tx: 10.0, ty: 20.0, scale: 2.0 };
        let result = tf.apply_point(Point { x: 3.0, y: 4.0 });
        assert_eq!(result.x, 16.0, "x = 10 + 2*3 = 16");
        assert_eq!(result.y, 28.0, "y = 20 + 2*4 = 28");
    }

    #[test]
    fn transform_apply_rect() {
        let tf = PaintTransform { tx: 10.0, ty: 20.0, scale: 2.0 };
        let result = tf.apply_rect(Rect { x: 3.0, y: 4.0, w: 5.0, h: 6.0 });
        assert_eq!(result.x, 16.0);
        assert_eq!(result.y, 28.0);
        assert_eq!(result.w, 10.0, "w = 2*5 = 10");
        assert_eq!(result.h, 12.0, "h = 2*6 = 12");
    }

    #[test]
    fn transform_ignores_rotate() {
        // 即使 child.rotate 非 0，结果 PaintTransform 也只有 tx/ty/scale
        let id = PaintTransform::identity();
        let child = Transform {
            translate: [10.0, 20.0],
            scale: 2.0,
            rotate: 1.5,  // 非零旋转
        };
        let result = id.compose(&child);
        // 只验证 tx/ty/scale（PaintTransform 没有 rotate 字段）
        assert_eq!(result.tx, 10.0);
        assert_eq!(result.ty, 20.0);
        assert_eq!(result.scale, 2.0);
    }
```

- [ ] **Step 2.3: 添加 3 个 grid_cells 测试**

```rust
    // ── grid_cells 测试 ──

    #[test]
    fn grid_cells_single_point() {
        // rect=(0,0,0,0)，spacing=10 → 只有 1 个点 (0, 0)
        let rect = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
        let cells = grid_cells(rect, 10.0);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0], Point { x: 0.0, y: 0.0 });
    }

    #[test]
    fn grid_cells_multi() {
        // rect=(0,0,30,30)，spacing=10 → 4×4 = 16 个点
        // x 遍历: 0, 10, 20, 30（4 个）
        // y 遍历: 0, 10, 20, 30（4 个）
        let rect = Rect { x: 0.0, y: 0.0, w: 30.0, h: 30.0 };
        let cells = grid_cells(rect, 10.0);
        assert_eq!(cells.len(), 16);
        // 检查几个关键点
        assert!(cells.contains(&Point { x: 0.0, y: 0.0 }));
        assert!(cells.contains(&Point { x: 10.0, y: 0.0 }));
        assert!(cells.contains(&Point { x: 30.0, y: 30.0 }));
    }

    #[test]
    fn grid_cells_zero_spacing() {
        // spacing=0 应返回空 Vec（避免死循环）
        let rect = Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 };
        let cells = grid_cells(rect, 0.0);
        assert!(cells.is_empty(), "spacing=0 应返回空");

        // 负 spacing 也应返回空
        let cells_neg = grid_cells(rect, -5.0);
        assert!(cells_neg.is_empty(), "负 spacing 应返回空");
    }
```

- [ ] **Step 2.4: 添加 3 个 bezier_control_points 测试**

```rust
    // ── bezier_control_points 测试 ──

    #[test]
    fn bezier_horizontal() {
        // from=(0,0), to=(100,0)
        // |dx| = 100, 偏移 = 50
        // p0=(0,0), p1=(50,0), p2=(50,0), p3=(100,0)
        let result = bezier_control_points(
            Point { x: 0.0, y: 0.0 },
            Point { x: 100.0, y: 0.0 },
        );
        assert_eq!(result[0], Point { x: 0.0, y: 0.0 });
        assert_eq!(result[1], Point { x: 50.0, y: 0.0 });
        assert_eq!(result[2], Point { x: 50.0, y: 0.0 });
        assert_eq!(result[3], Point { x: 100.0, y: 0.0 });
    }

    #[test]
    fn bezier_diagonal() {
        // from=(0,0), to=(100,50)
        // |dx| = 100, 偏移 = 50
        // p0=(0,0), p1=(50,0), p2=(50,50), p3=(100,50)
        let result = bezier_control_points(
            Point { x: 0.0, y: 0.0 },
            Point { x: 100.0, y: 50.0 },
        );
        assert_eq!(result[0], Point { x: 0.0, y: 0.0 });
        assert_eq!(result[1], Point { x: 50.0, y: 0.0 });
        assert_eq!(result[2], Point { x: 50.0, y: 50.0 });
        assert_eq!(result[3], Point { x: 100.0, y: 50.0 });
    }

    #[test]
    fn bezier_reverse() {
        // from=(100,0), to=(0,0)（反向）
        // |dx| = 100, 偏移 = 50
        // p0=(100,0), p1=(100+50, 0)=(150,0), p2=(0-50, 0)=(-50,0), p3=(0,0)
        let result = bezier_control_points(
            Point { x: 100.0, y: 0.0 },
            Point { x: 0.0, y: 0.0 },
        );
        assert_eq!(result[0], Point { x: 100.0, y: 0.0 });
        assert_eq!(result[1], Point { x: 150.0, y: 0.0 });
        assert_eq!(result[2], Point { x: -50.0, y: 0.0 });
        assert_eq!(result[3], Point { x: 0.0, y: 0.0 });
    }
```

- [ ] **Step 2.5: 添加 3 个 find_node_by_str_id 测试**

```rust
    // ── find_node_by_str_id 测试 ──

    #[test]
    fn find_node_hit_root() {
        let mut tree = Tree::new();
        let root_id = tree.insert(container_at("root", Rect {
            x: 10.0, y: 20.0, w: 100.0, h: 50.0,
        }));
        tree.set_root(root_id);

        let result = find_node_by_str_id(&tree, "root");
        assert!(result.is_some(), "应该找到 root");
        let rect = result.unwrap();
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
    }

    #[test]
    fn find_node_hit_nested() {
        // 三层嵌套：root → middle → target
        let mut tree = Tree::new();
        let target_id = tree.insert(container_at("target", Rect {
            x: 30.0, y: 40.0, w: 20.0, h: 10.0,
        }));
        let middle = {
            let mut n = container_at("middle", Rect {
                x: 5.0, y: 5.0, w: 100.0, h: 100.0,
            });
            n.children = vec![target_id];
            n
        };
        let middle_id = tree.insert(middle);
        let root = {
            let mut n = container_at("root", Rect {
                x: 0.0, y: 0.0, w: 200.0, h: 200.0,
            });
            n.children = vec![middle_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let result = find_node_by_str_id(&tree, "target");
        assert!(result.is_some(), "嵌套三层应该能找到 target");
        let rect = result.unwrap();
        assert_eq!(rect.x, 30.0);
        assert_eq!(rect.w, 20.0);
    }

    #[test]
    fn find_node_miss() {
        let mut tree = Tree::new();
        let root_id = tree.insert(container_at("root", Rect {
            x: 0.0, y: 0.0, w: 100.0, h: 100.0,
        }));
        tree.set_root(root_id);

        let result = find_node_by_str_id(&tree, "nonexistent");
        assert!(result.is_none(), "不存在的 id 应返回 None");
    }
```

- [ ] **Step 2.6: 添加 1 个 paint dry run 测试**

**注意**：这个测试需要一个能在 test 里构造的 Renderer。`Renderer::new` 需要 wgpu Device——如果没法在 test 里构造，**跳过此测试**，在 Task 4 Step 4.6 的 dry run 步骤里用 cargo build -p app 替代验证。

尝试写，如果编译/运行失败，注释掉整个测试并加 `// TODO: Renderer 需 wgpu Device，暂无法单测`：

```rust
    // ── paint_node dry run（依赖 Renderer 能否构造）──

    #[test]
    #[ignore = "Renderer 需 wgpu Device，暂无单测 stub"]
    fn paint_dry_run_variety() {
        // TODO: 如果未来引入 test-renderer 能构造，替代此 stub
    }
```

**决策**：本 plan 默认写 `#[ignore]` 跳过，保留测试入口方便未来补齐。

- [ ] **Step 2.7: cargo check 验证 tests 可编译**

Run: `cargo check -p gui 2>&1 | tail -15`

Expected: 无编译错误。

如果有错误：
- `PanelNode` 字段不对：读 `gui/src/tree/node.rs` 确认实际字段名
- `BoxStyle::default()` 不存在：读 `gui/src/tree/layout/types.rs` 确认是否 derive Default
- `Tree::new()` / `Tree::insert` / `Tree::set_root` 签名：读 `gui/src/tree/tree.rs`

- [ ] **Step 2.8: 运行测试，确认失败**

Run: `cargo test -p gui --lib tree::paint_helpers::tests 2>&1 | tail -40`

**预期**：
- `transform_apply_point` / `transform_apply_rect` / `transform_compose_nested` FAIL（占位返回 `*self` / `Point{0,0}` / `Rect{0,0,0,0}`）
- `grid_cells_multi` FAIL（占位返回空 Vec）
- `bezier_horizontal` / `bezier_diagonal` / `bezier_reverse` FAIL（占位返回全 0 Point）
- `find_node_hit_root` / `find_node_hit_nested` FAIL（占位返回 None）

**意外 PASS 的测试**（占位恰好满足）：
- `transform_identity_compose` — 占位 compose 返回 `*self`，`identity()` 返回 `{0,0,1}`，和测试期望的 `{10,20,2}` 不匹配 → FAIL（实际会 FAIL）
- `transform_ignores_rotate` — 同上 → FAIL
- `grid_cells_single_point` — 占位返回空 Vec，期望 len=1 → FAIL
- `grid_cells_zero_spacing` — 占位返回空 Vec，期望空 Vec → **意外 PASS**
- `find_node_miss` — 占位返回 None，期望 None → **意外 PASS**
- `paint_dry_run_variety` — `#[ignore]` → IGNORED

预期分布：**~11 FAIL + 2 意外 PASS + 1 IGNORED**（总 14 个有效 + 1 ignored，共 15）。

如果只有这 2 个意外 PASS，符合预期，继续 Task 3。

- [ ] **Step 2.9: 不 commit，继续 Task 3**

---

### Task 3: 实现 paint_helpers 的纯函数

**Files:**
- Modify: `gui/src/tree/paint_helpers.rs`（替换占位实现为真实实现）

**Goal:** 把 Task 1 的占位实现替换为真实逻辑，让 Task 2 的 ~14 个有效测试全部 PASS（不含 ignored 的 dry run）。

- [ ] **Step 3.1: 实现 PaintTransform::identity**

找到：
```rust
    pub fn identity() -> Self {
        // TODO(Task 3): 实际实现
        Self { tx: 0.0, ty: 0.0, scale: 1.0 }
    }
```

用 Edit 替换为（去掉 TODO，保持相同实现——恒等变换已经是 `{0,0,1}`）：

```rust
    pub fn identity() -> Self {
        Self { tx: 0.0, ty: 0.0, scale: 1.0 }
    }
```

- [ ] **Step 3.2: 实现 PaintTransform::compose**

找到：
```rust
    pub fn compose(&self, _child: &Transform) -> Self {
        // TODO(Task 3): 实际实现
        *self
    }
```

用 Edit 替换为：

```rust
    pub fn compose(&self, child: &Transform) -> Self {
        Self {
            tx: self.tx + self.scale * child.translate[0],
            ty: self.ty + self.scale * child.translate[1],
            scale: self.scale * child.scale,
        }
    }
```

- [ ] **Step 3.3: 实现 PaintTransform::apply_point**

找到：
```rust
    pub fn apply_point(&self, _p: Point) -> Point {
        // TODO(Task 3): 实际实现
        Point { x: 0.0, y: 0.0 }
    }
```

用 Edit 替换为：

```rust
    pub fn apply_point(&self, p: Point) -> Point {
        Point {
            x: self.tx + self.scale * p.x,
            y: self.ty + self.scale * p.y,
        }
    }
```

- [ ] **Step 3.4: 实现 PaintTransform::apply_rect**

找到：
```rust
    pub fn apply_rect(&self, _r: Rect) -> Rect {
        // TODO(Task 3): 实际实现
        Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }
    }
```

用 Edit 替换为：

```rust
    pub fn apply_rect(&self, r: Rect) -> Rect {
        Rect {
            x: self.tx + self.scale * r.x,
            y: self.ty + self.scale * r.y,
            w: self.scale * r.w,
            h: self.scale * r.h,
        }
    }
```

- [ ] **Step 3.5: 实现 grid_cells**

找到：
```rust
pub fn grid_cells(_rect: Rect, _spacing: f32) -> Vec<Point> {
    // TODO(Task 3): 实际实现
    Vec::new()
}
```

用 Edit 替换为：

```rust
pub fn grid_cells(rect: Rect, spacing: f32) -> Vec<Point> {
    if spacing <= 0.0 {
        return Vec::new();
    }

    let mut cells = Vec::new();
    let mut y = rect.y;
    while y <= rect.y + rect.h {
        let mut x = rect.x;
        while x <= rect.x + rect.w {
            cells.push(Point { x, y });
            x += spacing;
        }
        y += spacing;
    }
    cells
}
```

**注意**：`while x <= rect.x + rect.w` 用 `<=` 让终点（30.0）被包含，这样 w=30/spacing=10 的场景产生 4 个 x 值（0, 10, 20, 30）。

- [ ] **Step 3.6: 实现 bezier_control_points**

找到：
```rust
pub fn bezier_control_points(_from: Point, _to: Point) -> [Point; 4] {
    // TODO(Task 3): 实际实现
    [Point { x: 0.0, y: 0.0 }; 4]
}
```

用 Edit 替换为：

```rust
pub fn bezier_control_points(from: Point, to: Point) -> [Point; 4] {
    let dx_abs = (to.x - from.x).abs();
    let offset = dx_abs * 0.5;
    [
        from,
        Point { x: from.x + offset, y: from.y },
        Point { x: to.x - offset, y: to.y },
        to,
    ]
}
```

**注意**：对于 reverse 情况（from.x=100, to.x=0），`dx_abs=100, offset=50`，`p1.x = 100 + 50 = 150`, `p2.x = 0 - 50 = -50`，这正是测试 `bezier_reverse` 期望的。

- [ ] **Step 3.7: 实现 find_node_by_str_id**

找到：
```rust
pub fn find_node_by_str_id(_tree: &Tree, _id: &str) -> Option<Rect> {
    // TODO(Task 3): 实际实现
    None
}
```

用 Edit 替换为：

```rust
pub fn find_node_by_str_id(tree: &Tree, id: &str) -> Option<Rect> {
    let root = tree.root()?;
    find_recursive(tree, root, id)
}

fn find_recursive(tree: &Tree, node_id: NodeId, target_id: &str) -> Option<Rect> {
    let node = tree.get(node_id)?;
    if node.id.as_ref() == target_id {
        return Some(node.rect);
    }
    // 克隆 children 列表以释放借用（避免与递归冲突）
    let children: Vec<NodeId> = node.children.clone();
    for child_id in children {
        if let Some(rect) = find_recursive(tree, child_id, target_id) {
            return Some(rect);
        }
    }
    None
}
```

**注意**：用 `node.id.as_ref() == target_id` 做字符串比较。`Cow<'static, str>` 的 `as_ref()` 返回 `&str`。

- [ ] **Step 3.8: cargo check 验证编译**

Run: `cargo check -p gui 2>&1 | tail -15`

Expected: 编译通过（可能有 `unused` 或 `dead_code` 警告，如果 `find_recursive` 未被测试外的 paint 使用，先忽略，Task 4 会用）。

- [ ] **Step 3.9: 运行测试，验证通过**

Run: `cargo test -p gui --lib tree::paint_helpers::tests 2>&1 | tail -30`

Expected: **14 passed + 1 ignored，0 failed**（paint_dry_run_variety 是 `#[ignore]`）。

如果有失败：
- `grid_cells_multi` 期望 16 个但少/多：检查 `<=` 还是 `<`，确认 `while x <= rect.x + rect.w` 包含终点
- `find_node_hit_nested` 失败：确认 `node.children.clone()` 正确，`find_recursive` 递归顺序对
- `bezier_reverse` 失败：确认用的是 `abs()` 而不是 `(to.x - from.x)` 直接参与

- [ ] **Step 3.10: 确认 C.1~C.4 测试没有回归**

Run: `cargo test -p gui 2>&1 | tail -10`

Expected: `test result: ok. 57 passed; 0 failed; 1 ignored`（43 + 14，ignored 1 个）

如果有回归，检查是否无意中修改了其他文件。

- [ ] **Step 3.11: 不 commit，继续 Task 4**

---

### Task 4: 修改 paint.rs 使用 PaintTransform + 实现 Grid/Connection 分支

**Files:**
- Modify: `gui/src/tree/paint.rs`

**Goal:** 改造 paint_node 接收 `PaintTransform` 参数，所有 renderer 调用用 screen rect 代替 local rect。添加 Grid 和 Connection 的 LeafKind 分支。

- [ ] **Step 4.1: 读取 paint.rs 全文**

Read `gui/src/tree/paint.rs` 确认当前结构（应该是 39 行左右，有 `paint` 入口和 `paint_node` 递归函数）。

- [ ] **Step 4.2: 用 Write 重写整个 paint.rs**

为了保证改动清晰，整个文件重写。用 Write 工具覆盖 `gui/src/tree/paint.rs`：

```rust
use super::layout::LeafKind;
use super::node::{NodeId, NodeKind};
use super::paint_helpers::{
    bezier_control_points, find_node_by_str_id, grid_cells, rect_center_left,
    rect_center_right, PaintTransform,
};
use super::tree::Tree;
use crate::renderer::{Color, Point, Renderer, RectStyle, TextStyle};

// ── Connection 样式常量（C.7 可能接主题系统）──

/// 连线宽度（local 空间像素，paint 时按 scale 缩放）
const CONNECTION_WIDTH: f32 = 2.0;
/// 连线颜色（中性灰）
const CONNECTION_COLOR: Color = Color { r: 0.55, g: 0.58, b: 0.65, a: 1.0 };

pub fn paint(tree: &Tree, root: NodeId, renderer: &mut Renderer) {
    paint_node(tree, root, renderer, PaintTransform::identity());
}

fn paint_node(tree: &Tree, node_id: NodeId, renderer: &mut Renderer, tf: PaintTransform) {
    let Some(node) = tree.get(node_id) else { return };

    // 把 local rect 变换到 screen rect
    let screen_rect = tf.apply_rect(node.rect);

    // 1. Container decoration
    if let Some(dec) = &node.decoration {
        renderer.draw_rect(
            screen_rect,
            &RectStyle {
                color: dec.background.unwrap_or(Color::TRANSPARENT),
                border: dec.border,
                radius: dec.radius,
                shadow: dec.shadow.clone(),
            },
        );
    }

    // 2. Leaf 分发
    if let NodeKind::Leaf(leaf) = &node.kind {
        match leaf {
            LeafKind::Text { content, font_size, color } => {
                renderer.draw_text(
                    Point {
                        x: screen_rect.x,
                        y: screen_rect.y,
                    },
                    content,
                    &TextStyle {
                        color: *color,
                        size: *font_size * tf.scale,
                    },
                );
            }
            LeafKind::Grid { spacing, dot_color, dot_size } => {
                // 用 local rect（node.rect）遍历格点，逐点变换
                for p in grid_cells(node.rect, *spacing) {
                    let sp = tf.apply_point(p);
                    renderer.draw_circle(sp, *dot_size * tf.scale, *dot_color);
                }
            }
            LeafKind::Connection { from_port, to_port } => {
                let Some(from_rect) = find_node_by_str_id(tree, from_port.as_ref()) else {
                    return; // 静默跳过
                };
                let Some(to_rect) = find_node_by_str_id(tree, to_port.as_ref()) else {
                    return; // 静默跳过
                };
                // 锚点在 local 空间，变换到 screen
                let from_p = tf.apply_point(rect_center_right(from_rect));
                let to_p = tf.apply_point(rect_center_left(to_rect));
                let ctrl = bezier_control_points(from_p, to_p);
                renderer.draw_curve(ctrl, CONNECTION_WIDTH * tf.scale, CONNECTION_COLOR);
            }
            // 其余 LeafKind 暂不实现（Image / Icon / Circle / Line / Curve / Path / CustomPaint）
            _ => {}
        }
    }

    // 3. 复合 Transform 并递归子节点
    let child_tf = match &node.style.transform {
        Some(tf_decl) => tf.compose(tf_decl),
        None => tf,
    };

    let children: Vec<NodeId> = node.children.clone();
    for child_id in children {
        paint_node(tree, child_id, renderer, child_tf);
    }
}
```

**注意**：
- 子节点循环前用了 `let children: Vec<NodeId> = node.children.clone();` 然后 `drop(node)` 隐式发生（新作用域）。但这里 `node` 借用还活跃，我们只 clone 了 children。之后的递归 `paint_node(tree, ..., renderer, ...)` 需要重新借 tree。实际上 `node` 的借用在 clone 后还活着，但递归 paint_node 不会借 tree 的同一个 slot，因为递归是下行到 children。这跟 C.2 hit.rs 的借用冲突场景不同——hit.rs 递归时要重新 get node，paint.rs 只是把数据用完。
- 实际上，Rust 的借用检查器可能会投诉：`tree.get(node_id)` 的返回值 `node` 是 `&PanelNode`，活到 children 循环之前。在循环里 `paint_node(tree, ...)` 是对 `tree` 的新借用，与 `node` 的借用重叠。解决方案：
  - 方案 A：在循环前 `drop(node)` 或让 `node` 自然出作用域
  - 方案 B：提前把需要的数据拷贝出来再放手 `node` 借用
  - 方案 C：用 `let children = { let node = tree.get(node_id).unwrap(); node.children.clone() };` 在块里借完就放
- 最稳妥的是方案 B：在使用 `node` 的所有字段之后，循环之前，用 `let transform_opt = node.style.transform; drop 隐式`。实际上 Rust 的 NLL（Non-Lexical Lifetimes）会在 `node` 最后一次使用后自动释放借用。

让我改为更明确的结构（下一步 Step 4.3）。

- [ ] **Step 4.3: cargo check 验证编译**

Run: `cargo check -p gui 2>&1 | tail -20`

**可能的编译错误**：
1. **借用冲突**：`node` 借用还活着时调 `paint_node(tree, ...)`。NLL 应该能处理，但如果不行，修改 Step 4.2 的代码：
   - 在 node 字段全部使用完之后，用 `drop(node);` 或
   - 重构为 `let (children, transform_opt) = { let node = tree.get(node_id).unwrap(); (node.children.clone(), node.style.transform) };` 提前解构

2. **`dec.shadow.clone()` 错误**：如果 shadow 字段已经是 Option<Shadow> 并且 Shadow 不是 Clone，改为 `dec.shadow`（如果 Option<Shadow> 是 Copy）或读 types.rs 确认 Shadow 的 Clone/Copy 状态。

3. **`Transform` 的访问**：`node.style.transform` 的类型是 `Option<Transform>`，`Some(tf_decl)` 是 `&Transform`，compose 接收 `&Transform`，OK。

修复编译错误直到通过为止。

- [ ] **Step 4.4: 运行所有测试，验证无回归**

Run: `cargo test -p gui 2>&1 | tail -15`

Expected: `test result: ok. 57 passed; 0 failed; 1 ignored`

Task 4 对 paint.rs 的改动**不应该**影响 C.1~C.4 的测试（因为那些都是布局/命中/widget 测试，不涉及 paint 的实际输出）。paint_helpers 的测试继续 PASS。

如果有测试失败：
- C.1~C.4 回归：检查是否无意中改了其他文件
- paint_helpers 回归：检查 paint.rs 的 import 是否改动了 paint_helpers 的函数

- [ ] **Step 4.5: cargo clippy 检查**

Run: `cargo clippy -p gui 2>&1 | tail -30`

Expected: 没有本次 C.5 新引入的警告。预存的警告（text_edit、canvas 等）可以存在。

如果有新警告：
- `unused_imports`：删除未用的 import
- `needless_return`：移除多余的 return
- `clone_on_copy`：把 Clone 改为 Copy（如 `dec.shadow.clone()` 若可 Copy 则去掉 .clone()）
- `single_match`：`if let NodeKind::Leaf(leaf) = &node.kind { match leaf { ... } }` 可能触发这条

修复到 clippy 无新警告。

- [ ] **Step 4.6: dry run 验证 demo 编译**

Run: `cargo build -p app --release 2>&1 | tail -10`

Expected: 编译通过。

这一步验证两件事：
1. paint.rs 的签名改动（入口 `pub fn paint(tree, root, renderer)` 签名不变）对 panel/renderer.rs 没影响
2. 新的 paint_node 内部不会因 tree 结构被 C.4 的 PanelProps 扩展而 panic

**注意**：这一步 **不** 实际运行 demo，只是编译。真正的视觉验证要等 C.7 demo 重写。

- [ ] **Step 4.7: 不 commit，继续 Task 5**

---

### Task 5: 最终验证 + commit

**Files:** 无改动

**Goal:** 最后全量验证 + commit 所有 C.5 改动为一个 feat commit。

- [ ] **Step 5.1: cargo test --workspace 全量回归**

Run: `cargo test --workspace 2>&1 | tail -20`

Expected: 所有 workspace crate 通过。gui crate 应该是 `ok. 57 passed; 0 failed; 1 ignored`。

- [ ] **Step 5.2: cargo check --workspace**

Run: `cargo check --workspace 2>&1 | tail -10`

Expected: 无错误。

- [ ] **Step 5.3: 检查 git status**

Run: `git status`

预期改动：
- `modified:   gui/src/tree/mod.rs`
- `modified:   gui/src/tree/paint.rs`
- `new file:   gui/src/tree/paint_helpers.rs`

**如果有其他未预期改动**，停止并报告 BLOCKED。

- [ ] **Step 5.4: commit C.5**

Run:
```bash
cd /Users/aporicho/Desktop/nodeimg/.claude/worktrees/ui && git add gui/src/tree/mod.rs gui/src/tree/paint.rs gui/src/tree/paint_helpers.rs && git commit -m "$(cat <<'EOF'
feat(gui): C.5 paint 扩展 — Grid / Connection / Transform 累积

扩展 tree/paint.rs 支持画布需要的三项基础能力：Grid 绘制、Connection
绘制、Transform 在 paint 中的累积应用。

设计要点（详见 spec）：
- 新类型 PaintTransform 封装累积的 translate + scale（跳过 rotate，
  Renderer 不支持旋转矩形，与 C.2 inverse_transform 对称）
- paint_node 签名增加 tf 参数，从 root 向下递归时通过 compose 复合
  遇到的 Transform 节点
- Grid 绘制：用 local rect 遍历格点，逐点 apply_point 后 draw_circle，
  dot_size 也按 scale 缩放
- Connection 绘制：线性遍历 tree 查找 port id，取 center_right/left
  锚点，计算贝塞尔 4 控制点（水平偏移启发式），调 draw_curve；
  宽度和颜色硬编码为模块常量；找不到 port 时静默跳过
- 纯函数提取到 tree/paint_helpers.rs（新建）：PaintTransform 类型、
  grid_cells、bezier_control_points、find_node_by_str_id、rect_center_*

新增 ~14 个单元测试（+1 个 ignored dry run）：
- PaintTransform 5 个：identity_compose / compose_nested /
  apply_point / apply_rect / ignores_rotate
- grid_cells 3 个：single_point / multi / zero_spacing
- bezier_control_points 3 个：horizontal / diagonal / reverse
- find_node_by_str_id 3 个：hit_root / hit_nested / miss

零 demo 改动：app/src/demo.rs 不修改，canvas/background.rs 继续
画 demo 的网格（不删），panel/renderer.rs 外部 API 不变。C.7 会
处理 demo 迁移到新架构。

非目标：
- 不做 NodeCardProps（留给 C.5.5 或 C.7）
- 不应用 rotate（Renderer 不支持旋转矩形）
- 不做 Image/Icon/Path/CustomPaint 的 paint
- 不改 LeafKind 变体定义
- 不改 Renderer（不动 wgpu 层）

测试总数：43（C.1~C.4）→ 57（C.5 新增 14）

Refs:
- docs/superpowers/specs/2026-04-11-phase-c5-paint-extensions-design.md
- docs/superpowers/plans/2026-04-11-phase-c5-paint-extensions.md

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 5.5: 确认 commit 成功**

Run: `git log --oneline -5`

Expected 最顶部：`feat(gui): C.5 paint 扩展 — Grid / Connection / Transform 累积`

- [ ] **Step 5.6: 最后一次完整测试运行**

Run: `cargo test -p gui 2>&1 | tail -10`

Expected: `test result: ok. 57 passed; 0 failed; 1 ignored`

---

## 完成标志

- ✅ `cargo test -p gui`：57 passed, 0 failed, 1 ignored（43 原有 + 14 新增）
- ✅ `cargo check --workspace`：无错误
- ✅ `cargo clippy -p gui`：无 C.5 新增警告
- ✅ `cargo build -p app --release`：编译通过
- ✅ `git log` 最顶部是 C.5 commit
- ✅ 文件改动：`tree/mod.rs` (modified)、`tree/paint.rs` (modified)、`tree/paint_helpers.rs` (new)

## 风险点速查

**风险 1：Renderer 在测试中无法构造（dry run 测试）**
- 位置：Task 2 Step 2.6
- 缓解：测试标记 `#[ignore]`，Task 4 Step 4.6 用 `cargo build -p app` 间接验证 paint.rs 不 panic（编译通过即部分证明）

**风险 2：paint.rs 的借用冲突**
- 位置：Task 4 Step 4.2 / 4.3
- 缓解：Step 4.3 的诊断指引——NLL 应该能处理，若不行则提前 clone 数据释放借用

**风险 3：`Shadow::clone` 不存在**
- 位置：Task 4 Step 4.2 的 `dec.shadow.clone()`
- 缓解：如果 Shadow 是 Copy，改为 `dec.shadow`；如果不是 Copy 也不是 Clone，需读 types.rs 决定

**风险 4：`grid_cells_multi` 的数量不对（15 vs 16）**
- 位置：Task 3 Step 3.5
- 缓解：确认 `while x <= rect.x + rect.w`（终点包含），产生 (0, 10, 20, 30) × (0, 10, 20, 30) = 16 个点

**风险 5：find_node_by_str_id 递归时的借用冲突**
- 位置：Task 3 Step 3.7
- 缓解：`let children: Vec<NodeId> = node.children.clone();` 先克隆，释放 `node` 借用，再在循环里递归

**风险 6：`font_size * tf.scale` 意外改变 C.4 的 Text 渲染**
- 位置：Task 4 Step 4.2
- 缓解：C.4 中没有 Transform 节点，`tf.scale = 1.0`，`font_size * 1.0 = font_size`，行为不变。C.4 测试继续 PASS 可验证。
