# Affine Display List Rendering Phase 2 设计和执行计划

依据 `docs/todo/2026-04-23-affine-display-list-rendering-plan.md` 和已落地的 Phase 1 contract，Phase 2 的目标是把 tree paint / hit 从旧的 screen-space、scale-only 语义迁到统一的 affine DisplayList 语义。这个阶段仍不改 renderer pipeline，不要求 GPU backend 真的画旋转；renderer 主路径消费 DisplayList 是 Phase 3。

Phase 2 的核心验收不是“旋转视觉正确”，而是：

```text
tree paint recording = local primitive + resolved affine + clip state
tree hit = 使用同一 tree transform / paint_space adapter 的 inverse
旧 PaintTransform / inverse_supported_transform 不再作为事实来源
translate / scale 的现有视觉行为不回归
```

## 1. 阶段目标

Phase 2 交付以下能力：

1. 新增 `tree::transform`，把旧 `tree::layout::Transform` 适配成 `geometry::TransformSpec` / `Affine2D`。
2. 新增 `tree::paint_space`，集中处理当前 layout 坐标习惯，尤其 transform 节点 children local-space 特例。
3. `tree::paint` 改为生产 `paint::DisplayList`，内部使用 `paint::PaintTarget` / `DisplayListBuilder`。
4. `tree::hit` 使用同一 `tree::paint_space` adapter 做 affine inverse，不再维护 `inverse_supported_transform`。
5. `tree::paint_ops` 不再保存 screen-space `PaintOp`；recording 测试改为断言 `paint::ResolvedPaintCommand`。
6. 保留一个明确临时的 legacy replay adapter，把 DisplayList 转回旧 `Renderer::draw_*` 调用，保证 Phase 2 期间视觉不变。该 adapter 必须只放在 tree/renderer 边界，Phase 3 删除。
7. 补齐 tree paint 需要但 Phase 1 尚不足的 SVG/Icon paint contract，避免 tree 为 icon 直接依赖 renderer SVG backend。

## 2. 严格边界

必须做：

- tree paint/hit 共用同一 transform adapter。
- tree paint 记录 local-space primitive，不能提前写入 screen-space 点位。
- clip 使用 `paint::ClipShape`，不再以 `PushClip { rect, radius }` 作为跨层 API。
- `CustomPaint` 只依赖 `paint::PaintTarget`，上下文提供 local rect、current transform、screen bounds。
- legacy replay 对不支持的非轴对齐 affine 必须显式返回/记录 unsupported，不能 silent fallback 到错误 screen-space。

不能做：

- 不改 renderer pipeline / shader。
- 不把 `renderer::DrawCommand` 暴露给 tree。
- 不在 tree 里调用 `renderer::svg::resolve_svg_icon_paths` 或任何 renderer SVG internals。
- 不新增第二套 transform math。
- 不把旧 `PaintTransform` 留作兼容事实来源。
- Phase 2 当时未包含 shape-aware hit testing。后续已由 `tree::hit_shape` 完成 leaf geometry hit，`hit.rs` 仍保留 traversal / overflow / z-order / hittable 的职责边界。
- 不做 unrelated layout refactor。

## 3. 当前坐标语义和 Phase 2 定义

当前 layout 有两种 child 坐标习惯：

```text
普通节点:
  children rect 已经在当前 layout coordinate space 中
  递归 child 时不继承 parent node-local translation

transform 节点:
  children rect 在该 transform node 的 local coordinate space 中
  递归 child 时必须继承 node local-to-screen 和 node transform
```

这就是 Phase 2 必须新增 `tree::paint_space` 的原因。不要把这个历史行为写进 `paint` 或 `geometry`。

Phase 2 固定三种 transform：

```text
current_space_to_screen:
  当前 layout coordinate space -> screen/logical pixels

node_local_to_screen:
  current_space_to_screen * T(node.rect.x, node.rect.y)

transform_child_space_to_screen:
  node_local_to_screen * transform_spec.to_affine(local_node_rect)
```

`local_node_rect` 固定为：

```rust
Rect { x: 0.0, y: 0.0, w: node.rect.w, h: node.rect.h }
```

普通节点的 children 使用原 `current_space_to_screen`；transform 节点的 children 使用 `transform_child_space_to_screen`。

## 4. 文件结构

新增：

```text
gui/src/tree/
  transform.rs
  paint_space.rs
```

调整：

```text
gui/src/tree/
  mod.rs
  paint.rs
  hit.rs
  paint_target.rs
  paint_ops.rs
  paint_helpers.rs
  layout/types.rs

gui/src/paint/
  command.rs      # 补齐 SVG/Icon resource paint contract
  target.rs       # 不新增 required method，只新增 default helper
  mod.rs
```

不调整：

```text
gui/src/renderer/pipeline/*
gui/src/renderer/shaders/*
```

临时 adapter：

```text
gui/src/tree/paint_target.rs
  LegacyDisplayListRenderer     # Phase 2 only, Phase 3 删除
```

或拆分为：

```text
gui/src/tree/legacy_paint_replay.rs
```

推荐拆分成 `legacy_paint_replay.rs`，避免 `paint_target.rs` 同时承担 trait、context、renderer replay 三种职责。

## 5. `tree::transform` 设计

`gui/src/tree/transform.rs`：

```rust
use crate::geometry::{Affine2D, Rect, TransformOrigin, TransformSpec};
use crate::tree::layout::Transform;

pub fn legacy_transform_spec(transform: Transform) -> TransformSpec;
pub fn legacy_transform_affine(transform: Transform, local_bounds: Rect) -> Affine2D;
```

旧 `Transform` 到新 `TransformSpec`：

```text
Transform {
  translate,
  scale,
  rotate,
}
->
TransformSpec {
  translate,
  scale: [scale, scale],
  rotate,
  skew: [0.0, 0.0],
  origin: TransformOrigin::top_left(),
}
```

要求：

- 这是唯一允许理解旧 `tree::layout::Transform` 的 adapter。
- `paint.rs` 和 `hit.rs` 不得直接读取 `Transform::translate/scale/rotate` 做数学。
- 零 scale 由 `Affine2D::inverse()` 返回 `None`，不手写特殊 inverse。

测试：

```text
legacy_transform_spec_preserves_translate_scale_rotate
legacy_transform_affine_matches_old_translate_scale_for_top_left_origin
legacy_transform_affine_includes_rotate_in_recording_semantics
zero_scale_inverse_returns_none_through_affine
```

## 6. `tree::paint_space` 设计

`gui/src/tree/paint_space.rs`：

```rust
use crate::geometry::{Affine2D, Point, Rect};
use crate::tree::layout::Transform;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaintSpace {
    pub to_screen: Affine2D,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodePaintSpace {
    pub local_rect: Rect,
    pub local_to_screen: Affine2D,
    pub child_to_screen: Affine2D,
    pub children_are_local: bool,
}
```

API：

```rust
impl PaintSpace {
    pub fn root() -> Self;
    pub fn node_space(self, node_rect: Rect, transform: Option<Transform>) -> NodePaintSpace;
    pub fn point_to_local(self, node_rect: Rect, screen: Point) -> Option<Point>;
}

impl NodePaintSpace {
    pub fn child_space(self) -> PaintSpace;
    pub fn screen_bounds(self) -> Rect;
    pub fn contains_local_bounds(self, local: Point) -> bool;
}
```

语义：

```text
PaintSpace::root().to_screen = Affine2D::IDENTITY

node_space.local_rect = Rect { x: 0, y: 0, w: node_rect.w, h: node_rect.h }
node_space.local_to_screen = self.to_screen * T(node_rect.x, node_rect.y)

if transform.is_some():
  child_to_screen = local_to_screen * legacy_transform_affine(transform, local_rect)
  children_are_local = true
else:
  child_to_screen = self.to_screen
  children_are_local = false
```

为什么 `children_are_local = false` 时 child space 回到 parent current space：

- 现有普通 children 的 rect 已在 current layout coordinate space。
- 如果继承 `node_local_to_screen`，会重复加 parent rect offset。

clip 例外：

- clip shape 使用 node local rect，在 `local_to_screen` 下 resolve。
- 即使普通 children 在 parent current space 下绘制，clip id stack 仍保持有效，因为 `ResolvedClip` 已记录自己的 transform。

测试：

```text
root_space_is_identity
node_local_to_screen_translates_by_node_rect
ordinary_child_space_keeps_parent_current_space
transform_child_space_uses_node_local_then_transform
screen_to_node_local_round_trips_for_translate_scale
screen_to_node_local_round_trips_for_rotate
non_invertible_transform_returns_none
```

## 7. SVG / Icon paint contract 补全

Phase 1 的 `PaintCommand` 有 `SvgRasterPaint`，但当前 tree `LeafKind::Icon` 使用 `IconSpec` 和 `IconRegistry`，而 renderer 的 vector/raster SVG 解析在 `renderer::svg` 内部。Phase 2 不能让 tree 直接调用 renderer SVG backend。

推荐在 Phase 2 做一个小型 paint contract 补全：

```text
gui/src/paint/svg.rs
```

实现时把 Phase 1 里暂放在 `paint::command` 的 `SvgSourceKey` 移到 `paint::svg`，再由 `paint::mod` re-export；`SvgRasterPaint` 继续复用同一个 `SvgSourceKey`。不要保留两个同名 source key 类型。

新增通用 SVG resource 语义：

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SvgSourceKey {
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SvgFit {
    Stretch,
    Contain,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SvgPaintOverride {
    Preserve,
    ReplaceCurrent(Color),
    Force(Color),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SvgStrokeWidth {
    Preserve,
    SvgUnits(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgStyle {
    pub color: Color,
    pub fill: SvgPaintOverride,
    pub stroke: SvgPaintOverride,
    pub stroke_width: SvgStrokeWidth,
    pub opacity: f32,
    pub fit: SvgFit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SvgPaint {
    pub rect: Rect,
    pub source: SvgSourceKey,
    pub style: SvgStyle,
}
```

`SvgStyle` 必须提供 constructor：

```rust
impl SvgStyle {
    pub fn new(color: Color) -> Self;
    pub fn with_opacity(self, opacity: f32) -> Self; // clamp [0,1], non-finite -> 1.0
}
```

`PaintCommand` 调整：

```rust
pub enum PaintCommand {
    ...
    Svg(SvgPaint),
    SvgRaster(SvgRasterPaint),
    ...
}
```

`SvgRasterPaint` 可以保留为 raster fallback 专用命令，但 tree `LeafKind::Icon` 应优先记录 `PaintCommand::Svg(SvgPaint)`，不再记录 tree-only `Icon` 命令。

转换规则：

```text
IconSpec.id -> SvgSourceKey { id: spec.id.as_str().to_string() }
IconStyle.fit -> SvgFit
IconStyle.fill/stroke -> SvgPaintOverride
IconStyle.stroke_width -> SvgStrokeWidth
IconStyle.opacity.get() -> SvgStyle.opacity
IconStyle.color -> SvgStyle.color
```

这样：

- paint 不依赖 `icon`。
- tree 只负责 IconSpec -> generic SvgPaint conversion。
- Phase 3/4 renderer backend 用 RenderResources/IconRegistry/SvgSource resolver 把 `SvgSourceKey` 映射成 bytes。
- 不留下 `draw_icon` 旁路。

如果选择不新增 `paint/svg.rs`，则必须在 Phase 2 文档和代码中明确 Icon leaf 暂不迁移，这会违反“当前 LeafKind 最终进入 DisplayList”的方向，不推荐。

## 8. `tree::paint` 迁移设计

### 8.1 入口

推荐入口结构：

```rust
pub(crate) fn paint(...) {
    let display_list = paint_to_display_list(...);
    LegacyDisplayListRenderer::new(renderer, textures, icons)
        .render(&display_list);
}

pub(crate) fn paint_to_display_list(...) -> Result<DisplayList, PaintBuildError>;

pub(crate) fn paint_to_target(
    tree: &Tree,
    root: NodeId,
    target: &mut dyn paint::PaintTarget,
    ...
);
```

说明：

- `paint_to_display_list` 是 Phase 2 新测试主入口。
- `paint_to_target` 保留为 test/custom paint 友好的接口，但 target 必须是 `crate::paint::PaintTarget`。它不返回 `PaintBuildError`；只有 `paint_to_display_list` 在 `DisplayListBuilder::finish()` 时返回 builder stack 错误。
- `paint()` 中的 legacy replay 是临时视觉桥，不是 tree paint 的事实来源。

### 8.1.1 Transform stack 规则

`paint::DisplayListBuilder::push_transform()` 接收 relative transform，并会和当前 transform 复合。`tree::paint` 禁止直接 push `node_space.local_to_screen` 这种 absolute transform。

允许的 push 顺序只有：

```text
普通节点:
  push T(node.rect.x, node.rect.y)
  draw node local commands / push local clip
  pop T(node.rect.x, node.rect.y)
  paint ordinary children in original current space
  pop local clip

transform 节点:
  push T(node.rect.x, node.rect.y)
  draw node local commands / push local clip
  push legacy_transform_affine(transform, local_rect)
  paint children in transform local child space
  pop legacy transform
  pop local clip
  pop T(node.rect.x, node.rect.y)
```

如果实现需要检查绝对矩阵，只能从 `tree::paint_space` 读取，用于 `CustomPaintCx`、connection endpoint 和测试断言；不能把它作为 relative transform push 进 target。

### 8.2 递归规则

目标递归结构：

```text
paint_node(tree, node_id, target, current_space, ...)
  node_space = current_space.node_space(node.rect, node.style.transform)

  push T(node.rect.x, node.rect.y)
    draw decoration in local_rect
    draw leaf in local_rect
    if overflow hidden/scroll:
      push ClipShape::RoundedRect { rect: local_rect, radius }

    if transform exists:
      push legacy_transform_affine(transform, local_rect)
        paint children with node_space.child_space()
      pop transform
      pop clip if pushed
      pop T(node.rect)

    else:
      pop T(node.rect)
      paint children with current_space
      pop clip if pushed
```

注意 clip 的顺序：

- clip 在 node local transform 下 push。
- 普通 children 绘制前可以 pop node translation，但 clip id stack 仍保留。
- `DisplayListBuilder` 的 clip transform 已在 push_clip 时 resolve，不会受后续 transform pop 影响。

### 8.3 各 LeafKind 的 local command

| LeafKind | Phase 2 记录方式 |
|---|---|
| Text | local rect 下 resolve text；记录 `PaintCommand::Text { pos, bounds }` local values；`TextStyle.size` 不再预乘 transform scale |
| Grid | 遍历 `Rect {0,0,w,h}` 的点；记录 local `CirclePaint` |
| Image | `ImagePaint { rect: local_rect, texture, style }` |
| Icon | `SvgPaint { rect: local_rect, source: spec.id, style: spec.style.into() }` |
| Circle | local center = `(w/2,h/2)`；记录 fill/stroke，stroke width 不预乘 scale |
| Connection | endpoint 通过 paint-space index 转成当前 connection local path；记录 local `PathPaint` |
| PendingConnection | from port 和 cursor 统一转换到 connection local path；记录 local `PathPaint` |
| Line / Curve / Path | 使用 leaf-local `PathData` 原值；不再调用 `leaf_path_to_screen` |
| CustomPaint | 调用 custom painter 时 target 当前 transform 已在 node local；`CustomPaintCx.local_rect = local_rect` |

### 8.4 样式缩放规则

Phase 2 起，tree recording 不再预乘 transform scale：

```text
RectStyle.radius 保持 local value
Border.width 保持 local value
Shadow.offset / blur / spread 保持 local value
Stroke.width 保持 local value
TextStyle.size 保持 local value
Circle.radius 保持 local value
```

legacy replay 为了保持现有视觉，会用 `Affine2D::approx_uniform_scale()` 临时把这些 style metrics 转成旧 renderer 需要的 screen-space metrics。

这个缩放只允许存在于 legacy replay，不允许回流到 tree paint 或 paint recording。

### 8.5 Connection endpoint

当前 `find_node_by_str_id(tree, id)` 只返回 rect，Phase 2 需要能把端口中心转换到 connection local space。推荐新增：

```rust
pub struct PaintSpaceIndex {
    by_stable_id: HashMap<String, NodePaintSpace>,
}
```

或者先用无分配递归查询：

```rust
fn find_node_paint_anchor(
    tree: &Tree,
    root: NodeId,
    target_id: &str,
    current_space: PaintSpace,
) -> Option<Point>;
```

要求：

- 返回值是 screen point 或 caller local point，必须命名清楚。
- connection path 最终记录在 connection leaf local space。
- 不允许继续假设所有端口 rect 在同一 screen-space / scale-only transform 下。

推荐接口：

```rust
pub fn find_node_screen_center(tree, root, id, PaintSpace::root()) -> Option<Point>;
pub fn screen_point_to_node_local(node_space, point) -> Option<Point>;
```

Then:

```text
from_screen = find_node_screen_center(...)
to_screen = find_node_screen_center(...)
connection_local = connection_node_space.local_to_screen.inverse()
path = connection_path(connection_local(from_screen), connection_local(to_screen))
```

## 9. `CustomPaintCx` 迁移

当前：

```rust
pub struct CustomPaintCx {
    pub rect: Rect,
}
```

Phase 2 改为：

```rust
pub struct CustomPaintCx {
    pub local_rect: Rect,
    pub transform: Affine2D,
    pub screen_bounds: Rect,
}
```

位置：

- 可以继续放在 `tree::paint_target`，因为它是 tree custom paint 上下文。
- `CustomPainter` trait 改为使用 `crate::paint::PaintTarget`。

调用时：

```text
local_rect = Rect {0,0,w,h}
transform = node_space.local_to_screen
screen_bounds = transform.transformed_bounds(local_rect)
```

Custom painter draw 的命令仍然进入当前 target transform stack，所以 painter 应优先使用 `local_rect`。`screen_bounds` 只用于需要屏幕估算的特殊 painter。

兼容策略：

- 不保留旧 `cx.rect` 字段。
- 更新现有测试和内置 custom painter。
- 这是有意破坏旧 API，避免继续传 screen rect。

## 10. `tree::hit` 迁移设计

入口保持：

```rust
pub fn hit_test(tree: &Tree, root: NodeId, x: f32, y: f32) -> HitChain
```

内部改为：

```rust
fn hit_recursive(
    tree: &Tree,
    node_id: NodeId,
    screen: Point,
    current_space: PaintSpace,
    chain: &mut Vec<NodeId>,
) -> bool
```

算法：

```text
node_space = current_space.node_space(node.rect, node.style.transform)
local_point = node_space.local_to_screen.inverse().map(screen)

inside = local_rect.contains(local_point)
if !inside && overflow != Visible:
  return false

for child in hit_order:
  child_space =
    if node has transform:
      node_space.child_space()
    else:
      current_space
  recurse(child, screen, child_space)

if inside && is_hittable:
  hit self
```

Clip / overflow：

- Hidden/Scroll 仍以 local bounds 裁剪 children。
- Phase 2 的 ClipShape path 不参与 shape-aware hit；当前 leaf shape hit 不改变 overflow clip 语义。
- 如果 transform 不可逆，当前节点自身可以按 local inverse 失败视为不命中；children 也不命中。

测试替换：

```text
hit_transform_policy_ignores_rotate_until_paint_supports_it
```

改为：

```text
hit_transform_rotate_uses_affine_inverse
hit_transform_zero_scale_is_not_hittable
hit_transform_translate_scale_preserves_existing_behavior
hit_visible_overflow_keeps_affine_child_hit_order
```

## 11. `tree::paint_target` / `tree::paint_ops` 迁移

### 11.1 `tree::paint_target`

旧 trait 删除：

```rust
pub trait PaintTarget { draw_rect, draw_text, ... }
```

替换为：

```rust
pub use crate::paint::PaintTarget;

pub struct CustomPaintCx { ... }
```

如果需要 legacy replay，放到单独文件：

```rust
pub(crate) struct LegacyDisplayListRenderer<'a> { ... }
```

不要让 `tree::paint_target` 继续定义第二套 draw trait。

### 11.2 `tree::paint_ops`

旧 screen-space `PaintOp` enum 删除。

推荐保留兼容 re-export：

```rust
pub use crate::paint::{
    DisplayList,
    PaintBuildError,
    RecordingPaintTarget,
    ResolvedPaintCommand,
};
```

或直接删除 `paint_ops.rs` 和 `tree/mod.rs` 中的 `pub mod paint_ops`。如果担心外部测试引用，可先保留 re-export shim，但不能保留 `PaintOp`。

## 12. Legacy DisplayList Replay

Phase 2 期间 `Renderer` 还没有 `draw_display_list`，所以需要临时 replay：

```rust
pub(crate) struct LegacyDisplayListRenderer<'a> {
    renderer: &'a mut Renderer,
    textures: Option<&'a HashMap<TextureHandle, TextureResource>>,
    icons: Option<&'a IconRegistry>,
}
```

入口：

```rust
impl LegacyDisplayListRenderer<'_> {
    pub fn render(&mut self, list: &DisplayList) -> LegacyReplayReport;
}
```

`LegacyReplayReport`：

```rust
pub struct LegacyReplayReport {
    pub unsupported: Vec<UnsupportedPaintCommand>,
}
```

支持范围：

| Command | Legacy replay |
|---|---|
| Rect | transform local rect to screen rect if axis-aligned; scale style metrics by approx scale |
| Shadow | same as Rect |
| Path | `PathData::transformed(transform)`; scale stroke width by approx scale |
| Circle | if transform similarity: transformed center + radius * approx scale；non-similarity unsupported |
| Image | if axis-aligned: transform rect and call old `draw_image` |
| Text | if axis-aligned: transform pos/bounds and scale font size |
| Svg | if axis-aligned: resolve icon source from `IconRegistry` and call old `draw_svg_icon` |
| SvgRaster | if axis-aligned: old raster fallback |
| Layer | unsupported until Phase 5 |

不支持时：

- 记录 `UnsupportedPaintCommand`。
- `tracing::warn!` 一次，不 silent fallback。
- 不把 rotated rect 当 bounds 画出来。

这样 Phase 2 的 visual path 对 translate/scale 保持不变；rotate recording/hit 可以测试，但 rotated visual 仍等 Phase 4/5。

Phase 3 删除：

```text
LegacyDisplayListRenderer
LegacyReplayReport
UnsupportedPaintCommand
```

## 13. 实施步骤

### Step 0: 基线

运行：

```text
cargo test -p gui
cargo check --workspace
```

记录已有警告，不混入无关修复。

### Step 1: 补齐 SVG/Icon paint contract

修改：

```text
gui/src/paint/svg.rs
gui/src/paint/command.rs
gui/src/paint/target.rs
gui/src/paint/mod.rs
```

验收：

```text
cargo test -p gui paint::svg
rg "crate::renderer" gui/src/paint
```

第二条应无输出。

### Step 2: 新增 `tree::transform`

修改：

```text
gui/src/tree/transform.rs
gui/src/tree/mod.rs
```

验收：

```text
cargo test -p gui tree::transform
```

### Step 3: 新增 `tree::paint_space`

修改：

```text
gui/src/tree/paint_space.rs
gui/src/tree/mod.rs
```

验收：

```text
cargo test -p gui tree::paint_space
```

### Step 4: 迁移 recording target

修改：

```text
gui/src/tree/paint_ops.rs
gui/src/tree/paint_target.rs
```

目标：

- 删除 screen-space `PaintOp`。
- `tree::paint_target::PaintTarget` 改为 re-export `crate::paint::PaintTarget`。
- `CustomPaintCx` 改字段。

验收：

```text
cargo test -p gui paint::recording
rg "enum PaintOp" gui/src/tree
```

第二条应无输出。`PaintTransform` 会在 Step 5 删除。

### Step 5: 迁移 `tree::paint`

修改：

```text
gui/src/tree/paint.rs
gui/src/tree/paint_helpers.rs
```

目标：

- `paint_to_display_list` 成为 recording 主入口。
- `paint_to_target` 接收 `&mut dyn crate::paint::PaintTarget`。
- 删除 `PaintTransform` 使用。
- 删除 `leaf_path_to_screen` 作为 paint 主路径。
- 所有 leaf command 记录 local primitive。

验收：

```text
cargo test -p gui tree::paint
rg "PaintTransform|leaf_path_to_screen|scaled_rect_style|scaled_text_style" gui/src/tree/paint.rs
```

第二条应无输出。

### Step 6: 添加 legacy replay

修改：

```text
gui/src/tree/legacy_paint_replay.rs
gui/src/tree/mod.rs
gui/src/tree/paint.rs
```

目标：

- `paint()` 仍能画到旧 renderer。
- identity/translate/scale 视觉路径对应旧结果。
- unsupported affine 显式报告。

验收：

```text
cargo test -p gui tree::legacy_paint_replay
cargo test -p gui tree::paint
```

### Step 7: 迁移 `tree::hit`

修改：

```text
gui/src/tree/hit.rs
```

目标：

- 删除 `inverse_supported_transform`。
- hit 使用 `PaintSpace` / `NodePaintSpace`。
- rotate affine inverse 生效。

验收：

```text
cargo test -p gui tree::hit
rg "inverse_supported_transform|Transform .*scale|rotate ignored|ignores rotate" gui/src/tree/hit.rs
```

第二条应无输出。

### Step 8: 全量验证和债务扫描

运行：

```text
cargo fmt --check
cargo test -p gui
cargo check --workspace
cargo clippy -p gui
```

债务扫描：

```text
rg "struct PaintTransform|enum PaintOp|inverse_supported_transform" gui/src/tree
rg "crate::renderer::svg|DrawCommand" gui/src/tree
rg "crate::renderer" gui/src/paint
```

预期：

- 第一条无输出。
- 第二条无输出。
- 第三条无输出。

## 14. 测试矩阵

### `tree::transform`

```text
legacy_transform_spec_preserves_translate_scale_rotate
legacy_transform_affine_matches_translate_scale
legacy_transform_affine_applies_rotate_clockwise_y_down
zero_scale_inverse_returns_none
```

### `tree::paint_space`

```text
ordinary_node_local_to_screen_translates_by_rect
ordinary_children_keep_parent_space
transform_children_use_node_local_space
transform_children_compose_parent_then_node_then_transform
screen_to_local_round_trip_translate_scale
screen_to_local_round_trip_rotate
```

### `tree::paint`

```text
rect_decoration_records_local_rect_and_affine
overflow_hidden_records_local_clip_with_resolved_transform
line_leaf_records_untranslated_local_path
path_leaf_records_local_path_and_transform
text_leaf_records_local_pos_bounds_and_unscaled_style
grid_leaf_records_local_circle_commands
image_leaf_records_local_rect
icon_leaf_records_svg_paint
circle_leaf_records_local_center_radius
connection_leaf_records_path_in_connection_local_space
custom_paint_receives_local_rect_transform_screen_bounds
transform_translate_scale_recording_matches_old_screen_output_after_replay
transform_rotate_records_affine_without_screen_space_points
```

### `tree::hit`

```text
hit_transform_translate_preserves_existing_behavior
hit_transform_scale_preserves_existing_behavior
hit_transform_rotate_uses_affine_inverse
hit_transform_zero_scale_is_not_hittable
hit_hidden_overflow_clips_in_local_space_under_transform
hit_visible_overflow_under_transform_keeps_z_order
```

### legacy replay

```text
replay_rect_identity_matches_old_rect
replay_path_translate_scale_matches_old_screen_path
replay_text_translate_scale_scales_font_for_old_renderer
replay_circle_similarity_supported
replay_rotated_rect_reports_unsupported
replay_layer_reports_unsupported_until_phase5
```

## 15. 验收标准

Phase 2 完成必须满足：

```text
tree paint recording 不再包含 screen-space PaintOp
tree paint 不再使用 PaintTransform
tree hit 不再使用 inverse_supported_transform
tree paint/hit 共用 tree::paint_space 和 tree::transform
RecordingPaintTarget 断言 local primitive + affine + clip state
translate/scale 视觉通过 legacy replay 不回归
rotate recording 和 hit affine inverse 有测试
paint 模块仍不依赖 renderer
tree 不依赖 renderer::DrawCommand 或 renderer::svg internals
```

允许暂存的 Phase 2 临时物：

```text
LegacyDisplayListRenderer
LegacyReplayReport
UnsupportedPaintCommand
```

这些必须在文件和注释中标注：

```text
Temporary Phase 2 bridge. Remove in Phase 3 when renderer consumes DisplayList directly.
```

不允许暂存：

```text
PaintTransform
screen-space PaintOp
inverse_supported_transform
draw_icon trait method as tree PaintTarget core API
silent rotated fallback in legacy replay
```

## 16. 风险和处理

| 风险 | 处理 |
|---|---|
| 普通 children 与 transform children 坐标空间不同 | 所有逻辑集中在 `tree::paint_space`，paint/hit 不再各自猜 |
| Clip push 时 transform 与 children 绘制 transform 不同 | ClipShape 在 push 时 resolve transform，clip stack 与 transform stack 分离 |
| Icon 无 paint-level command | Phase 2 先补齐 generic `SvgPaint` contract，不让 tree 调 renderer SVG |
| Legacy replay 被误当长期 backend | 文件名、类型名、注释、验收清单都标明 Phase 3 删除 |
| Rotate hit 早于 rotate visual | replay 对 rotate 显式 unsupported；recording/hit 先稳定语义，backend 后续完成 |
| CustomPaint 旧 screen rect 依赖 | 直接改 `CustomPaintCx` 字段，不保留 `rect` 别名 |
| 测试大面积改写 | 优先新增 DisplayList assertion helper，避免每个测试手写 enum 匹配 |

## 17. 建议提交拆分

推荐拆成 4 个提交：

1. `Add tree affine paint-space adapters`
   - `tree::transform`
   - `tree::paint_space`
   - pure tests

2. `Complete paint SVG command contract`
   - `paint::svg`
   - `PaintCommand::Svg`
   - IconSpec -> SvgStyle conversion helper

3. `Migrate tree paint recording to DisplayList`
   - `tree::paint`
   - `tree::paint_target`
   - `tree::paint_ops`
   - legacy replay

4. `Migrate tree hit testing to affine space`
   - `tree::hit`
   - inverse tests
   - debt scan cleanup

如果实现时希望单提交，也必须保留上述顺序，确保每个阶段都可局部验证。
