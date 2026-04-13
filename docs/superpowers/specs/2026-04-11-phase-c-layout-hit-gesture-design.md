# 阶段 C.1 + C.2 + C.3 设计:布局/命中/手势基础设施

## 背景

阶段 B(commit `fbd4d4e`)完成后,`gui/src/tree/` 已作为顶层模块就位。但是 `BoxStyle` 在阶段 A 加入的四个关键字段(`position` / `transform` / `hittable` / `gestures`)都**还没有被消费**:

- `tree/layout/arrange.rs` 忽略 `Position::Absolute`,所有节点按 Flow 排列
- `tree/hit.rs` 返回 `Option<&str>`(单个 widget id),忽略 `hittable` 和 `gestures`,判据仅靠 `decoration.is_some()`,且正向遍历子节点(不是反向 z-order)
- `gesture/` 下只有 Tap / Drag / LongPress 三个识别器,没有 Resize

阶段 C 大纲把这些缺失拆成 C.1 (Absolute 布局) / C.2 (hit chain) / C.3 (ResizeRecognizer) 三个子阶段。本 spec 把三个子阶段合并成一次性交付,因为它们构成"声明式命中 + 手势基础设施"的一个连贯单元,合并后依赖链清晰、测试互通。

阶段 C 的后续子阶段(C.4 Panel widget / C.5 Canvas 分支 / C.6 Context 重构 / C.7 demo 重写 / C.8 删除 PanelLayer)**不在本 spec 范围**。它们依赖本 spec 提供的基础设施,但设计和实施独立。

## 目标与非目标

### 目标

1. **C.1 布局**:`tree/layout/arrange.rs` 支持 `Position::Absolute { x, y }`。Absolute 子节点不参与 Flexbox 空间分配,位置相对父节点 content(padding 内)。同时配套调整 Transform 节点的 content 起点(供 C.2 Transform 逆变换使用)。
2. **C.2 命中**:`tree/hit.rs` 改写为返回 `HitChain`(从叶子到根的 NodeId 序列)。实现反向子节点遍历(后渲染的先命中)、`hittable` 三态强制覆盖、混合判据("gestures 非空 OR decoration 非空")、Transform 节点的坐标逆变换。
3. **C.3 手势**:新建 `gesture/resize.rs` 实现 `ResizeRecognizer`,边缘检测 6px 阈值,8 个方向,符合现有 `GestureRecognizer` trait。`gesture/kind.rs` 加 `Gesture::Resize` 变体。`widget/action.rs` 加 `ResizeStart/ResizeMove/ResizeEnd` 变体。新建 `widget/resize_edge.rs` 定义 `ResizeEdge` 枚举。
4. **零行为变化(demo 层面)**:`app/src/demo.rs` 不需要改动,视觉和交互与阶段 B 结束时完全一致。`panel/renderer.rs::hit_test` 保留 `Option<&str>` 签名(内部适配取 HitChain.leaf())。

### 非目标

- **不改 demo.rs** — demo 继续走旧 panel 系统(`panel/frame.rs` / `panel/layer.rs` / `panel/drag.rs` / `panel/resize.rs` 全部保留不动)
- **不改 widget/atoms/*.rs** — 按钮/滑块/开关/下拉/文本输入 5 个控件的 `build()` 不动
- **不实现 Panel widget 框架** — 那是 C.4
- **不实现 canvas 分支 / NodeCard widget / Grid / Connection paint** — 那是 C.5
- **不重构 Context** — 那是 C.6
- **不删 PanelLayer / PanelFrame / PanelRenderer** — 那是 C.8
- **不实现 Transform 的 paint 正向变换** — C.2 只做 hit test 的逆变换和 arrange 的 content 重置,实际画布绘制留给 C.5
- **不写声明式 arena 创建逻辑** — C.3 只提供 ResizeRecognizer,arena 按 hit chain 的 `gestures` 字段自动构造识别器的逻辑属于 C.6 / C.7
- **不删除 panel/resize.rs 里的 ResizeEdge** — C.3 新的 `widget/resize_edge.rs::ResizeEdge` 和 `panel/resize.rs::ResizeEdge` 暂时共存,后者留给 C.8 清理
- **不修复已知的反向依赖**(tree → widget::props)

## 指导原则

1. **基础设施优先,贯通再谈** — C.1+C.2+C.3 提供"一棵树上的声明式命中 + 手势"能力,但本阶段不让 demo 实际用起来。真正的贯通(declarative arena creation、panel-as-widget)在 C.4~C.7。
2. **零 demo 破坏** — 任何会让 demo 表现变化的改动都不在 scope。API 兼容通过 `panel/renderer.rs::hit_test` 的 `Option<&str>` 兼容层维持。
3. **文件结构 = 软件结构** — 独立职责独立成文件,即使很小。体现在 `widget/resize_edge.rs` 独立出来(不塞进 action.rs)。
4. **混合判据不激进** — C.2 的 hit 判据保留 "decoration 非空" 作为兼容通路,让 widget/atoms 不需要同步补 gestures 声明。纯粹的 "仅 gestures 非空" 留给 C.4+ 按需切换。
5. **测试优先 vs 手测** — 纯算法层(Absolute arrange / HitChain / ResizeRecognizer)全部用单元测试覆盖。demo 手测只是行为回归检查。

## 决策记录

### 决策 1 · C.2 API 直接破坏,内部适配

**决定**:`tree/hit.rs::hit_test` 直接改签名为 `fn hit_test(tree, root, x, y) -> HitChain`。`panel/renderer.rs::hit_test` 改为内部调新 API 取 `HitChain.leaf()` 对应节点的 `id`,外部签名仍返回 `Option<&str>`。

**替代方案**:
- 保留旧 `hit_test`(`Option<&str>`) + 新增 `hit_chain()`(`HitChain`)并存。避免破坏内部模块 API。
- 立即改所有调用点(包括 demo.rs),彻底断绝兼容。

**理由**:
- `tree::hit::hit_test` 在 gui crate 内的唯一 call site 是 `panel/renderer.rs::hit_test`。改一处内部兼容层即可消化。
- 保留两个 API 是长期的重复,违反"一个文件一个职责"的精神。
- 立即破坏 demo.rs 属于 C.7 scope,不该在基础设施阶段做。
- 选择中间方案:破坏 `tree::hit::hit_test`,但用 `panel/renderer.rs` 做一次性转换层,对外 API 等价。这让 C.7 真正需要 HitChain 时可以直接消费,不需要拆兼容层。

### 决策 2 · C.2 混合命中判据

**决定**:默认的可命中判据是 `style.gestures 非空 OR decoration 非空`。`hittable: Some(true)` 强制可命中,`hittable: Some(false)` 强制不可命中,`hittable: None` 走默认判据。

**替代方案**:
- 纯粹的 "仅 gestures 非空"。要求同步修改 `widget/atoms/*.rs` 5 个文件给按钮/滑块等加 gestures 声明,并修改 demo.rs 的 arena 创建逻辑变成声明式。scope 膨胀 ~150 行。
- 保留旧判据 "仅 decoration 非空",完全忽略 `gestures` 字段。违反阶段 A 已引入的语义,没实用价值。

**理由**:
- 混合判据是纯扩展(老行为 + 新能力),demo 里所有 widget 都不声明 gestures 的状态下,行为与阶段 B 完全等价。
- C.4 新增 Panel widget 时可以直接在 `build()` 里声明 `gestures: vec![Drag, Resize]`,立即参与命中。
- 同步修改 atoms 和 demo 会把 C.3/C.6 的 scope 混进来,不利于分阶段验证。

### 决策 3 · C.3 Resize 手势声明式 + arena 竞争

**决定**:`Gesture::Resize` 作为 `BoxStyle.gestures` 的新变体。arena 创建者按节点的 `gestures` 列表逐个创建 recognizer(C.4+ 的事)。`ResizeRecognizer::on_pointer_down` 做边缘检测:不在边缘(6px 阈值)时返回 `false`,后续 `on_pointer_move` 返回 `Rejected`。让 `DragRecognizer` 自然竞争胜出。

**替代方案**:
- hit 阶段直接判断在边缘就只注册 Resize,否则只注册 Drag。逻辑在 arena 外部,不符合"一个节点声明所有可用手势"的语义。
- `BoxStyle` 单独加一个 `resizable: Option<ResizeSpec>` 字段。另外开一个维度,破坏"所有手势统一走 gestures 字段"的设计。

**理由**:
- 声明式 + arena 竞争符合现有 Tap/Drag/LongPress 的模式,心智负担最小。
- `ResizeRecognizer` 的"在边缘就胜出,否则就退出"策略让它可以和 Drag 共存而不冲突,竞争逻辑由 arena 的现有机制(见 `gesture/arena.rs:38-55` 的 `pointer_move` 反向遍历)保证。

### 决策 4 · C.1 Absolute 相对 parent content

**决定**:`Position::Absolute { x, y }` 表示相对于父节点 content(扣除 padding 后的内容区域)的左上角偏移。与 CSS 的 `position: absolute` 语义一致。

**替代方案**:
- 相对 parent rect(不扣 padding)。更"直接"。
- 相对 root(全局坐标)。无视嵌套。

**理由**:
- CSS 约定是行业通识,降低设计者的心智负担。
- 让父节点的 padding 对 Absolute 子节点自然生效(想让面板离窗口边缘有间距就给 PanelRoot 加 padding)。
- 相对 parent rect 会让 padding 对 Absolute 子节点无效,破坏 padding 的语义一致性。

### 决策 5 · C.2 Transform 逆变换 + C.1 arrange content 重置

**决定**:`tree/hit.rs` 在遇到带 `Transform` 的节点时,把当前坐标 `(x - node.rect.x, y - node.rect.y)` 通过逆变换映射到子节点的 local 空间。配套地,`tree/layout/arrange.rs` 在计算 Transform 节点的 content 区域时**把 origin 重置为 `(padding.left, padding.top)`**(不是 `(node_rect.x + padding.left, node_rect.y + padding.top)`),让 Transform 节点的子节点 rect 存储在 local 空间。

**替代方案**:
- 完全推迟到 C.5 — 本 spec 只实现 Absolute 和反向命中,不碰 Transform。
- 只改 hit.rs,不改 arrange.rs — 结果不自洽,arrange 存绝对坐标,hit 却按 local 查。

**理由**:
- 基础设施完整性:同时改 arrange 和 hit 让 Transform 在 C.2 完成后就是"语义正确"的(虽然 paint 还没实现正向变换,但命中链能正确计算,数据结构能正确存储)。
- C.5 canvas 分支实现时只需要补 paint 正向变换,hit/arrange 已就位。
- 风险:没有 CanvasRoot 实例运行时无法手测这条代码路径。只能用单元测试(构造带 Transform 的合成树)验证,本 spec 的测试计划包含这点。

### 决策 6 · 三个 commit 而非一个原子 commit

**决定**:C.1 / C.2 / C.3 各自独立 commit,合并成一个 PR。

**替代方案**:像 Phase B 那样整体一次 commit。

**理由**:
- 每个子阶段独立可编译 + 可测试(不像 Phase B 的 `panel/tree/` 搬迁必须原子完成)。
- 独立 commit 提供清晰的 git 历史,review 和 revert 单位更细。
- PR 层面仍然是一个打包的交付单元,不影响合并决策。

### 决策 7 · `ResizeEdge` 独立文件

**决定**:`ResizeEdge` 枚举放 `gui/src/widget/resize_edge.rs`(新文件),`widget/action.rs` 和 `gesture/resize.rs` 都从这里引用。

**替代方案**:
- 塞进 `widget/action.rs` 里 — 和 Action 定义混在一起。
- 塞进 `gesture/resize.rs` 里 — 会导致 `widget/action.rs` 反向依赖 `gesture/`,形成循环。

**理由**:
- 遵循项目的"一个文件一个职责"原则:`ResizeEdge` 是"resize 的几何方向"这个独立概念,值得独立文件。
- 依赖方向:`widget/` 不依赖 `gesture/`;`gesture/` 依赖 `widget/`(已经通过 `Action` 建立了依赖)。把 `ResizeEdge` 放 `widget/` 下不破坏这个方向。

## C.1 设计 · 布局引擎 Absolute 定位

### 核心算法

`tree/layout/arrange.rs` 的 `arrange()` 函数在子节点循环前**按 position 分组**,Flex 阶段只处理 Flow 子节点,然后单独一趟处理 Absolute 子节点:

```rust
pub(crate) fn arrange<T: LayoutTree>(
    tree: &mut T,
    node: T::NodeId,
    available: Rect,
    measure_text: &mut dyn FnMut(&str, f32) -> (f32, f32),
) {
    // ── 现有代码:算 node_rect、padding ──
    // (基本不变,但 content 的起点对 Transform 节点做调整,见 C.2 决策 5)

    let children = tree.children(node);
    if children.is_empty() { return; }

    // 【新】按 position 分组
    let (flow_children, abs_children): (Vec<_>, Vec<_>) = children
        .iter()
        .copied()
        .partition(|&c| matches!(tree.style(c).position, Position::Flow));

    // ── Flex 阶段(现有代码,但循环变量从 children 改为 flow_children)──
    // measure → 算 fixed_main 和 total_grow → justify → 循环 arrange
    // 滚动 content_height 也只算 flow_children(避免 Absolute 污染)

    // ── 【新】Absolute 阶段 ──
    for child in abs_children {
        let style = tree.style(child).clone();
        let (abs_x, abs_y) = match style.position {
            Position::Absolute { x, y } => (x, y),
            _ => unreachable!(),
        };
        
        let available = Rect {
            x: content.x + abs_x,
            y: content.y + abs_y,
            w: (content.w - abs_x).max(0.0),
            h: (content.h - abs_y).max(0.0),
        };
        
        arrange(tree, child, available, measure_text);
    }
}
```

### Transform 节点的 content 起点调整(配合 C.2)

```rust
// 计算 content 时:
let content = if node.style.transform.is_some() {
    // Transform 节点:子节点 rect 在 local 空间,起点相对节点本身 (0,0)
    Rect {
        x: node.style.padding.left,
        y: node.style.padding.top,
        w: (node_rect.w - node.style.padding.horizontal()).max(0.0),
        h: (node_rect.h - node.style.padding.vertical()).max(0.0),
    }
} else {
    // 普通节点:content 在绝对坐标(和之前一样)
    Rect {
        x: node_rect.x + node.style.padding.left,
        y: node_rect.y + node.style.padding.top,
        w: (node_rect.w - node.style.padding.horizontal()).max(0.0),
        h: (node_rect.h - node.style.padding.vertical()).max(0.0),
    }
};
```

这是 C.1 和 C.2 之间的约定:Transform 节点的子节点在 local 空间,hit.rs 的逆变换负责把坐标从外部空间映射进来。

### 语义细节

| 字段 | Absolute 子节点的行为 |
|------|----------------------|
| `position: Absolute { x, y }` | x/y 相对父 content 左上角(padding 内,CSS 标准) |
| `width: Fixed(w)` | 节点宽度 = w |
| `width: Auto` / `Fill` | 节点宽度 = `parent_content.w - abs.x`(剩余空间) |
| `height` | 同上 |
| `margin` | 会被扣除(走 child 的 arrange top 逻辑) |
| `min/max_width/height` | 正常 clamp |
| `flex_grow / align_items / justify_content` | **被父 Flex 忽略**(Absolute 不参与 flex) |
| `content_height`(滚动容器) | **不计入** Absolute 子节点 |

### 单元测试(5 个)

- `absolute_simple`: 父 padding=0,Absolute 子节点 `{x:100, y:50}`,验证 child.rect 起点 `(100, 50)`
- `absolute_with_padding`: 父 padding=20,Absolute 子节点 `{x:100, y:50}`,验证 child.rect 起点 `(120, 70)`
- `absolute_does_not_take_flex_space`: 父 Row + 两个 Flex 子节点 + 一个 Absolute,验证 Flex 子节点占用空间与没有 Absolute 时相同
- `absolute_fixed_size`: Absolute 子节点 `{width: Fixed(100), height: Fixed(50)}`,验证 rect 尺寸与 available 无关
- `absolute_in_scroll_container`: 父 overflow=Scroll,有 Flow 和 Absolute 两类子节点,验证 `content_height` 只累计 Flow 子节点

## C.2 设计 · HitChain

### HitChain 数据结构

`gui/src/tree/hit.rs` 新增:

```rust
use super::node::NodeId;
use super::tree::Tree;
use super::layout::{BoxStyle, Decoration, Transform};

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
```

### 新的 `hit_test()` 算法

```rust
pub fn hit_test(tree: &Tree, root: NodeId, x: f32, y: f32) -> HitChain {
    let mut nodes = Vec::new();
    hit_recursive(tree, root, x, y, &mut nodes);
    HitChain::new(nodes)
}

fn hit_recursive(
    tree: &Tree,
    node_id: NodeId,
    x: f32, y: f32,
    chain: &mut Vec<NodeId>,
) -> bool {
    let Some(node) = tree.get(node_id) else { return false };
    
    // 1. 自身 bounds 检查(在当前坐标空间)
    let r = &node.rect;
    if x < r.x || x > r.x + r.w || y < r.y || y > r.y + r.h {
        return false;
    }
    
    // 2. Transform 节点:逆变换后进入子节点的 local 空间
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
fn is_hittable(style: &BoxStyle, decoration: &Option<Decoration>) -> bool {
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

### panel/renderer.rs 兼容适配

```rust
// 唯一的适配,外部 API 签名保持 Option<&str>
pub fn hit_test(&self, x: f32, y: f32) -> Option<&str> {
    let root = self.tree.root()?;
    let chain = hit_test(&self.tree, root, x, y);
    chain.leaf()
        .and_then(|id| self.tree.get(id))
        .map(|n| n.id.as_ref())
}
```

### tree/mod.rs 更新

```rust
// 原来是 pub use hit::hit_test;
pub use hit::{hit_test, HitChain};
```

### 单元测试(12 个)

- `hit_empty_tree`: 空树返回 empty HitChain
- `hit_root_only`: 只有 root 的树,命中 root
- `hit_leaf_nested`: 三层嵌套,leaf 有 decoration,chain 返回三个 NodeId(顺序 leaf → middle → root)
- `hit_reverse_z_order`: 两个重叠的兄弟节点,第二个(后添加的)先命中
- `hit_hittable_force_false`: `hittable: Some(false)` 强制不命中,即使有 decoration
- `hit_hittable_force_true`: `hittable: Some(true)` 强制命中,即使啥都没有
- `hit_gestures_makes_hittable`: 有 gestures 无 decoration 的节点可命中
- `hit_neither_gestures_nor_decoration`: 两者都没的节点不可命中
- `hit_outside_bounds`: 点击在 root 外返回 empty
- `hit_transform_identity`: Transform `{translate: [0,0], scale: 1, rotate: 0}` 时行为与无 transform 相同
- `hit_transform_translate`: Transform `{translate: [50, 50]}` 后,screen (100, 100) 对应子节点 local (50, 50)
- `hit_transform_scale`: Transform `{scale: 2.0}` 后,screen (200, 200) 对应子节点 local (100, 100)

## C.3 设计 · ResizeRecognizer

### 新建 `widget/resize_edge.rs`

```rust
/// Resize 手势命中的边/角。
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

### `widget/action.rs` 扩展

```rust
use super::resize_edge::ResizeEdge;

#[derive(Debug, Clone)]
pub enum Action {
    Click(String),
    DoubleClick(String),
    DragStart { id: String, x: f32, y: f32 },
    DragMove { id: String, x: f32, y: f32 },
    DragEnd { id: String, x: f32, y: f32 },
    LongPress(String),
    // 【新】
    ResizeStart { id: String, edge: ResizeEdge, x: f32, y: f32 },
    ResizeMove { id: String, edge: ResizeEdge, x: f32, y: f32 },
    ResizeEnd { id: String, edge: ResizeEdge, x: f32, y: f32 },
}
```

### `widget/mod.rs` 更新

```rust
// 原有
pub mod action;
// ...

// 【新】
pub mod resize_edge;
```

### `gesture/kind.rs` 扩展

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gesture {
    Tap,
    DoubleTap,
    Drag,
    LongPress,
    Resize,  // 【新】
}
```

### `gesture/resize.rs` 新建

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
    pub fn new(target_id: String, node_rect: Rect) -> Self {
        Self {
            target_id,
            node_rect,
            edge: None,
            down_x: 0.0, down_y: 0.0,
            current_x: 0.0, current_y: 0.0,
            resizing: false,
            done: false,
        }
    }
}

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

impl GestureRecognizer for ResizeRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x;
        self.down_y = y;
        self.current_x = x;
        self.current_y = y;
        self.edge = detect_edge(&self.node_rect, x, y);
        self.edge.is_some()
    }

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

    fn reject(&mut self) {}
}
```

### `gesture/mod.rs` 更新

```rust
pub mod arena;
pub mod drag;
pub mod kind;
pub mod long_press;
pub mod recognizer;
pub mod resize;  // 【新】
pub mod tap;

pub use kind::Gesture;
```

### 单元测试(12 个)

- `detect_edge_corners`: 四角命中 → 返回 TopLeft/TopRight/BottomLeft/BottomRight
- `detect_edge_sides`: 四边命中 → 返回 Top/Bottom/Left/Right
- `detect_edge_inside`: 节点内部(距边 > 6px)返回 None
- `recognizer_rejects_interior_down`: `on_pointer_down` 在内部返回 false
- `recognizer_accepts_edge_down`: `on_pointer_down` 在边缘返回 true
- `recognizer_pending_until_move_threshold`: down 在边缘后,不动 → `on_pointer_move` 返回 Pending
- `recognizer_accept_on_sufficient_move`: down 在边缘 + move > 3px → `on_pointer_move` 返回 Accepted
- `recognizer_reject_on_up_without_move`: down 在边缘 + 未动 + up → `on_pointer_up` 返回 Rejected
- `resize_start_action`: 第一次 `accept()` 产出 `Action::ResizeStart`
- `resize_move_action`: 后续 `accept()` 产出 `Action::ResizeMove`
- `resize_end_action`: 经过 `on_pointer_up` 后的 `accept()` 产出 `Action::ResizeEnd`
- `all_eight_edges_action_variants`: 各方向产出的 Action 带有正确的 edge 字段

## 完整文件清单

| 文件 | 改动性质 | 所属子阶段 |
|------|---------|-----------|
| `gui/src/tree/layout/arrange.rs` | 改(加 Absolute 分支 + Transform content 起点) | C.1 |
| `gui/src/tree/hit.rs` | 完全重写 | C.2 |
| `gui/src/tree/mod.rs` | 加 `pub use hit::HitChain` | C.2 |
| `gui/src/panel/renderer.rs` | 小改(hit_test 内部适配) | C.2 |
| `gui/src/widget/resize_edge.rs` | **新建** | C.3 |
| `gui/src/widget/action.rs` | 加 3 个 ResizeAction 变体 + import | C.3 |
| `gui/src/widget/mod.rs` | 加 `pub mod resize_edge;` | C.3 |
| `gui/src/gesture/kind.rs` | 加 `Resize` 变体 | C.3 |
| `gui/src/gesture/resize.rs` | **新建** | C.3 |
| `gui/src/gesture/mod.rs` | 加 `pub mod resize;` | C.3 |

合计 **10 个文件**,2 个新建,8 个修改。

### 非改动保证

- `app/src/demo.rs` — 零改动
- `gui/src/widget/atoms/*.rs`(5 个控件)— 零改动
- `gui/src/panel/` 下 `drag.rs` / `resize.rs` / `frame.rs` / `layer.rs` / `hit.rs` — 零改动
- `gui/src/canvas/*` — 零改动

## 执行顺序

```
C.1 布局引擎
    ├─ 改 arrange.rs(Absolute 分支 + Transform content 调整)
    ├─ cargo check + 5 个 arrange 单元测试
    └─ commit: feat(gui): C.1 布局引擎支持 Position::Absolute
        │
        ▼
C.2 HitChain
    ├─ 重写 hit.rs(HitChain + 反向 + 混合判据 + Transform 逆变换)
    ├─ 更新 tree/mod.rs re-export
    ├─ panel/renderer.rs 内部适配
    ├─ cargo check + 12 个 hit 单元测试
    └─ commit: feat(gui): C.2 hit test 返回 HitChain 支持 z-order / hittable / Transform
        │
        ▼
C.3 ResizeRecognizer
    ├─ 新建 widget/resize_edge.rs
    ├─ 扩展 widget/action.rs
    ├─ 新建 gesture/resize.rs
    ├─ 扩展 gesture/kind.rs
    ├─ 更新 mod.rs 导出
    ├─ cargo check + 12 个 resize 单元测试
    └─ commit: feat(gui): C.3 ResizeRecognizer 和 Gesture::Resize
```

三个 commit 合并成一个 PR。每个 commit 都独立可编译 + 测试通过 + demo 手测无回归。

## 测试策略

### 单元测试

三个子阶段共增加 **29 个单元测试**,覆盖所有新增算法和边界情况:

- C.1:5 个(arrange.rs)
- C.2:12 个(hit.rs)
- C.3:12 个(resize.rs)

当前 `gui` crate 没有现存的单元测试(`cargo test --workspace` 显示 0 tests),本 spec 一次性补齐三个关键模块的覆盖。

### 集成验证

无集成测试。验证手段:

1. `cargo check -p gui` 编译通过,零新 warning
2. `cargo clippy -p gui` 零新 warning
3. `cargo build -p app --release` 编译通过
4. `cargo test --workspace` 29 个新测试全过
5. `cargo run -p app --release` 启动 demo:
   - 面板(toolbar)可见
   - 按钮点击、滑块拖拽正常
   - 面板拖拽移动、面板边缘 resize 正常
   - 行为与 Phase B 结束时(commit `fbd4d4e`)完全一致

### 验收标准

- [ ] 10 个文件的改动/新建完成,git diff 清晰
- [ ] `cargo check` / `cargo clippy` / `cargo build --release` / `cargo test --workspace` 全过
- [ ] 29 个新单元测试全部通过
- [ ] demo 手测行为无回归
- [ ] 三个 commit 按顺序提交,每个 commit 独立可编译 + 可测试
- [ ] PR 标题:`feat(gui): 阶段 C.1+C.2+C.3 —— 布局/命中/手势基础设施`

## 已知 TODO(不在本 spec scope)

- **Transform 正向变换应用到 paint**:C.2 实现了逆变换 + content 重置约定,但 `tree/paint.rs` 没处理 transform 矩阵。C.5 canvas 分支实现时补齐。
- **声明式 arena 创建**:demo.rs 目前手动创建 recognizer 的逻辑(第 105-136 行)保持不变,C.6/C.7 改为按 hit chain 上每个节点的 gestures 字段自动构造。
- **按钮/滑块的 gestures 声明**:所有 widget atoms 目前的 `build()` 没声明 gestures,混合判据下仍可命中。C.4+ 按需补充。
- **ResizeEdge 双枚举并存**:`widget/resize_edge.rs::ResizeEdge`(新,C.3)和 `panel/resize.rs::ResizeEdge`(旧)在 C.8 删除 `panel/resize.rs` 前共存。
- **tree → widget::props 反向依赖**:Phase B 已知 TODO,Phase C.4+ 解决。

## 预估工作量

| 子阶段 | 代码量 | 测试数 | 预估 |
|-------|-------|-------|------|
| C.1 | ~70 行 arrange.rs 增补 | 5 | 1-2 天 |
| C.2 | ~120 行(完全重写 hit.rs + mod.rs 小改 + panel/renderer.rs 适配) | 12 | 2-3 天 |
| C.3 | ~140 行(新建 resize.rs + action.rs 扩展 + resize_edge.rs + mod.rs 更新) | 12 | 2-3 天 |
| **合计** | **~330 行** | **29** | **5-8 天** |
