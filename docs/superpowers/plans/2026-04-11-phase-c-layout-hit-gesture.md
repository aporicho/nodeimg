# 阶段 C.1+C.2+C.3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现阶段 C 的三个独立基础设施子阶段——Absolute 布局、HitChain 命中测试、ResizeRecognizer 手势识别器——同时保持 demo 行为零变化。

**Architecture:** 三个子阶段按依赖顺序(C.1 → C.2 → C.3)独立实现,每个独立 commit。每个子阶段遵循 TDD:先写全部失败测试,再一次实现到全部通过。整个阶段不改动 `app/src/demo.rs`、`widget/atoms/*`、旧 `panel/` 系统(drag/resize/frame/layer/hit)、canvas/,保证 demo 行为与 Phase B 结束时完全一致。

**Tech Stack:** Rust、`#[cfg(test)] mod tests` 单元测试、`cargo test -p gui` 验证。

**Spec:** `docs/superpowers/specs/2026-04-11-phase-c-layout-hit-gesture-design.md`

---

## 前置知识

### commit 策略

与 Phase B 的一次性 commit 不同,C 阶段本 plan 产生 **3 个 commit**,每个子阶段一个:

1. `feat(gui): C.1 布局引擎支持 Position::Absolute`
2. `feat(gui): C.2 hit test 返回 HitChain 支持 z-order / hittable / Transform`
3. `feat(gui): C.3 ResizeRecognizer 和 Gesture::Resize`

每个 commit 都独立可编译 + 单元测试通过 + demo 手测无回归。

### 测试基础设施

当前 `gui` crate **没有任何单元测试**(`cargo test --workspace` 显示 0 tests)。本 plan 在三个模块底部添加 `#[cfg(test)] mod tests`,合计 29 个单元测试。

测试使用真实的 `Tree` 结构(不用 mock),通过构造合成 `PanelNode` 和直接设置 `rect` 字段(PanelNode 的字段都是 pub)来搭建测试场景。

### 关键类型回顾

```rust
// gui/src/tree/node.rs
pub struct PanelNode {
    pub id: Cow<'static, str>,
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub kind: NodeKind,
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub scroll_offset: f32,
    pub content_height: f32,
}

pub enum NodeKind {
    Container,
    Leaf(LeafKind),
    Widget(Box<dyn WidgetProps>),
}

// gui/src/tree/layout/types.rs
pub enum Position {
    Flow,
    Absolute { x: f32, y: f32 },
}

pub struct Transform {
    pub translate: [f32; 2],
    pub scale: f32,
    pub rotate: f32,
}
```

### 依赖方向提醒

- `widget/` 不依赖 `gesture/`
- `gesture/` 依赖 `widget/`(通过 `Action`)
- `tree/` 核心(但 `tree/node.rs` 和 `tree/desc.rs` 有已知的反向依赖到 `widget::props::WidgetProps`,Phase C 后续解决)

这意味着 `ResizeEdge` 必须放 `widget/` 下(不能放 `gesture/`),否则会形成循环依赖。

---

## File Structure

### 本 plan 创建或修改的文件

| 路径 | 改动 | 所属 Task |
|------|-----|----------|
| `gui/src/tree/layout/arrange.rs` | 修改 + 新增 mod tests | Task 1-2 |
| `gui/src/tree/hit.rs` | 完全重写 + 新增 mod tests | Task 3-4 |
| `gui/src/tree/mod.rs` | 加 `pub use hit::HitChain` | Task 4 |
| `gui/src/panel/renderer.rs` | hit_test 内部适配 | Task 4 |
| `gui/src/widget/resize_edge.rs` | **新建** | Task 5 |
| `gui/src/widget/mod.rs` | 加 `pub mod resize_edge;` | Task 5 |
| `gui/src/widget/action.rs` | 加 ResizeEdge import + 3 个 ResizeAction 变体 | Task 5 |
| `gui/src/gesture/kind.rs` | 加 `Resize` 变体 | Task 5 |
| `gui/src/gesture/resize.rs` | **新建** + mod tests | Task 5-6 |
| `gui/src/gesture/mod.rs` | 加 `pub mod resize;` | Task 5 |

### 本 plan 不修改的文件(零改动保证)

- `app/src/demo.rs`
- `gui/src/widget/atoms/*.rs` (button/slider/text_input/toggle/dropdown)
- `gui/src/panel/drag.rs`, `resize.rs`, `frame.rs`, `layer.rs`, `hit.rs`
- `gui/src/canvas/*`
- `gui/src/tree/layout/{measure.rs, layout.rs, types.rs}` (除 Task 1 不动类型)
- `gui/src/tree/{tree.rs, node.rs, desc.rs, diff.rs, paint.rs, layout_adapter.rs, scroll.rs}`

---

## Tasks

### Task 1: C.1 · 添加 Absolute 布局失败测试

**Files:**
- Modify: `gui/src/tree/layout/arrange.rs`(底部加 `#[cfg(test)] mod tests`)

**Goal:** 写 6 个 arrange 单元测试,全部 fail,为 Task 2 的实现做红灯验证。

- [ ] **Step 1.1: 读取 arrange.rs 当前内容**

Read `gui/src/tree/layout/arrange.rs` 全文。确认末尾没有现存 tests 模块。

- [ ] **Step 1.2: 在 arrange.rs 末尾追加 tests 模块骨架**

使用 Edit 工具,在文件末尾(最后一行 `}` 之后)追加:

```rust

#[cfg(test)]
mod tests {
    // super::* 已从 arrange.rs 引入 Rect + types::*(BoxStyle/Position/Size/Direction/Overflow/Transform 等)
    use super::*;
    use crate::tree::tree::Tree;
    use crate::tree::node::{NodeKind, PanelNode};
    use std::borrow::Cow;

    /// 构造一个基础 Container 节点(decoration 无、子节点无)
    fn container(style: BoxStyle) -> PanelNode {
        PanelNode {
            id: Cow::Borrowed("test"),
            style,
            decoration: None,
            kind: NodeKind::Container,
            rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
            children: Vec::new(),
            scroll_offset: 0.0,
            content_height: 0.0,
        }
    }

    /// 不依赖字体的 measure 回调
    fn no_measure(_text: &str, _size: f32) -> (f32, f32) { (0.0, 0.0) }
}
```

- [ ] **Step 1.3: 添加 `absolute_simple` 测试**

在 tests 模块里(`no_measure` 函数之后)追加:

```rust
    #[test]
    fn absolute_simple() {
        // 父容器 800x600,padding=0,子节点 Absolute {x:100, y:50} + Fixed(100x80)
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 100.0, y: 50.0 },
            width: Size::Fixed(100.0),
            height: Size::Fixed(80.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(800.0),
            height: Size::Fixed(600.0),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, &mut measure);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.x, 100.0);
        assert_eq!(child.rect.y, 50.0);
        assert_eq!(child.rect.w, 100.0);
        assert_eq!(child.rect.h, 80.0);
    }
```

- [ ] **Step 1.4: 添加 `absolute_with_padding` 测试**

```rust
    #[test]
    fn absolute_with_padding() {
        // 父容器 padding=20,Absolute 子节点应该从 content 左上角算(20+100, 20+50)
        use crate::tree::layout::types::Edges;
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 100.0, y: 50.0 },
            width: Size::Fixed(100.0),
            height: Size::Fixed(80.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(800.0),
            height: Size::Fixed(600.0),
            padding: Edges::all(20.0),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, &mut measure);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.x, 120.0, "应该是 padding.left (20) + abs.x (100)");
        assert_eq!(child.rect.y, 70.0, "应该是 padding.top (20) + abs.y (50)");
    }
```

- [ ] **Step 1.5: 添加 `absolute_does_not_take_flex_space` 测试**

```rust
    #[test]
    fn absolute_does_not_take_flex_space() {
        // 父 Row 布局,两个 Flex 子(Fill)+ 一个 Absolute 子。
        // Flex 子应该各自占用 400(因为父宽 800,Absolute 不占空间)。
        let mut tree = Tree::new();

        let flex1_id = tree.insert(container(BoxStyle {
            flex_grow: 1.0,
            height: Size::Fixed(100.0),
            ..Default::default()
        }));
        let flex2_id = tree.insert(container(BoxStyle {
            flex_grow: 1.0,
            height: Size::Fixed(100.0),
            ..Default::default()
        }));
        let abs_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 200.0, y: 200.0 },
            width: Size::Fixed(50.0),
            height: Size::Fixed(50.0),
            ..Default::default()
        }));

        let mut root = container(BoxStyle {
            width: Size::Fixed(800.0),
            height: Size::Fixed(600.0),
            direction: Direction::Row,
            ..Default::default()
        });
        root.children = vec![flex1_id, flex2_id, abs_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, &mut measure);

        let flex1 = tree.get(flex1_id).unwrap();
        let flex2 = tree.get(flex2_id).unwrap();
        assert_eq!(flex1.rect.w, 400.0, "Flex1 应占一半");
        assert_eq!(flex2.rect.w, 400.0, "Flex2 应占一半");
        assert_eq!(flex1.rect.x, 0.0);
        assert_eq!(flex2.rect.x, 400.0);

        // Absolute 子节点正确定位
        let abs = tree.get(abs_id).unwrap();
        assert_eq!(abs.rect.x, 200.0);
        assert_eq!(abs.rect.y, 200.0);
    }
```

- [ ] **Step 1.6: 添加 `absolute_fixed_size` 测试**

```rust
    #[test]
    fn absolute_fixed_size() {
        // Absolute 节点用 Fixed 尺寸,应该和父 available 无关
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 50.0, y: 50.0 },
            width: Size::Fixed(200.0),
            height: Size::Fixed(150.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(400.0),
            height: Size::Fixed(300.0),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 }, &mut measure);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.w, 200.0);
        assert_eq!(child.rect.h, 150.0);
    }
```

- [ ] **Step 1.7: 添加 `absolute_in_scroll_container` 测试**

```rust
    #[test]
    fn absolute_in_scroll_container() {
        // 父 overflow=Scroll,两个 Flow 子节点(Column 方向,各高 200)+ 一个 Absolute 子。
        // content_height 应该是 400(2 * 200),不包括 Absolute。
        let mut tree = Tree::new();
        let flow1_id = tree.insert(container(BoxStyle {
            height: Size::Fixed(200.0),
            ..Default::default()
        }));
        let flow2_id = tree.insert(container(BoxStyle {
            height: Size::Fixed(200.0),
            ..Default::default()
        }));
        let abs_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 10.0, y: 10.0 },
            width: Size::Fixed(100.0),
            height: Size::Fixed(999.0),  // 超大,验证不会污染 content_height
            ..Default::default()
        }));

        let mut root = container(BoxStyle {
            width: Size::Fixed(400.0),
            height: Size::Fixed(300.0),
            direction: Direction::Column,
            overflow: Overflow::Scroll,
            ..Default::default()
        });
        root.children = vec![flow1_id, flow2_id, abs_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 }, &mut measure);

        let root = tree.get(root_id).unwrap();
        assert_eq!(root.content_height, 400.0, "scroll content_height 应只累计 Flow 子节点");
    }
```

- [ ] **Step 1.8: 添加 `transform_node_children_in_local_space` 测试**

```rust
    #[test]
    fn transform_node_children_in_local_space() {
        // 带 Transform 的父节点,子节点的 rect 应该在 local 空间(起点 0,0),
        // 而不是父节点的绝对位置加偏移。
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(100.0),
            height: Size::Fixed(50.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(400.0),
            height: Size::Fixed(300.0),
            transform: Some(Transform { translate: [10.0, 20.0], scale: 2.0, rotate: 0.0 }),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        // 父放在 (50, 60) 位置,但子节点应该相对 (0, 0) 不受父 absolute 位置影响
        arrange(&mut tree, root_id, Rect { x: 50.0, y: 60.0, w: 400.0, h: 300.0 }, &mut measure);

        let child = tree.get(child_id).unwrap();
        // Transform 父的子节点在 local 空间,起点是 (padding.left, padding.top) = (0, 0)
        assert_eq!(child.rect.x, 0.0, "Transform 父的 Flow 子应从 local origin (0,0) 开始");
        assert_eq!(child.rect.y, 0.0);
        assert_eq!(child.rect.w, 100.0);
        assert_eq!(child.rect.h, 50.0);
    }
```

- [ ] **Step 1.9: 运行测试,验证全部失败**

Run: `cargo test -p gui --lib tree::layout::arrange::tests 2>&1 | tail -30`
Expected: 6 个测试全部失败(FAILED)。典型错误:Absolute 子节点的 rect 是错的(因为 arrange 把它当 Flow 处理),transform 子节点坐标也错。

如果测试意外 PASS,说明测试本身有 bug 或对 arrange 的预期理解错了——重新检查。

- [ ] **Step 1.10: 不 commit,继续 Task 2**

---

### Task 2: C.1 · 实现 Absolute 布局 + Transform content 重置

**Files:**
- Modify: `gui/src/tree/layout/arrange.rs`(修改 `arrange` 函数主体)

**Goal:** 让 Task 1 的 6 个测试全部通过。

- [ ] **Step 2.1: 修改 arrange.rs 的 content 计算逻辑**

读取当前 arrange.rs(特别是 L40-50 附近的 content 计算)。用 Edit 把原先的:

```rust
    // 扣除 padding → 内容区域
    let content = Rect {
        x: node_rect.x + style.padding.left,
        y: node_rect.y + style.padding.top,
        w: (node_rect.w - style.padding.horizontal()).max(0.0),
        h: (node_rect.h - style.padding.vertical()).max(0.0),
    };
```

改为:

```rust
    // 扣除 padding → 内容区域
    // Transform 节点的子节点在 local 空间,起点相对 (0, 0) + padding;
    // 普通节点的子节点在绝对空间,起点相对 node_rect + padding。
    let content = if style.transform.is_some() {
        Rect {
            x: style.padding.left,
            y: style.padding.top,
            w: (node_rect.w - style.padding.horizontal()).max(0.0),
            h: (node_rect.h - style.padding.vertical()).max(0.0),
        }
    } else {
        Rect {
            x: node_rect.x + style.padding.left,
            y: node_rect.y + style.padding.top,
            w: (node_rect.w - style.padding.horizontal()).max(0.0),
            h: (node_rect.h - style.padding.vertical()).max(0.0),
        }
    };
```

- [ ] **Step 2.2: 在 arrange.rs 里把 children 按 position 分组**

读取当前 arrange.rs L39-42 附近:
```rust
    let children = tree.children(node);
    if children.is_empty() {
        return;
    }
```

用 Edit 把这段改为:

```rust
    let children = tree.children(node);
    if children.is_empty() {
        return;
    }

    // 按 position 分组:Flow 子节点走 Flex 流程,Absolute 子节点单独一趟
    let (flow_children, abs_children): (Vec<T::NodeId>, Vec<T::NodeId>) = children
        .iter()
        .copied()
        .partition(|&c| matches!(tree.style(c).position, Position::Flow));
```

注意:`T::NodeId` 是关联类型,`copied()` 要求 NodeId: Copy(已有约束)。`partition` 的返回类型需要显式标注,否则 Rust 推不出。

- [ ] **Step 2.3: 把 Flex 主循环从 `children` 替换为 `flow_children`**

在 arrange.rs 中定位 "处理滚动" 块(当前 L52 附近)。这段代码读 `children`:

```rust
    // 处理滚动
    let scroll_offset = if style.overflow == Overflow::Scroll {
        let offset = tree.scroll_offset(node);

        let child_sizes: Vec<DesiredSize> = children.iter().map(|&c| measure(&*tree, c, measure_text)).collect();
        let n = child_sizes.len();
        ...
```

把这一整段里所有 `children` 替换为 `flow_children`。定位方法:

- L56: `let child_sizes: Vec<DesiredSize> = children.iter()...` → `let child_sizes: Vec<DesiredSize> = flow_children.iter()...`

继续往下,L70 附近:
```rust
    // 度量子节点 + 预读子节点样式字段
    let is_column = style.direction == Direction::Column;
    let child_sizes: Vec<DesiredSize> = children.iter().map(...).collect();
    let child_styles: Vec<(f32, Size, Size, f32)> = children.iter().map(...).collect();
```

这两处也改成 `flow_children.iter()`.

L123 附近的主循环:
```rust
    // 排列子节点
    for (i, &child) in children.iter().enumerate() {
```
改为:
```rust
    // 排列子节点(仅 Flow)
    for (i, &child) in flow_children.iter().enumerate() {
```

- [ ] **Step 2.4: 在 arrange.rs 末尾(函数闭括号前)追加 Absolute 子节点循环**

定位 arrange() 函数的最后一行(现在是 flow children 循环之后),在 `}` 前追加:

```rust

    // ── Absolute 阶段:每个 Absolute 子节点独立 arrange ──
    for child in abs_children {
        let child_style = tree.style(child).clone();
        let (abs_x, abs_y) = match child_style.position {
            Position::Absolute { x, y } => (x, y),
            Position::Flow => unreachable!("partition 已保证这里只有 Absolute"),
        };

        // Absolute 子节点的"可用空间":从 (content.x + abs_x, content.y + abs_y) 开始的剩余区域
        let child_available = Rect {
            x: content.x + abs_x,
            y: content.y + abs_y,
            w: (content.w - abs_x).max(0.0),
            h: (content.h - abs_y).max(0.0),
        };

        arrange(tree, child, child_available, measure_text);
    }
```

- [ ] **Step 2.5: 运行测试,验证 C.1 的 6 个测试全部通过**

Run: `cargo test -p gui --lib tree::layout::arrange::tests 2>&1 | tail -30`
Expected: 6 个测试全部 PASS(`test result: ok. 6 passed`)。

如果失败,用错误信息定位问题。常见问题:
- `Position::Flow` 导入缺失:检查 arrange.rs 顶部 import
- `partition` 返回类型推断失败:加显式类型标注
- Transform 测试失败:确认 Step 2.1 的 if 分支正确

- [ ] **Step 2.6: cargo check 和 cargo clippy 验证**

Run: `cargo check -p gui 2>&1 | tail -10`
Expected: 无编译错误

Run: `cargo clippy -p gui -- -A warnings 2>&1 | tail -10`
Expected: 无新的 clippy 错误(使用 `-A warnings` 暂时静音预存警告,只看新增的)

- [ ] **Step 2.7: 确认 demo 仍然编译**

Run: `cargo build -p app --release 2>&1 | tail -10`
Expected: 编译通过

- [ ] **Step 2.8: commit C.1**

Run:

```bash
git add gui/src/tree/layout/arrange.rs && git commit -m "$(cat <<'EOF'
feat(gui): C.1 布局引擎支持 Position::Absolute

arrange() 函数按 position 分组子节点:Flow 子节点走 Flex 流程,
Absolute 子节点单独一趟,每个独立 arrange。Absolute 位置相对父
content(扣除 padding 后),符合 CSS 标准语义。

同时配套 C.2 的 Transform 逆变换约定:Transform 节点的子节点 rect
存在 local 空间(起点 (padding.left, padding.top)),而非绝对空间。
这让 C.2 hit_test 的逆变换能正确把外部坐标映射到 local 空间。

新增 6 个单元测试覆盖:
- absolute_simple: 基本 Absolute 定位
- absolute_with_padding: 父 padding 影响 Absolute 起点
- absolute_does_not_take_flex_space: Absolute 不占 flex 空间
- absolute_fixed_size: Fixed 尺寸不受 available 影响
- absolute_in_scroll_container: Absolute 不污染 scroll content_height
- transform_node_children_in_local_space: Transform 节点子节点起点 (0,0)

demo 行为零变化(demo 里没有 Absolute 或 Transform 节点)。

Refs:
- docs/superpowers/specs/2026-04-11-phase-c-layout-hit-gesture-design.md

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 2.9: 确认 commit 成功**

Run: `git log --oneline -3`
Expected: 最顶部是 `feat(gui): C.1 布局引擎支持 Position::Absolute`

---

### Task 3: C.2 · 添加 HitChain 脚手架 + 失败测试

**Files:**
- Modify: `gui/src/tree/hit.rs`(完全重写为含 HitChain 骨架 + tests 模块)
- Modify: `gui/src/tree/mod.rs`(添加 `pub use hit::HitChain`)

**Goal:** 定义 `HitChain` 类型并写 12 个 hit 单元测试,但 `hit_test` 函数仍返回空 HitChain 让所有测试失败。

- [ ] **Step 3.1: 读取当前 hit.rs 内容**

Read `gui/src/tree/hit.rs` 全文。当前 27 行,返回 `Option<&str>`。

- [ ] **Step 3.2: 完全重写 hit.rs 为 HitChain 骨架(函数体未实现)**

使用 Write 工具完全重写 `gui/src/tree/hit.rs`:

```rust
use super::node::NodeId;
use super::tree::Tree;

/// 从命中的叶子到根的节点链。
/// 链上 [0] 是最深的命中节点(叶子),[len-1] 是 root。
#[derive(Debug, Clone)]
pub struct HitChain {
    nodes: Vec<NodeId>,
}

impl HitChain {
    pub fn new(nodes: Vec<NodeId>) -> Self { Self { nodes } }
    pub fn empty() -> Self { Self { nodes: Vec::new() } }
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }
    pub fn len(&self) -> usize { self.nodes.len() }

    /// 最深的命中节点(叶子)
    pub fn leaf(&self) -> Option<NodeId> { self.nodes.first().copied() }
    /// 最外层的命中节点(root)
    pub fn root(&self) -> Option<NodeId> { self.nodes.last().copied() }

    /// 从叶子到根顺序遍历
    pub fn iter(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().copied()
    }

    pub fn contains(&self, id: NodeId) -> bool { self.nodes.contains(&id) }
}

/// 公开入口:返回命中链。如果没命中返回 empty HitChain。
pub fn hit_test(_tree: &Tree, _root: NodeId, _x: f32, _y: f32) -> HitChain {
    // TODO(C.2): 实际实现在 Task 4
    HitChain::empty()
}
```

注意:`_` 前缀用于未使用参数,避免编译警告。Task 4 会替换为真实实现。

- [ ] **Step 3.3: 更新 tree/mod.rs 的 re-export**

Read `gui/src/tree/mod.rs` 当前内容。找到 `pub use hit::hit_test;` 那行,用 Edit 改为:

```rust
pub use hit::{hit_test, HitChain};
```

- [ ] **Step 3.4: 适配 panel/renderer.rs 的 hit_test**

Read `gui/src/panel/renderer.rs`。找到当前 `hit_test` 方法(约 L35-38):

```rust
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&str> {
        let root = self.tree.root()?;
        hit_test(&self.tree, root, x, y)
    }
```

用 Edit 把函数体改为:

```rust
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&str> {
        let root = self.tree.root()?;
        let chain = hit_test(&self.tree, root, x, y);
        chain.leaf()
            .and_then(|id| self.tree.get(id))
            .map(|n| n.id.as_ref())
    }
```

- [ ] **Step 3.5: cargo check 确认骨架可编译**

Run: `cargo check -p gui 2>&1 | tail -10`
Expected: 编译通过(因为 hit_test 返回 empty chain,panel/renderer.rs::hit_test 内部调用不会报错)

此时 demo 的命中测试会永远失败(所有按钮/滑块 click 无反应),但这是预期的中间状态。

- [ ] **Step 3.6: 在 hit.rs 末尾追加 tests 模块骨架**

用 Edit 在 `pub fn hit_test(...)` 之后追加:

```rust

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{Color, Rect};
    use crate::tree::layout::types::{BoxStyle, Decoration, Position, Size, Transform};
    use crate::tree::node::{NodeKind, PanelNode};
    use std::borrow::Cow;

    /// 构造一个设定好 rect 的 Container 节点
    fn container_with_rect(style: BoxStyle, decoration: Option<Decoration>, rect: Rect) -> PanelNode {
        PanelNode {
            id: Cow::Borrowed("test"),
            style,
            decoration,
            kind: NodeKind::Container,
            rect,
            children: Vec::new(),
            scroll_offset: 0.0,
            content_height: 0.0,
        }
    }

    /// 默认 decoration(background 让节点可命中)
    fn decor() -> Decoration {
        Decoration {
            background: Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
            border: None,
            radius: [0.0; 4],
            shadow: None,
        }
    }
}
```

- [ ] **Step 3.7: 添加 `hit_empty_tree` 测试**

在 tests 模块追加:

```rust
    #[test]
    fn hit_empty_tree() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle::default(),
            None,
            Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert!(chain.is_empty(), "没 decoration 没 gestures 应该不可命中");
    }
```

- [ ] **Step 3.8: 添加 `hit_root_only` 测试**

```rust
    #[test]
    fn hit_root_only() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),  // 有 decoration → 可命中
            Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.leaf(), Some(root));
        assert_eq!(chain.root(), Some(root));
    }
```

- [ ] **Step 3.9: 添加 `hit_leaf_nested` 测试**

```rust
    #[test]
    fn hit_leaf_nested() {
        let mut tree = Tree::new();
        let leaf_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect { x: 20.0, y: 20.0, w: 30.0, h: 30.0 },
        ));
        let middle = {
            let mut n = container_with_rect(
                BoxStyle::default(),
                Some(decor()),
                Rect { x: 10.0, y: 10.0, w: 80.0, h: 80.0 },
            );
            n.children = vec![leaf_id];
            n
        };
        let middle_id = tree.insert(middle);
        let root = {
            let mut n = container_with_rect(
                BoxStyle::default(),
                Some(decor()),
                Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
            );
            n.children = vec![middle_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let chain = hit_test(&tree, root_id, 30.0, 30.0);
        assert_eq!(chain.len(), 3);
        // 叶子到根顺序
        let ids: Vec<_> = chain.iter().collect();
        assert_eq!(ids, vec![leaf_id, middle_id, root_id]);
    }
```

- [ ] **Step 3.10: 添加 `hit_reverse_z_order` 测试**

```rust
    #[test]
    fn hit_reverse_z_order() {
        // 两个重叠的兄弟节点,后添加的应该先命中(反向遍历)
        let mut tree = Tree::new();
        let child1_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect { x: 10.0, y: 10.0, w: 50.0, h: 50.0 },
        ));
        let child2_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect { x: 10.0, y: 10.0, w: 50.0, h: 50.0 },  // 完全重叠
        ));
        let root = {
            let mut n = container_with_rect(
                BoxStyle::default(),
                None,
                Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
            );
            n.children = vec![child1_id, child2_id];  // child2 是后添加的
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let chain = hit_test(&tree, root_id, 30.0, 30.0);
        assert_eq!(chain.leaf(), Some(child2_id), "后添加的 child2 应该先命中");
    }
```

- [ ] **Step 3.11: 添加 `hit_hittable_force_false` 测试**

```rust
    #[test]
    fn hit_hittable_force_false() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle {
                hittable: Some(false),  // 强制不可命中
                ..Default::default()
            },
            Some(decor()),  // 即使有 decoration
            Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert!(chain.is_empty(), "hittable=Some(false) 应强制不命中");
    }
```

- [ ] **Step 3.12: 添加 `hit_hittable_force_true` 测试**

```rust
    #[test]
    fn hit_hittable_force_true() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle {
                hittable: Some(true),  // 强制可命中
                ..Default::default()
            },
            None,  // 即使没 decoration 和 gestures
            Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.leaf(), Some(root));
    }
```

- [ ] **Step 3.13: 添加 `hit_gestures_makes_hittable` 测试**

```rust
    #[test]
    fn hit_gestures_makes_hittable() {
        use crate::gesture::Gesture;
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle {
                gestures: vec![Gesture::Tap],  // 有 gestures 声明
                ..Default::default()
            },
            None,  // 没 decoration
            Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert_eq!(chain.leaf(), Some(root), "有 gestures 应该可命中");
    }
```

- [ ] **Step 3.14: 添加 `hit_neither_gestures_nor_decoration` 测试**

```rust
    #[test]
    fn hit_neither_gestures_nor_decoration() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle::default(),  // 默认 hittable=None,gestures 空
            None,  // 无 decoration
            Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert!(chain.is_empty());
    }
```

- [ ] **Step 3.15: 添加 `hit_outside_bounds` 测试**

```rust
    #[test]
    fn hit_outside_bounds() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 200.0, 200.0);
        assert!(chain.is_empty());
    }
```

- [ ] **Step 3.16: 添加 `hit_transform_identity` 测试**

```rust
    #[test]
    fn hit_transform_identity() {
        // Transform 为 identity(translate=0, scale=1, rotate=0),行为应与无 transform 相同
        let mut tree = Tree::new();
        let child_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect { x: 10.0, y: 10.0, w: 30.0, h: 30.0 },  // 在 local 空间
        ));
        let root = {
            let mut n = container_with_rect(
                BoxStyle {
                    transform: Some(Transform {
                        translate: [0.0, 0.0],
                        scale: 1.0,
                        rotate: 0.0,
                    }),
                    ..Default::default()
                },
                None,
                Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 },
            );
            n.children = vec![child_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        // 屏幕点 (15, 15) → 进入 root → 逆变换到 local (15-0, 15-0) = (15, 15) → 命中 child (10,10,30,30)
        let chain = hit_test(&tree, root_id, 15.0, 15.0);
        assert_eq!(chain.leaf(), Some(child_id));
    }
```

- [ ] **Step 3.17: 添加 `hit_transform_translate` 测试**

```rust
    #[test]
    fn hit_transform_translate() {
        // Transform 平移 (50, 50):child local (10, 10) 被画到 screen (60, 60) 附近
        let mut tree = Tree::new();
        let child_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect { x: 10.0, y: 10.0, w: 20.0, h: 20.0 },  // local (10-30, 10-30)
        ));
        let root = {
            let mut n = container_with_rect(
                BoxStyle {
                    transform: Some(Transform {
                        translate: [50.0, 50.0],
                        scale: 1.0,
                        rotate: 0.0,
                    }),
                    ..Default::default()
                },
                None,
                Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 },
            );
            n.children = vec![child_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        // 屏幕 (70, 70) → 相对 root (70, 70) → 逆变换 (70-50, 70-50) = (20, 20) → 命中 child local (10,10,20,20)
        let chain = hit_test(&tree, root_id, 70.0, 70.0);
        assert_eq!(chain.leaf(), Some(child_id), "屏幕 (70,70) 应通过 translate(50,50) 逆变换到 local (20,20) 命中 child");
    }
```

- [ ] **Step 3.18: 添加 `hit_transform_scale` 测试**

```rust
    #[test]
    fn hit_transform_scale() {
        // Transform 缩放 2x:child local (10, 10) 被画到 screen (20, 20) 附近
        let mut tree = Tree::new();
        let child_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect { x: 10.0, y: 10.0, w: 10.0, h: 10.0 },  // local (10,10,10,10)
        ));
        let root = {
            let mut n = container_with_rect(
                BoxStyle {
                    transform: Some(Transform {
                        translate: [0.0, 0.0],
                        scale: 2.0,
                        rotate: 0.0,
                    }),
                    ..Default::default()
                },
                None,
                Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 },
            );
            n.children = vec![child_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        // 屏幕 (30, 30) → 相对 root (30, 30) → 逆变换 (30/2, 30/2) = (15, 15) → 命中 child local (10,10,10,10)
        let chain = hit_test(&tree, root_id, 30.0, 30.0);
        assert_eq!(chain.leaf(), Some(child_id), "屏幕 (30,30) 应通过 scale(2) 逆变换到 local (15,15) 命中 child");
    }
```

- [ ] **Step 3.19: 运行 hit 测试,验证全部失败**

Run: `cargo test -p gui --lib tree::hit::tests 2>&1 | tail -40`
Expected: 12 个测试全部失败(FAILED)。hit_test 骨架返回 empty HitChain,所有断言 HitChain 非空的测试都会失败。只有 `hit_empty_tree`、`hit_neither_gestures_nor_decoration`、`hit_outside_bounds`、`hit_hittable_force_false` 这 4 个断言 empty 的测试会意外 PASS —— 这是符合预期的(骨架行为恰巧和这些负样本一致)。

**如果只有 4 个 PASS、8 个 FAIL,符合预期,继续 Task 4。**

- [ ] **Step 3.20: 不 commit,继续 Task 4**

---

### Task 4: C.2 · 实现 HitChain hit_test 算法

**Files:**
- Modify: `gui/src/tree/hit.rs`(替换 hit_test 函数体 + 加辅助函数)

**Goal:** 让 Task 3 的 12 个测试全部通过。

- [ ] **Step 4.1a: 在 hit.rs 顶部添加缺失的 import**

读取 hit.rs 当前内容。找到文件顶部的:

```rust
use super::node::NodeId;
use super::tree::Tree;
```

用 Edit 替换为(添加 layout types 的 import,供新增的 is_hittable 和 inverse_transform 使用):

```rust
use super::layout::types::{BoxStyle, Transform};
use super::node::NodeId;
use super::tree::Tree;
```

- [ ] **Step 4.1b: 替换 hit_test 函数体 + 添加辅助函数**

找到 `pub fn hit_test(...)` 这部分:

```rust
/// 公开入口:返回命中链。如果没命中返回 empty HitChain。
pub fn hit_test(_tree: &Tree, _root: NodeId, _x: f32, _y: f32) -> HitChain {
    // TODO(C.2): 实际实现在 Task 4
    HitChain::empty()
}
```

用 Edit 工具替换上面那段为完整实现:

```rust
/// 公开入口:返回命中链。从 root 开始向下递归,返回的链从叶子到根。
/// 如果没命中返回 empty HitChain。
pub fn hit_test(tree: &Tree, root: NodeId, x: f32, y: f32) -> HitChain {
    let mut nodes = Vec::new();
    hit_recursive(tree, root, x, y, &mut nodes);
    HitChain::new(nodes)
}

fn hit_recursive(
    tree: &Tree,
    node_id: NodeId,
    x: f32,
    y: f32,
    chain: &mut Vec<NodeId>,
) -> bool {
    let Some(node) = tree.get(node_id) else { return false };

    // 1. 自身 bounds 检查(在当前坐标空间)
    let r = &node.rect;
    if x < r.x || x > r.x + r.w || y < r.y || y > r.y + r.h {
        return false;
    }

    // 2. Transform 节点:把当前点逆变换到子节点的 local 空间
    let (cx, cy) = match &node.style.transform {
        Some(tf) => inverse_transform(tf, x - r.x, y - r.y),
        None => (x, y),
    };

    // 3. 反向遍历子节点(实现 z-order:后渲染的先命中)
    for &child_id in node.children.iter().rev() {
        if hit_recursive(tree, child_id, cx, cy, chain) {
            chain.push(node_id);
            return true;
        }
    }

    // 4. 没有子命中,检查自身是否可命中
    if is_hittable(&node.style, &node.decoration) {
        chain.push(node_id);
        return true;
    }

    false
}

/// 混合判据:hittable 强制覆盖优先,否则 gestures 非空或 decoration 非空
fn is_hittable(style: &BoxStyle, decoration: &Option<super::layout::types::Decoration>) -> bool {
    match style.hittable {
        Some(h) => h,
        None => !style.gestures.is_empty() || decoration.is_some(),
    }
}

/// 仿射逆变换:把 (abs_x, abs_y)(相对 Transform 节点 origin)映射到子节点 local 空间
fn inverse_transform(tf: &Transform, abs_x: f32, abs_y: f32) -> (f32, f32) {
    // 正向:local → abs = (local.rotate(θ) * s) + translate
    // 逆向:abs → local = ((abs - translate) / s).rotate(-θ)
    let tx = abs_x - tf.translate[0];
    let ty = abs_y - tf.translate[1];
    let sx = tx / tf.scale;
    let sy = ty / tf.scale;
    let cos_r = (-tf.rotate).cos();
    let sin_r = (-tf.rotate).sin();
    (sx * cos_r - sy * sin_r, sx * sin_r + sy * cos_r)
}
```

- [ ] **Step 4.2: 运行 hit 测试,验证 12 个测试全部通过**

Run: `cargo test -p gui --lib tree::hit::tests 2>&1 | tail -40`
Expected: 12 个测试全部 PASS(`test result: ok. 12 passed`)

如果失败:
- `hit_leaf_nested` 失败:确认 chain push 顺序(`chain.push(node_id)` 在 child 递归成功之后)
- `hit_reverse_z_order` 失败:确认 `node.children.iter().rev()` 使用了 rev
- Transform 测试失败:确认逆变换数学(参考 spec 决策 5)

- [ ] **Step 4.3: 同时运行 C.1 测试,确保没有回归**

Run: `cargo test -p gui --lib tree::layout::arrange::tests 2>&1 | tail -10`
Expected: 6 个测试仍然 PASS

- [ ] **Step 4.4: cargo check 和 cargo clippy**

Run: `cargo check -p gui 2>&1 | tail -10`
Expected: 编译通过

Run: `cargo clippy -p gui -- -A warnings 2>&1 | tail -10`
Expected: 无新 clippy 错误

- [ ] **Step 4.5: 确认 demo 可编译 + 运行**

Run: `cargo build -p app --release 2>&1 | tail -10`
Expected: 编译通过

注:此时 demo 的 `self.gui.panel.hit_test` 已经走新的 HitChain.leaf() 取 leaf id 的路径。demo 行为应与 Phase B 完全一致(当前 widget 都没声明 gestures,但都有 decoration,混合判据下仍然可命中)。

- [ ] **Step 4.6: commit C.2**

Run:

```bash
git add gui/src/tree/hit.rs gui/src/tree/mod.rs gui/src/panel/renderer.rs && git commit -m "$(cat <<'EOF'
feat(gui): C.2 hit test 返回 HitChain 支持 z-order / hittable / Transform

完全重写 tree/hit.rs:

API 改变:
- hit_test 返回类型从 Option<&str> 改为 HitChain(叶到根的 NodeId 列表)
- 新增 HitChain 公共类型,带 is_empty / len / leaf / root / iter / contains 方法
- panel/renderer.rs::hit_test 内部适配:hit_test → chain.leaf() → node.id
  外部 Option<&str> 签名保留,demo.rs 零改动

语义改变:
- 反向遍历子节点(children.iter().rev()),实现正确 z-order(后渲染先命中)
- 混合判据:hittable 三态强制覆盖;None 时默认 "gestures 非空 OR
  decoration 非空"
- Transform 节点:遇到带 Transform 的节点,对当前点应用仿射逆变换
  (translate + scale + rotate),传给子节点递归
- 配合 C.1 arrange 的 Transform content 重置,Transform 节点的子节点
  rect 在 local 空间,逆变换把外部坐标映射进来

新增 12 个单元测试覆盖:
- hit_empty_tree, hit_root_only, hit_leaf_nested, hit_reverse_z_order
- hit_hittable_force_true/false, hit_gestures_makes_hittable
- hit_neither_gestures_nor_decoration, hit_outside_bounds
- hit_transform_identity/translate/scale

demo 行为零变化:demo 中的 widget 全部有 decoration,混合判据下仍然
可命中,且它们都不重叠,反向遍历结果等价于正向。

Refs:
- docs/superpowers/specs/2026-04-11-phase-c-layout-hit-gesture-design.md

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 4.7: 确认 commit 成功**

Run: `git log --oneline -3`
Expected: 最顶部是 `feat(gui): C.2 hit test 返回 HitChain 支持 z-order / hittable / Transform`

---

### Task 5: C.3 · 添加 ResizeEdge / Gesture::Resize / Action 变体 + ResizeRecognizer 失败测试

**Files:**
- Create: `gui/src/widget/resize_edge.rs`
- Modify: `gui/src/widget/mod.rs`
- Modify: `gui/src/widget/action.rs`
- Modify: `gui/src/gesture/kind.rs`
- Create: `gui/src/gesture/resize.rs`
- Modify: `gui/src/gesture/mod.rs`

**Goal:** 定义所有新类型(ResizeEdge、Resize gesture 变体、ResizeAction 变体),创建 ResizeRecognizer 骨架 + 12 个失败测试。

- [ ] **Step 5.1: 创建 widget/resize_edge.rs**

使用 Write 工具创建 `gui/src/widget/resize_edge.rs`:

```rust
/// Resize 手势命中的边或角。
///
/// 8 个方向覆盖矩形的所有可 resize 位置。TopLeft / TopRight / BottomLeft /
/// BottomRight 是角(两个方向同时 resize),Top / Bottom / Left / Right 是
/// 单方向边。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
```

- [ ] **Step 5.2: 更新 widget/mod.rs 声明 resize_edge 模块**

Read `gui/src/widget/mod.rs`。当前内容(大概是):

```rust
pub mod atoms;
pub mod props;

pub mod action;
mod focus;
mod mapping;
pub mod state;
mod text_edit;
```

用 Edit 在 `pub mod action;` 之后加一行 `pub mod resize_edge;`:

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

- [ ] **Step 5.3: 扩展 widget/action.rs,添加 3 个 ResizeAction 变体**

Read `gui/src/widget/action.rs`。当前内容(~11 行):

```rust
/// 手势识别后产出的动作。
#[derive(Debug, Clone)]
pub enum Action {
    Click(String),
    DoubleClick(String),
    DragStart { id: String, x: f32, y: f32 },
    DragMove { id: String, x: f32, y: f32 },
    DragEnd { id: String, x: f32, y: f32 },
    LongPress(String),
}
```

用 Write 工具完全重写为:

```rust
use super::resize_edge::ResizeEdge;

/// 手势识别后产出的动作。
#[derive(Debug, Clone)]
pub enum Action {
    Click(String),
    DoubleClick(String),
    DragStart { id: String, x: f32, y: f32 },
    DragMove { id: String, x: f32, y: f32 },
    DragEnd { id: String, x: f32, y: f32 },
    LongPress(String),
    ResizeStart { id: String, edge: ResizeEdge, x: f32, y: f32 },
    ResizeMove { id: String, edge: ResizeEdge, x: f32, y: f32 },
    ResizeEnd { id: String, edge: ResizeEdge, x: f32, y: f32 },
}
```

- [ ] **Step 5.4: 扩展 gesture/kind.rs,添加 Resize 变体**

Read `gui/src/gesture/kind.rs`。用 Edit 修改 enum,在 `LongPress,` 之后加 `Resize,`:

```rust
/// 节点可声明的手势类型。
///
/// 命中测试后,hit chain 上每个节点的 gestures 列表告诉手势竞技场
/// 应该注册哪些识别器。本枚举与 gesture/ 下已有的识别器一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gesture {
    Tap,
    DoubleTap,
    Drag,
    LongPress,
    Resize,
}
```

- [ ] **Step 5.5: 创建 gesture/resize.rs 骨架(结构 + 未实现的方法)**

使用 Write 工具创建 `gui/src/gesture/resize.rs`:

```rust
use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::renderer::Rect;
use crate::widget::action::Action;
use crate::widget::resize_edge::ResizeEdge;

const EDGE_THRESHOLD: f32 = 6.0;
const MOVE_THRESHOLD: f32 = 3.0;

pub struct ResizeRecognizer {
    target_id: String,
    node_rect: Rect,
    edge: Option<ResizeEdge>,
    down_x: f32,
    down_y: f32,
    current_x: f32,
    current_y: f32,
    resizing: bool,
    done: bool,
}

impl ResizeRecognizer {
    /// 构造时必须传入节点当前 rect。调用方(gesture arena 创建者)
    /// 从 tree.get(node_id).rect 读取。
    pub fn new(target_id: String, node_rect: Rect) -> Self {
        Self {
            target_id,
            node_rect,
            edge: None,
            down_x: 0.0,
            down_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            resizing: false,
            done: false,
        }
    }
}

/// 在 rect 的 8 个边/角附近(6px 阈值)内检测具体命中的是哪个。
/// 返回 None 表示点击在内部(非边缘)。
fn detect_edge(rect: &Rect, x: f32, y: f32) -> Option<ResizeEdge> {
    // TODO(C.3 Task 6): 实际实现
    let _ = (rect, x, y);
    None
}

impl GestureRecognizer for ResizeRecognizer {
    fn on_pointer_down(&mut self, _x: f32, _y: f32) -> bool {
        // TODO(C.3 Task 6)
        false
    }

    fn on_pointer_move(&mut self, _x: f32, _y: f32) -> GestureDisposition {
        // TODO(C.3 Task 6)
        GestureDisposition::Rejected
    }

    fn on_pointer_up(&mut self, _x: f32, _y: f32) -> GestureDisposition {
        // TODO(C.3 Task 6)
        GestureDisposition::Rejected
    }

    fn accept(&mut self) -> Action {
        // TODO(C.3 Task 6)
        Action::ResizeStart {
            id: self.target_id.clone(),
            edge: ResizeEdge::TopLeft,
            x: 0.0,
            y: 0.0,
        }
    }

    fn reject(&mut self) {}
}
```

- [ ] **Step 5.6: 更新 gesture/mod.rs 声明 resize 模块**

Read `gui/src/gesture/mod.rs`。当前内容大概是:

```rust
pub mod arena;
pub mod drag;
pub mod kind;
pub mod long_press;
pub mod recognizer;
pub mod tap;

pub use kind::Gesture;
```

用 Edit 加一行 `pub mod resize;`(在 recognizer 和 tap 之间按字母顺序):

```rust
pub mod arena;
pub mod drag;
pub mod kind;
pub mod long_press;
pub mod recognizer;
pub mod resize;
pub mod tap;

pub use kind::Gesture;
```

- [ ] **Step 5.7: cargo check 确认骨架可编译**

Run: `cargo check -p gui 2>&1 | tail -10`
Expected: 编译通过(所有新类型都定义了,ResizeRecognizer 骨架能编译但行为错误)

- [ ] **Step 5.8: 在 gesture/resize.rs 末尾加 tests 模块骨架**

用 Edit 在 resize.rs 最末尾(最后的 `}` 之后)追加:

```rust

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助:构造一个 100x100 的测试 rect
    fn rect_100() -> Rect {
        Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }
    }
}
```

- [ ] **Step 5.9: 添加 `detect_edge_corners` 测试**

```rust
    #[test]
    fn detect_edge_corners() {
        let r = rect_100();
        // 每个角距离两条边都 < 6
        assert_eq!(detect_edge(&r, 2.0, 2.0), Some(ResizeEdge::TopLeft));
        assert_eq!(detect_edge(&r, 98.0, 2.0), Some(ResizeEdge::TopRight));
        assert_eq!(detect_edge(&r, 2.0, 98.0), Some(ResizeEdge::BottomLeft));
        assert_eq!(detect_edge(&r, 98.0, 98.0), Some(ResizeEdge::BottomRight));
    }
```

- [ ] **Step 5.10: 添加 `detect_edge_sides` 测试**

```rust
    #[test]
    fn detect_edge_sides() {
        let r = rect_100();
        // 四条边中间位置,距离一条边 < 6,距离其他边 >= 6
        assert_eq!(detect_edge(&r, 50.0, 2.0), Some(ResizeEdge::Top));
        assert_eq!(detect_edge(&r, 50.0, 98.0), Some(ResizeEdge::Bottom));
        assert_eq!(detect_edge(&r, 2.0, 50.0), Some(ResizeEdge::Left));
        assert_eq!(detect_edge(&r, 98.0, 50.0), Some(ResizeEdge::Right));
    }
```

- [ ] **Step 5.11: 添加 `detect_edge_inside` 测试**

```rust
    #[test]
    fn detect_edge_inside() {
        let r = rect_100();
        // 内部点(距所有边 > 6)
        assert_eq!(detect_edge(&r, 50.0, 50.0), None);
        assert_eq!(detect_edge(&r, 20.0, 30.0), None);
    }
```

- [ ] **Step 5.12: 添加 `recognizer_rejects_interior_down` 测试**

```rust
    #[test]
    fn recognizer_rejects_interior_down() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        // 内部点击,on_pointer_down 应该返回 false
        assert!(!rec.on_pointer_down(50.0, 50.0));
    }
```

- [ ] **Step 5.13: 添加 `recognizer_accepts_edge_down` 测试**

```rust
    #[test]
    fn recognizer_accepts_edge_down() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        // 边缘点击,返回 true
        assert!(rec.on_pointer_down(2.0, 2.0));
        assert!(rec.on_pointer_down(50.0, 2.0));
        assert!(rec.on_pointer_down(2.0, 50.0));
    }
```

- [ ] **Step 5.14: 添加 `recognizer_pending_until_move_threshold` 测试**

```rust
    #[test]
    fn recognizer_pending_until_move_threshold() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        // 没动,应该 Pending
        assert_eq!(rec.on_pointer_move(2.0, 2.0), GestureDisposition::Pending);
        // 动了 1 像素,还不够 3px 阈值
        assert_eq!(rec.on_pointer_move(3.0, 3.0), GestureDisposition::Pending);
    }
```

- [ ] **Step 5.15: 添加 `recognizer_accept_on_sufficient_move` 测试**

```rust
    #[test]
    fn recognizer_accept_on_sufficient_move() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        // 动了 10 像素,超过 3px 阈值 → Accepted
        assert_eq!(rec.on_pointer_move(12.0, 12.0), GestureDisposition::Accepted);
    }
```

- [ ] **Step 5.16: 添加 `recognizer_reject_on_up_without_move` 测试**

```rust
    #[test]
    fn recognizer_reject_on_up_without_move() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        // 没动就 up → Rejected
        assert_eq!(rec.on_pointer_up(2.0, 2.0), GestureDisposition::Rejected);
    }
```

- [ ] **Step 5.17: 添加 `resize_start_action` 测试**

```rust
    #[test]
    fn resize_start_action() {
        let mut rec = ResizeRecognizer::new("panel_1".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);  // TopLeft
        rec.on_pointer_move(12.0, 12.0);  // Accepted
        match rec.accept() {
            Action::ResizeStart { id, edge, .. } => {
                assert_eq!(id, "panel_1");
                assert_eq!(edge, ResizeEdge::TopLeft);
            }
            other => panic!("期望 ResizeStart,实际 {:?}", other),
        }
    }
```

- [ ] **Step 5.18: 添加 `resize_move_action` 测试**

```rust
    #[test]
    fn resize_move_action() {
        let mut rec = ResizeRecognizer::new("panel_1".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        rec.on_pointer_move(12.0, 12.0);
        let _ = rec.accept();  // ResizeStart
        // 第二次 accept → ResizeMove
        match rec.accept() {
            Action::ResizeMove { id, edge, .. } => {
                assert_eq!(id, "panel_1");
                assert_eq!(edge, ResizeEdge::TopLeft);
            }
            other => panic!("期望 ResizeMove,实际 {:?}", other),
        }
    }
```

- [ ] **Step 5.19: 添加 `resize_end_action` 测试**

```rust
    #[test]
    fn resize_end_action() {
        let mut rec = ResizeRecognizer::new("panel_1".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        rec.on_pointer_move(12.0, 12.0);
        rec.on_pointer_up(12.0, 12.0);
        match rec.accept() {
            Action::ResizeEnd { id, edge, x, y } => {
                assert_eq!(id, "panel_1");
                assert_eq!(edge, ResizeEdge::TopLeft);
                assert_eq!(x, 12.0);
                assert_eq!(y, 12.0);
            }
            other => panic!("期望 ResizeEnd,实际 {:?}", other),
        }
    }
```

- [ ] **Step 5.20: 添加 `all_eight_edges_action_variants` 测试**

```rust
    #[test]
    fn all_eight_edges_action_variants() {
        // 对每个边缘位置,验证产出的 Action 带正确的 edge
        let cases = [
            (2.0, 2.0, ResizeEdge::TopLeft),
            (98.0, 2.0, ResizeEdge::TopRight),
            (2.0, 98.0, ResizeEdge::BottomLeft),
            (98.0, 98.0, ResizeEdge::BottomRight),
            (50.0, 2.0, ResizeEdge::Top),
            (50.0, 98.0, ResizeEdge::Bottom),
            (2.0, 50.0, ResizeEdge::Left),
            (98.0, 50.0, ResizeEdge::Right),
        ];
        for (x, y, expected_edge) in cases {
            let mut rec = ResizeRecognizer::new("p".to_string(), rect_100());
            assert!(rec.on_pointer_down(x, y), "down at ({}, {}) should succeed", x, y);
            rec.on_pointer_move(x + 20.0, y + 20.0);  // 足够 move 让它 resizing
            let action = rec.accept();
            match action {
                Action::ResizeStart { edge, .. } => {
                    assert_eq!(edge, expected_edge, "edge mismatch at ({}, {})", x, y);
                }
                other => panic!("期望 ResizeStart,实际 {:?}", other),
            }
        }
    }
```

- [ ] **Step 5.21: 运行 resize 测试,验证全部失败**

Run: `cargo test -p gui --lib gesture::resize::tests 2>&1 | tail -40`
Expected: 12 个测试全部失败。骨架返回 false / Rejected / 错误 edge,所有断言都会失败。

- [ ] **Step 5.22: 不 commit,继续 Task 6**

---

### Task 6: C.3 · 实现 ResizeRecognizer

**Files:**
- Modify: `gui/src/gesture/resize.rs`(实现 detect_edge、所有 trait 方法)

**Goal:** 让 Task 5 的 12 个测试全部通过。

- [ ] **Step 6.1: 实现 detect_edge 函数**

用 Edit 把 resize.rs 里的 detect_edge 函数(骨架)替换为实际实现:

```rust
/// 在 rect 的 8 个边/角附近(6px 阈值)内检测具体命中的是哪个。
/// 返回 None 表示点击在内部(非边缘)。
fn detect_edge(rect: &Rect, x: f32, y: f32) -> Option<ResizeEdge> {
    let near_left = (x - rect.x).abs() < EDGE_THRESHOLD;
    let near_right = (x - (rect.x + rect.w)).abs() < EDGE_THRESHOLD;
    let near_top = (y - rect.y).abs() < EDGE_THRESHOLD;
    let near_bottom = (y - (rect.y + rect.h)).abs() < EDGE_THRESHOLD;

    match (near_left, near_right, near_top, near_bottom) {
        (true, _, true, _) => Some(ResizeEdge::TopLeft),
        (true, _, _, true) => Some(ResizeEdge::BottomLeft),
        (_, true, true, _) => Some(ResizeEdge::TopRight),
        (_, true, _, true) => Some(ResizeEdge::BottomRight),
        (true, _, _, _) => Some(ResizeEdge::Left),
        (_, true, _, _) => Some(ResizeEdge::Right),
        (_, _, true, _) => Some(ResizeEdge::Top),
        (_, _, _, true) => Some(ResizeEdge::Bottom),
        _ => None,
    }
}
```

- [ ] **Step 6.2: 实现 on_pointer_down**

用 Edit 替换骨架的 on_pointer_down:

```rust
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x;
        self.down_y = y;
        self.current_x = x;
        self.current_y = y;
        // 边缘检测:不在边缘 → 返回 false(调用方可据此放弃注册该 recognizer)
        self.edge = detect_edge(&self.node_rect, x, y);
        self.edge.is_some()
    }
```

- [ ] **Step 6.3: 实现 on_pointer_move**

```rust
    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition {
        self.current_x = x;
        self.current_y = y;
        if self.edge.is_none() {
            return GestureDisposition::Rejected;
        }
        if self.resizing {
            return GestureDisposition::Accepted;
        }
        let dx = x - self.down_x;
        let dy = y - self.down_y;
        if dx * dx + dy * dy > MOVE_THRESHOLD * MOVE_THRESHOLD {
            self.resizing = true;
            GestureDisposition::Accepted
        } else {
            GestureDisposition::Pending
        }
    }
```

- [ ] **Step 6.4: 实现 on_pointer_up**

```rust
    fn on_pointer_up(&mut self, x: f32, y: f32) -> GestureDisposition {
        self.current_x = x;
        self.current_y = y;
        if self.resizing {
            self.done = true;
            GestureDisposition::Accepted
        } else {
            GestureDisposition::Rejected
        }
    }
```

- [ ] **Step 6.5: 实现 accept**

```rust
    fn accept(&mut self) -> Action {
        let edge = self.edge.expect("accept called before on_pointer_down succeeded");
        if self.done {
            Action::ResizeEnd {
                id: self.target_id.clone(),
                edge,
                x: self.current_x,
                y: self.current_y,
            }
        } else if self.resizing {
            Action::ResizeMove {
                id: self.target_id.clone(),
                edge,
                x: self.current_x,
                y: self.current_y,
            }
        } else {
            self.resizing = true;
            Action::ResizeStart {
                id: self.target_id.clone(),
                edge,
                x: self.current_x,
                y: self.current_y,
            }
        }
    }
```

- [ ] **Step 6.6: 运行 resize 测试,验证全部通过**

Run: `cargo test -p gui --lib gesture::resize::tests 2>&1 | tail -30`
Expected: 12 个测试全部 PASS

如果 `all_eight_edges_action_variants` 失败,调试:单个 case 失败时打印哪个 edge 不匹配,检查 detect_edge 的 match 顺序(角的优先级高于边)。

- [ ] **Step 6.7: 运行所有 C 相关测试,确保无回归**

Run: `cargo test -p gui --lib tree::layout::arrange::tests 2>&1 | tail -10`
Expected: 6 个 PASS

Run: `cargo test -p gui --lib tree::hit::tests 2>&1 | tail -10`
Expected: 12 个 PASS

Run: `cargo test -p gui --lib gesture::resize::tests 2>&1 | tail -10`
Expected: 12 个 PASS

- [ ] **Step 6.8: cargo check + clippy + build**

Run: `cargo check -p gui 2>&1 | tail -10`
Expected: 编译通过

Run: `cargo clippy -p gui -- -A warnings 2>&1 | tail -10`
Expected: 无新错误

Run: `cargo build -p app --release 2>&1 | tail -10`
Expected: 编译通过

- [ ] **Step 6.9: commit C.3**

Run:

```bash
git add gui/src/gesture/resize.rs gui/src/gesture/kind.rs gui/src/gesture/mod.rs gui/src/widget/resize_edge.rs gui/src/widget/mod.rs gui/src/widget/action.rs && git commit -m "$(cat <<'EOF'
feat(gui): C.3 ResizeRecognizer 和 Gesture::Resize

新增 resize 手势识别器,和 Tap/Drag/LongPress 同级作为声明式 gesture:

- widget/resize_edge.rs(新):定义 ResizeEdge 枚举(8 方向)
  遵循"一个文件一个职责"的哲学,不塞进 widget/action.rs

- widget/action.rs:加 ResizeStart / ResizeMove / ResizeEnd 三个 Action
  变体,payload 带 id / edge / x / y

- gesture/kind.rs:Gesture 枚举加 Resize 变体,供节点的
  style.gestures 声明使用

- gesture/resize.rs(新):ResizeRecognizer 实现
  - 构造时接收 target_id 和节点 rect(用于边缘检测)
  - 6px 边缘检测阈值,8 个方向(TopLeft / TopRight / ... / Right)
  - 3px 移动阈值后 accept(和 Drag 一致)
  - on_pointer_down 不在边缘 → 返回 false,让 Drag 自然胜出
  - 状态机:down → (move > 3px → ResizeStart) → move → move → up → ResizeEnd

新增 12 个单元测试覆盖:
- detect_edge_corners/sides/inside
- recognizer_rejects_interior_down, recognizer_accepts_edge_down
- recognizer_pending_until_move_threshold
- recognizer_accept_on_sufficient_move
- recognizer_reject_on_up_without_move
- resize_start/move/end_action
- all_eight_edges_action_variants

依赖方向:gesture → widget(通过 Action 和 ResizeEdge)。widget 不依赖
gesture,避免循环。

demo 行为零变化:demo 中没有节点声明 gestures: [Resize],
ResizeRecognizer 不会被实例化。真正用起来在 C.4+(Panel widget)。

已知 TODO:panel/resize.rs::ResizeEdge 和 widget/resize_edge.rs::ResizeEdge
是两个独立枚举,C.8 删除 panel/resize.rs 时统一。

Refs:
- docs/superpowers/specs/2026-04-11-phase-c-layout-hit-gesture-design.md

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 6.10: 确认 commit 成功**

Run: `git log --oneline -5`
Expected: 看到三个新 commit(C.1, C.2, C.3)按顺序排列:

```
<sha> feat(gui): C.3 ResizeRecognizer 和 Gesture::Resize
<sha> feat(gui): C.2 hit test 返回 HitChain 支持 z-order / hittable / Transform
<sha> feat(gui): C.1 布局引擎支持 Position::Absolute
<sha> docs(specs): 阶段 C.1+C.2+C.3 设计 —— 布局/命中/手势基础设施
<sha> fix(gui): 修复阶段 B 迁移带入的 clippy 警告
```

---

### Task 7: 最终验证

**Files:** 无改动

**Goal:** 全面验证 C.1+C.2+C.3 的三个 commit 完整无回归。

- [ ] **Step 7.1: 运行整个 gui crate 的全部测试**

Run: `cargo test -p gui 2>&1 | tail -20`
Expected: 30 个测试全部通过(29 个新增 + 0 个预存)。`test result: ok. 30 passed; 0 failed`

- [ ] **Step 7.2: cargo check**

Run: `cargo check --workspace 2>&1 | tail -10`
Expected: 无错误

- [ ] **Step 7.3: cargo clippy**

Run: `cargo clippy -p gui 2>&1 | tail -20`
Expected: 没有**本次 C 阶段新引入的**错误。Phase B 遗留的 pre-existing warnings(widget/text_edit.rs dead_code 等)可以存在,但本次不新增。

如果发现本次新增的 clippy 警告(比如 unused import),用 Edit 修复,再 `cargo clippy` 验证。修复后需要追加到 C.3 的 commit 还是新开 fix commit?**如果是简单的 unused import,可以 amend 到 C.3 commit**(`git commit --amend --no-edit`);如果是复杂改动,新开一个 `fix(gui): clippy 清理` commit。

- [ ] **Step 7.4: cargo build release**

Run: `cargo build -p app --release 2>&1 | tail -10`
Expected: 编译通过

- [ ] **Step 7.5: demo 手测**

Run: `cargo run -p app --release`

手测项目(每个都要验证):
- [ ] 窗口打开,面板(toolbar)可见
- [ ] 按钮可点击(有视觉反馈)
- [ ] 滑块可拖拽
- [ ] 面板可从 title 拖拽移动
- [ ] 面板边缘可 resize
- [ ] 行为与 Phase B 结束时(`fbd4d4e`)完全一致

**关闭窗口前**,对比一下交互:所有 demo 的 pan / click / drag / resize 应该都正常。**任何行为差异都是 bug**。

如果发现行为差异:
1. 不要 commit 任何新改动
2. 用 `git diff HEAD~3 HEAD -- gui/` 审阅本次 C 的所有改动
3. 定位差异来源(可能是 hit_test 的新判据、或 arrange 的 partition 顺序)
4. 修复后回到 Task 对应的 Step 重跑测试

- [ ] **Step 7.6: git log 验证**

Run: `git log --oneline main..HEAD`
Expected: 4 个 commit(spec + 3 个 feat):

```
<sha> feat(gui): C.3 ResizeRecognizer 和 Gesture::Resize
<sha> feat(gui): C.2 hit test 返回 HitChain 支持 z-order / hittable / Transform
<sha> feat(gui): C.1 布局引擎支持 Position::Absolute
<sha> docs(specs): 阶段 C.1+C.2+C.3 设计 —— 布局/命中/手势基础设施
```

- [ ] **Step 7.7: git status 确认 clean**

Run: `git status`
Expected: `nothing to commit, working tree clean`

---

## 回滚策略

每个子阶段是独立 commit,可以用 `git reset --hard HEAD~N` 单独撤销某几个 commit 回到之前的状态。

最常见的回滚场景:

```bash
# 撤销最后一个 commit(保留 worktree 的修改)
git reset --soft HEAD~1

# 完全撤销并删除修改(谨慎)
git reset --hard HEAD~1

# 撤销最后 3 个 feat commit,保留 spec
git reset --hard HEAD~3
```

如果在 Task 中间发现测试失败无法修复,用 `git stash` 暂存当前修改,回到 Task 起点重新开始:

```bash
git stash
# 修复或重写代码
git stash drop  # 最后扔掉旧的
```

## 预估工作量

| Task | 步骤数 | 代码量 | 预估 |
|------|-------|-------|------|
| Task 1(C.1 tests) | 10 | ~200 行测试 | 40 分钟 |
| Task 2(C.1 impl)  | 9  | ~50 行 arrange 修改 | 30 分钟 |
| Task 3(C.2 tests) | 20 | ~400 行测试 + 骨架 | 90 分钟 |
| Task 4(C.2 impl)  | 7  | ~100 行 hit_test 重写 | 30 分钟 |
| Task 5(C.3 tests) | 22 | ~250 行 + 骨架 | 90 分钟 |
| Task 6(C.3 impl)  | 10 | ~140 行 Resize 实现 | 40 分钟 |
| Task 7(最终验证)  | 7  | 0 | 20 分钟 |

**总计约 5-6 小时**(一次 session 可以完成)。

## 已知 TODO(不在本 plan scope)

- **Transform 正向变换应用到 paint**:C.2 实现了逆变换和 content 重置,但 paint.rs 没应用 transform。C.5 canvas 分支实现时补齐
- **声明式 arena 创建**:demo.rs 的 arena 创建逻辑保持不变,C.6/C.7 改为按 hit chain 的 gestures 字段自动构造 recognizer
- **Widget atoms 的 gestures 声明**:按钮、滑块等目前没声明 gestures,混合判据下仍可命中。C.4+ 按需补充
- **ResizeEdge 双枚举并存**:widget/resize_edge.rs 和 panel/resize.rs 的 ResizeEdge 独立,C.8 统一
- **tree → widget::props 反向依赖**:Phase B 已知 TODO,阶段 C 后续解决
