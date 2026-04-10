# Widget Props（配置）系统设计

## 背景

当前 `gui/` 框架的 Desc → reconcile → layout → paint 管线已跑通，但控件只有 demo.rs 中的 `button()` 纯函数，且 `LeafKind` 只有 `Text` 一个变体。框架无法表达交互控件（Slider、Toggle、Dropdown 等）的类型化配置。

根据 2.12.0-widget-anatomy.md §2（配置/Props）的规范：
- Props 是从外部传入的不可变参数，描述"这个控件是什么样的"
- Props 是只读的，控件自己不修改
- Props 变化由外部驱动（父组件传入新配置 → diff 检测 → 更新控件）

本设计解决：如何让框架支持类型化的控件 Props，并在树上保留控件的语义信息。

## 设计决策

### 1. 树上保留控件语义

Desc 新增 `Widget` 变体，携带控件类型和 Props。

树的层级为：**面板支树 → 面板 → 控件（Widget）→ 图元（Container/Leaf）**。

查看树即可知道"这是一个 Slider，label 是半径，range 是 0..10，当前 value 是 5.0"，不是纯渲染树。

树节点（PanelNode）将 `style` 和 `decoration` 提到节点级别，所有节点统一持有。NodeKind 只存差异部分：Container 无字段、Leaf 存内容、Widget 存 Props。这样 layout/paint/hit 直接读 `node.style` 和 `node.decoration`，不需要为 Widget 写额外的匹配分支。

### 2. Props 使用 trait object（开放扩展）

Props 定义为 trait 而非 enum。每种控件独立定义 Props 结构体并实现 `WidgetProps` trait。

理由：
- 加新控件不需要修改已有代码（开放-封闭）
- 控件的 Props 定义和 build 逻辑自包含在一个 impl 里
- 控件集将来可能扩展（自定义控件）

代价是 `as_any`/`clone_box`/`props_eq` 样板代码，后续可用宏消除。

### 3. build() 是纯函数，不依赖渲染

`build()` 只描述控件的图元结构，Text 叶子的尺寸用 `Size::Auto`。文字测量在 layout 的 resolve 阶段完成，由独立的 `TextMeasurer` 负责。

这保证了 build() 和 reconcile 都是纯粹的树操作，不碰渲染。

`build()` 返回 `WidgetBuild` 结构体，显式提供 Widget 节点的根样式、装饰和子树：

```rust
pub struct WidgetBuild {
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub children: Vec<Desc>,
}
```

### 4. TextMeasurer 从 Renderer 拆出

现有的文字测量链路：`Renderer → TextPipeline → FontSystem`。量文字只需要 `FontSystem` + buffer cache，不需要 GPU 资源。

将 `FontSystem` 和 `buffer_cache` 从 `TextPipeline` 拆出为独立的 `TextMeasurer`：

```rust
/// 文字测量。只依赖字体系统，不依赖 GPU。
pub struct TextMeasurer {
    font_system: FontSystem,
    buffer_cache: HashMap<TextCacheKey, CachedBuffer>,
}

impl TextMeasurer {
    pub fn measure(&mut self, text: &str, size: f32) -> (f32, f32) { ... }
}
```

- **layout/resolve** 使用 `TextMeasurer` 量文字
- **TextPipeline** 借用 `FontSystem` 做 draw_text 排版
- 两者共享同一个 `FontSystem`，但 layout 不依赖 Renderer

### 5. layout/resolve.rs — 内容尺寸解析

新增 resolve 阶段，在 layout 的 measure/arrange 之前执行。遍历树，把 `Size::Auto` 的 Text 叶子解析为具体尺寸：

```rust
/// 解析叶子节点的内容尺寸。Text → 文字测量，将来 Image → 图片尺寸。
pub fn resolve(tree: &mut PanelTree, measurer: &mut TextMeasurer) {
    for node in tree.iter_mut() {
        if let NodeKind::Leaf(LeafKind::Text { content, font_size, .. }) = &node.kind {
            if node.style.width == Size::Auto {
                let (w, h) = measurer.measure(content, *font_size);
                node.style.width = Size::Fixed(w);
                node.style.height = Size::Fixed(h);
            }
        }
    }
}
```

完整管线变为：

```
view() → reconcile → resolve → measure → arrange → paint
  纯描述    纯对比    量文字    算尺寸    分配位置    画
```

每个阶段职责单一：
- **view()** — 纯函数，构建 Desc 描述树
- **reconcile** — 纯对比，更新 PanelTree
- **resolve** — 用 TextMeasurer 解析 Auto 尺寸
- **measure/arrange** — 纯布局计算
- **paint** — 用 Renderer 画

### 6. 文件结构：类型提到 widget/，树操作留在 panel/tree/

Desc 和 NodeKind 是面板和画布共享的类型定义，从 `panel/tree/` 移到 `widget/`。树操作（reconcile、paint、hit）留在 `panel/tree/`，将来画布做的时候再看是否抽泛型。

### 7. value 是 Props 的一部分

`value` 从外部读取后作为 Props 传入控件。控件只读 value 用于渲染，不修改它。用户交互时控件产出 Action，由上层处理后更新数据源，触发新一轮 view() → reconcile。

## 详细设计

### WidgetProps trait

```rust
use std::any::Any;
use std::fmt;

/// 控件展开结果。build() 返回此结构，提供 Widget 节点的根样式和子树。
pub struct WidgetBuild {
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub children: Vec<Desc>,
}

/// 控件配置 trait。每种控件实现此 trait，定义自己的 Props 和展开逻辑。
pub trait WidgetProps: 'static {
    /// 控件类型名（调试、检查用）
    fn widget_type(&self) -> &'static str;

    /// 向下转型，用于 PartialEq 比较
    fn as_any(&self) -> &dyn Any;

    /// Clone 到 Box
    fn clone_box(&self) -> Box<dyn WidgetProps>;

    /// 比较两个 Props 是否相同（reconcile 用）
    fn props_eq(&self, other: &dyn WidgetProps) -> bool;

    /// Debug 格式化
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;

    /// 从 Props 展开成图元子树。纯函数，不依赖渲染。
    /// Text 叶子使用 Size::Auto，文字测量由 layout/resolve 阶段处理。
    /// - `id`: 控件自身的 id，用于生成子节点 id（如 `format!("{id}::track")`）
    fn build(&self, id: &str) -> WidgetBuild;
}

impl Clone for Box<dyn WidgetProps> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn WidgetProps> {
    fn eq(&self, other: &Self) -> bool {
        self.props_eq(other.as_ref())
    }
}

impl fmt::Debug for Box<dyn WidgetProps> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_fmt(f)
    }
}
```

### PanelNode 变更

`style` 和 `decoration` 从 NodeKind 提到 PanelNode 级别。所有节点统一持有，layout/paint/hit 直接读 `node.style` / `node.decoration`，不需要 match NodeKind。

```rust
pub struct PanelNode {
    pub id: Cow<'static, str>,
    pub style: BoxStyle,                // 所有节点共有
    pub decoration: Option<Decoration>, // 所有节点共有（Leaf 通常为 None）
    pub kind: NodeKind,                 // 只放差异部分
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub scroll_offset: f32,
    pub content_height: f32,
}
```

reconcile 的变化检测通过辅助方法封装：

```rust
impl PanelNode {
    pub fn props_match(&self, style: &BoxStyle, decoration: &Option<Decoration>, kind: &NodeKind) -> bool {
        self.style == *style && self.decoration == *decoration && self.kind == *kind
    }
}
```

### NodeKind 变更

NodeKind 精简为只存差异部分：

```rust
#[derive(PartialEq)]
pub enum NodeKind {
    Container,
    Leaf(LeafKind),
    Widget(Box<dyn WidgetProps>),
}
```

`Box<dyn WidgetProps>` 的手动 `PartialEq` impl 满足 derive 的要求。

### Desc 变更

```rust
pub enum Desc {
    Container {
        id: Cow<'static, str>,
        style: BoxStyle,
        decoration: Option<Decoration>,
        children: Vec<Desc>,
    },
    Leaf {
        id: Cow<'static, str>,
        style: BoxStyle,
        kind: LeafKind,
    },
    Widget {
        id: Cow<'static, str>,
        props: Box<dyn WidgetProps>,
    },
}
```

现有的 `Desc::into_parts()` 方法假设所有变体都有 children 和 NodeKind，Widget 不适用此模式（Widget 的 children 由 `build()` 动态生成，不在 Desc 中）。reconcile 改为直接 match Desc 变体，不再通过 `into_parts()` 统一解构。`into_parts()` 移除。

### 各控件 Props 定义

每个控件一个结构体，实现 `WidgetProps` trait。

```rust
// atoms/button.rs
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonProps {
    pub label: Cow<'static, str>,
    pub icon: Option<Cow<'static, str>>,
    pub disabled: bool,
}

// atoms/slider.rs
#[derive(Clone, Debug, PartialEq)]
pub struct SliderProps {
    pub label: Cow<'static, str>,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub value: f32,
    pub disabled: bool,
}

// atoms/toggle.rs
#[derive(Clone, Debug, PartialEq)]
pub struct ToggleProps {
    pub label: Cow<'static, str>,
    pub value: bool,
    pub disabled: bool,
}

// atoms/text_input.rs
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputProps {
    pub label: Cow<'static, str>,
    pub value: Cow<'static, str>,
    pub disabled: bool,
}

// atoms/dropdown.rs
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownProps {
    pub label: Cow<'static, str>,
    pub options: Vec<Cow<'static, str>>,
    pub selected: usize,
    pub disabled: bool,
}
```

每个 Props 的 `WidgetProps` impl 包含样板方法（`as_any`、`clone_box`、`props_eq`、`debug_fmt`）和 `build()` 方法。本次 `build()` 用 `todo!()` 占位，具体的控件展开逻辑（轨道、滑块、开关等图元组合）在后续实现各控件时完成。

### reconcile 变更

reconcile 是纯树操作，不需要 renderer。不再使用 `into_parts()` 统一解构，改为直接 match 三个 Desc 变体。style/decoration 写到 `node` 级别，kind 只存差异部分。

```rust
pub fn reconcile(tree: &mut PanelTree, desc: Desc) {
    if let Some(root_id) = tree.root() {
        reconcile_node(tree, root_id, desc);
    } else {
        let root_id = create_from_desc(tree, desc);
        tree.set_root(root_id);
    }
}

fn reconcile_node(tree: &mut PanelTree, node_id: NodeId, desc: Desc) {
    match desc {
        Desc::Container { id, style, decoration, children } => {
            if let Some(node) = tree.get_mut(node_id) {
                if !node.props_match(&style, &decoration, &NodeKind::Container) {
                    node.style = style;
                    node.decoration = decoration;
                    node.kind = NodeKind::Container;
                }
            }
            reconcile_children(tree, node_id, children);
        }

        Desc::Leaf { id, style, kind } => {
            let new_kind = NodeKind::Leaf(kind);
            if let Some(node) = tree.get_mut(node_id) {
                if !node.props_match(&style, &None, &new_kind) {
                    node.style = style;
                    node.decoration = None;
                    node.kind = new_kind;
                }
            }
        }

        Desc::Widget { id, props } => {
            // Props 变了才重新 build
            let props_changed = tree.get(node_id)
                .map(|n| match &n.kind {
                    NodeKind::Widget(old) => !old.props_eq(props.as_ref()),
                    _ => true,
                })
                .unwrap_or(true);

            if props_changed {
                let wb = props.build(&id);
                if let Some(node) = tree.get_mut(node_id) {
                    node.style = wb.style;
                    node.decoration = wb.decoration;
                    node.kind = NodeKind::Widget(props);
                }
                reconcile_children(tree, node_id, wb.children);
            }
        }
    }
}
```

`create_from_desc` 同样按 Desc 变体分别处理，Widget 变体调用 `build()` 创建子树。

### paint / hit / layout

style 和 decoration 在 PanelNode 级别，layout/paint/hit 直接读取，不需要为 Widget 写额外匹配分支：

- **layout** — `node.style` 直接读取。LayoutTree impl 的 `style()` 返回 `&node.style`，无需 match kind。layout 之前先执行 resolve 解析 Text 尺寸。
- **paint** — `node.decoration` 直接读取。有 decoration 就 `draw_rect`，然后递归子节点。只有 Leaf 需要 match kind 画内容（Text → draw_text）。
- **hit** — `node.decoration.is_some()` 直接判断，无需 match kind。

### 使用示例

面板的 view() 函数中构建控件树：

```rust
fn build_properties_panel(params: &Params) -> Desc {
    Desc::Container {
        id: Cow::Borrowed("properties"),
        style: BoxStyle { direction: Direction::Column, gap: 8.0, ..Default::default() },
        decoration: None,
        children: vec![
            Desc::Widget {
                id: Cow::Borrowed("radius"),
                props: Box::new(SliderProps {
                    label: "半径".into(),
                    min: 0.0,
                    max: 10.0,
                    step: 0.1,
                    value: params.radius,
                    disabled: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("hdr"),
                props: Box::new(ToggleProps {
                    label: "HDR".into(),
                    value: params.hdr,
                    disabled: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("execute"),
                props: Box::new(ButtonProps {
                    label: "执行".into(),
                    icon: None,
                    disabled: false,
                }),
            },
        ],
    }
}
```

view() 不需要任何渲染资源。

对应的树结构：

```
Container("properties")
├── Widget(Slider { label: "半径", min: 0, max: 10, step: 0.1, value: 5.0 })
│   ├── Leaf(Text "半径")           ← Size::Auto，resolve 阶段量好
│   ├── Container(轨道 decoration)
│   ├── Container(滑块 decoration)
│   └── Leaf(Text "5.0")           ← Size::Auto，resolve 阶段量好
├── Widget(Toggle { label: "HDR", value: true })
│   ├── Container(开关轨道 decoration)
│   ├── Container(开关圆点 decoration)
│   └── Leaf(Text "HDR")
└── Widget(Button { label: "执行" })
    └── Leaf(Text "执行")
```

## 文件结构变更

### 移动

| 原位置 | 新位置 | 原因 |
|--------|--------|------|
| `panel/tree/desc.rs` | `widget/desc.rs` | Desc 是面板和画布共享的类型 |
| `panel/tree/node.rs` | `widget/node.rs` | NodeKind 同上 |

移动后，`panel/tree/` 下的 `diff.rs`、`paint.rs`、`hit.rs`、`layout.rs` 将导入路径从 `super::desc` / `super::node` 改为 `crate::widget::desc` / `crate::widget::node`。`panel/tree/mod.rs` 移除 `mod desc; mod node;`，改为 `use crate::widget::{desc::Desc, node::{NodeId, NodeKind}};` 并 re-export。

### 新增

| 文件 | 内容 |
|------|------|
| `widget/props.rs` | `WidgetProps` trait、`WidgetBuild` 结构体、`Box<dyn WidgetProps>` 的 Clone/PartialEq/Debug impl |
| `widget/atoms/button.rs` | `ButtonProps` + `impl WidgetProps`（重写，替换现有占位注释） |
| `widget/atoms/slider.rs` | `SliderProps` + `impl WidgetProps` |
| `widget/atoms/toggle.rs` | `ToggleProps` + `impl WidgetProps` |
| `widget/atoms/text_input.rs` | `TextInputProps` + `impl WidgetProps` |
| `widget/atoms/dropdown.rs` | `DropdownProps` + `impl WidgetProps` |
| `widget/layout/resolve.rs` | resolve 函数：遍历树，用 TextMeasurer 解析 Auto 尺寸的 Text 叶子 |
| `renderer/text_measurer.rs` | `TextMeasurer` 结构体：从 TextPipeline 拆出的 FontSystem + buffer_cache |

### 修改

| 文件 | 变更 |
|------|------|
| `widget/mod.rs` | 新增 `pub mod desc, node, props` 导出 |
| `widget/atoms/mod.rs` | 导出各控件模块（button, slider, toggle, text_input, dropdown） |
| `widget/layout/mod.rs` | 新增 `pub mod resolve` 导出 |
| `widget/layout/types.rs` | `LeafKind::Text` 注释更新（移除"尺寸在创建时预算"的描述） |
| `renderer/pipeline/text.rs` | `FontSystem` 和 `buffer_cache` 移出到 `TextMeasurer`；`TextPipeline` 改为借用 `FontSystem` |
| `renderer/renderer.rs` | `measure_text` 委托给 `TextMeasurer`（或移除，改为直接用 TextMeasurer） |
| `panel/tree/mod.rs` | 移除 `mod desc; mod node;`，改为从 `crate::widget` 导入并 re-export |
| `panel/tree/diff.rs` | 移除 `into_parts()` 依赖，改为直接 match Desc 变体；style/decoration 写到 node 级别；增加 Widget 处理分支 |
| `panel/tree/paint.rs` | 改为读 `node.decoration`（不再 match kind 取 decoration）；Leaf 分支 match `node.kind` 画内容 |
| `panel/tree/hit.rs` | 改为读 `node.decoration.is_some()`（不再 match kind） |
| `panel/tree/layout.rs` | LayoutTree impl 的 `style()` 改为返回 `&node.style`（不再 match kind）；layout 流程前增加 resolve 调用 |
| `demo.rs` | 用 `Desc::Widget` + `ButtonProps` 替换现有 `button()` 函数；移除 renderer 依赖 |

## 不做的事

- **不实现 InteractionState** — 那是"状态"（§3），不是"配置"（§2）
- **不实现 Action 系统** — 那是"事件"（§6），不是"配置"
- **不实现控件 build() 的具体展开逻辑** — Props 结构和 trait impl 都创建，但 build() 方法体用 `todo!()` 占位。实际的图元组合（Slider 的轨道+滑块+文字等）在后续实现各控件时完成
- **不重构 panel/tree/ 的算法为泛型** — 画布做的时候再看
- **不加宏消除样板代码** — 先手写，确定模式稳定后再加
