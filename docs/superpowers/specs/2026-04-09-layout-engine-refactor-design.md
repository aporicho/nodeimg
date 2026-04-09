# 布局引擎重构：消除中间 LayoutBox 转换

> 状态：已设计，待实施

## 问题

当前布局流程有一层多余的 `LayoutBox` 临时树：

```
PanelTree → build_layout_box() 构造临时树 → Flexbox 计算 → find_node_by_id() 按字符串回写 PanelNode
```

同时 Button 作为 Widget 叶子在 `paint()` 里手动居中文字（`button.rs:46-48`），绕过了布局引擎。这导致三个连锁问题：

1. **style() 返回不了引用** — Widget 节点的 BoxStyle 是运行时调 `w.layout(renderer)` 临时算出来的，不存在于任何持久位置
2. **Renderer 借用冲突** — 布局阶段需要 `&mut Renderer`（文字度量），与 `&mut PanelTree` 同时可变借用冲突
3. **滚动状态无归宿** — `LayoutResult` 删除后 `ScrollArea` 失去存储位置

## 核心洞察

**叶子应该是渲染图元（Text、Image），不是 Button 这样的组合控件。**

Button 分解为 Container（带 padding + 背景装饰）+ Text 叶子。文字居中由布局引擎的 `align_items: Center` 处理，不需要任何控件手动定位。

Text 叶子的尺寸是固有属性——在构建描述树时 `measure_text()` 一次，存为 `Size::Fixed`。到布局阶段，所有 BoxStyle 都是纯数据，不需要 Renderer。三个问题全部消失。

## 设计

### 1. 新增类型

**`widget/layout/types.rs`：**

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
/// 与 renderer::RectStyle 的区别：Decoration 是描述层的数据，RectStyle 是渲染命令的参数。
/// paint 时 Decoration → RectStyle 转换（background → color，shadow 暂不支持，后续按需添加）。
#[derive(Debug, Clone)]
pub struct Decoration {
    pub background: Option<Color>,
    pub border: Option<Border>,
    pub radius: [f32; 4],
}
```

### 2. 布局引擎泛型化

**`widget/layout/measure.rs`** — 签名改为泛型：
```rust
pub(crate) fn measure<T: LayoutTree>(tree: &T, node: T::NodeId) -> DesiredSize {
    let children = tree.children(node);
    if children.is_empty() {
        let style = tree.style(node);
        return DesiredSize {
            width: match style.width { Size::Fixed(w) => w, _ => 0.0 },
            height: match style.height { Size::Fixed(h) => h, _ => 0.0 },
        };
    }
    let child_sizes: Vec<DesiredSize> = children.iter().map(|&c| measure(tree, c)).collect();
    let style = tree.style(node);
    // ... 汇总逻辑与现有代码相同（Direction::Column/Row 分支）
}
```
递归模式从 `node.children.iter().map(measure)` 变为 `children.iter().map(|&c| measure(tree, c))`。

**`widget/layout/arrange.rs`** — 签名改为泛型：
```rust
pub(crate) fn arrange<T: LayoutTree>(tree: &mut T, node: T::NodeId, available: Rect)
```
主要变化：
- 删除 `rects`/`scroll_areas` 输出参数，直接调 `tree.set_rect(node, rect)`
- 滚动处理改用 `tree.scroll_offset(node)` / `tree.set_content_height(node, h)`
- 函数开头 clone 当前节点的 style：`let style = tree.style(node).clone()`（BoxStyle ~60 bytes，clone 零开销）
- **子节点样式预读**：循环开始前，把每个子节点的 flex_grow、width/height（用于 Fill 检测）等值提前读出到局部变量，避免在递归 `arrange(tree, child, ...)` 时与 `tree.style(child)` 产生借用冲突：
  ```rust
  let children = tree.children(node);
  let child_sizes: Vec<DesiredSize> = children.iter().map(|&c| measure(&*tree, c)).collect();
  // 预读子节点样式字段
  let child_styles: Vec<(f32, Size, Size)> = children.iter().map(|&c| {
      let s = tree.style(c);
      (s.flex_grow, s.width, s.height)
  }).collect();
  // 后续循环使用 child_styles[i] 而非 tree.style(child)
  ```

**`widget/layout/mod.rs`** — 入口改为泛型：
```rust
pub fn layout<T: LayoutTree>(tree: &mut T, root: T::NodeId, available: Rect) {
    arrange::arrange(tree, root, available);
}
```

**额外：`BoxStyle` 加 `Debug` derive**（当前只有 `Clone`），使包含 BoxStyle 的新类型能自动 derive Debug。

### 3. NodeKind 和 PanelNode 改造

**`panel/tree/node.rs`：**

```rust
pub enum NodeKind {
    /// 容器：布局样式 + 可选视觉装饰（背景/边框/圆角）
    Container { style: BoxStyle, decoration: Option<Decoration> },
    /// 叶子：渲染图元，尺寸已预计算存入 style
    Leaf { style: BoxStyle, kind: LeafKind },
}

pub struct PanelNode {
    pub id: &'static str,
    pub kind: NodeKind,
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub props_hash: u64,
    pub scroll_offset: f32,     // 持久化滚动位置
    pub content_height: f32,    // 内容总高度（用于 clamp）
}
```

滚动状态直接存在节点上。reconcile 更新 `kind` 时不触碰 `scroll_offset`，天然保留已有节点的滚动位置。

### 4. Desc 改造

**`panel/tree/desc.rs`：**

```rust
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
```

`into_parts()` 对应返回 `NodeKind::Container` 或 `NodeKind::Leaf`。

`props_hash()` 更新：
- `Desc::Container` — hash id + children.len() + decoration 状态（background color 等），确保按钮颜色切换触发 reconcile
- `Desc::Leaf` — hash id + LeafKind 全部字段（content、font_size、color），确保文字内容变化被检测到

### 5. PanelTree 实现 LayoutTree

**`panel/tree/layout.rs`** — 完全重写：

```rust
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
        if let Some(n) = self.get_mut(node) { n.rect = rect; }
    }

    fn scroll_offset(&self, node: NodeId) -> f32 {
        self.get(node).map(|n| n.scroll_offset).unwrap_or(0.0)
    }

    fn set_content_height(&mut self, node: NodeId, height: f32) {
        if let Some(n) = self.get_mut(node) { n.content_height = height; }
    }
}

pub fn layout(tree: &mut PanelTree, root: NodeId, available: Rect) {
    crate::widget::layout::layout(tree, root, available);
}
```

不需要 Adapter 包装，不需要 Renderer 参数。

`build_layout_box()` 和 `find_node_by_id()` 删除。

### 6. paint 和 hit_test 改造

**`panel/tree/paint.rs`** — 直接画图元：

- `NodeKind::Container { decoration: Some(dec), .. }` → `draw_rect(node.rect, ...)` 画背景
- `NodeKind::Leaf { kind: LeafKind::Text { .. }, .. }` → `draw_text(node.rect 位置, ...)` 画文字
- `overflow: Scroll` 的容器 → 画子节点前 `push_clip`，画完 `pop_clip`

文字位置由布局引擎决定，paint 只在 `node.rect` 处绘制，零手动偏移。

**`panel/tree/hit.rs`** — 有 decoration 的 Container 可命中，叶子不可命中：

```rust
match &node.kind {
    NodeKind::Container { decoration, .. } if decoration.is_some() => Some(node.id),
    _ => None,
}
```

### 7. App trait 和 demo 改造

**`shell/app.rs`** — update() 加 Renderer 参数：
```rust
fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext);
```

**`shell/runner.rs`** — 调用时传入：
```rust
state.app.update(&mut state.renderer, &mut state.ctx);
```

**`demo.rs`** — Button 从 Widget impl 变为函数：

```rust
fn button(id: &'static str, text_id: &'static str, label: &'static str,
          color: Color, renderer: &mut Renderer) -> Desc {
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
```

### 8. 滚动函数迁移

`widget/layout/scroll.rs` 删除。滚动操作改为直接修改 PanelNode：

```rust
// panel/tree/ 下
pub fn scroll(tree: &mut PanelTree, node_id: NodeId, delta: f32) {
    if let Some(node) = tree.get_mut(node_id) {
        let max = (node.content_height - node.rect.h).max(0.0);
        node.scroll_offset = (node.scroll_offset + delta).clamp(0.0, max);
    }
}
```

## 删除清单

| 目标 | 文件 |
|------|------|
| `LayoutBox` 结构体 | `widget/layout/types.rs` |
| `LayoutResult` 结构体 | `widget/layout/types.rs` |
| `ScrollArea` 结构体 | `widget/layout/types.rs` |
| `Widget` trait | `widget/trait.rs`（整个文件） |
| Button 的 Widget impl | `widget/atoms/button.rs`（改为函数） |
| `build_layout_box()` | `panel/tree/layout.rs` |
| `find_node_by_id()` | `panel/tree/layout.rs` |
| `widget/layout/scroll.rs` | 整个文件 |
| `NodeKind::Widget` variant | `panel/tree/node.rs` |
| `Desc::Widget` variant | `panel/tree/desc.rs` |

## 涉及文件

| 文件 | 操作 |
|------|------|
| `widget/layout/types.rs` | 新增 LayoutTree/LeafKind/Decoration，删除 LayoutBox/LayoutResult/ScrollArea |
| `widget/layout/measure.rs` | 改为泛型 `<T: LayoutTree>` |
| `widget/layout/arrange.rs` | 改为泛型，直接调 set_rect，clone style |
| `widget/layout/mod.rs` | 签名改为泛型 |
| `widget/layout/scroll.rs` | 删除 |
| `widget/trait.rs` | 删除 |
| `widget/atoms/button.rs` | Widget impl → 返回 Desc 的函数 |
| `widget/mod.rs` | 更新导出 |
| `panel/tree/node.rs` | NodeKind 改造，PanelNode 加 scroll_offset/content_height |
| `panel/tree/desc.rs` | Desc 加 Leaf/decoration，删 Widget，更新 props_hash() |
| `panel/tree/diff.rs` | create_from_desc 初始化 scroll_offset/content_height 为 0.0 |
| `panel/tree/layout.rs` | impl LayoutTree for PanelTree，删除中间转换 |
| `panel/tree/paint.rs` | 直接画图元 |
| `panel/tree/hit.rs` | decoration 命中检测 |
| `panel/tree/mod.rs` | 更新导出 |
| `shell/app.rs` | update() 加 `renderer: &mut Renderer` 参数 |
| `shell/runner.rs` | update() 调用传入 renderer |
| `demo.rs` | Button 函数化，layout() 不再传 renderer |

## 不受影响的文件

`widget/` 目录下以下文件不受本次重构影响，保持原样：
- `widget/focus.rs` — 焦点管理
- `widget/text_edit.rs` — 文字编辑状态
- `widget/action.rs`、`widget/mapping.rs`、`widget/state.rs` — 占位文件

## 验证

```bash
cargo build -p gui         # 编译通过
cargo run -p gui           # demo 面板显示两个按钮，点击切换颜色
cargo test --workspace     # 全量测试
```

验证点：
- 按钮渲染正确（背景色 + 居中文字 + 边框 + 圆角）
- 按钮点击响应正确（active 状态切换）
- 无限画布正常（拖拽/缩放不受影响）
- 面板拖拽/缩放正常
