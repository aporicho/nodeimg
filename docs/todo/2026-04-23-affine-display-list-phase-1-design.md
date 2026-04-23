# Affine Display List Rendering Phase 1 设计和执行计划

依据 `docs/todo/2026-04-23-affine-display-list-rendering-plan.md`，Phase 1 的目标不是启用旋转渲染，也不是迁移 tree/renderer 主路径，而是冻结后续所有阶段依赖的 `geometry` 和 `paint` contract。这个阶段必须做到可独立验收、视觉行为不变、没有把旧 screen-space 语义带进新模块。

## 1. 阶段目标

Phase 1 交付以下能力：

1. 新增 `gui/src/geometry`，作为全局唯一的 2D 几何和 affine 数学事实来源。
2. 新增 `gui/src/paint`，作为 DisplayList、PaintCommand、ClipShape、PaintTarget、Recording 的稳定接口层。
3. 将 renderer 里当前纯数据 primitive 迁到 `geometry` 或 `paint`，并在旧 `renderer::*` 路径保留 re-export 兼容层，避免 `paint` 反向依赖 `renderer`。
4. 新增完整的纯逻辑单元测试，覆盖 affine 数学、TransformSpec、DisplayListBuilder、clip stack 和 recording。
5. 不改变 tree paint、hit test、renderer dispatch 的实际运行行为。旧路径继续工作，后续 Phase 2/3 再迁移主链路。

Phase 1 完成后，后续阶段只能往这个 contract 上接入，不再新增第二套 transform 或 recording 语义。

## 2. 严格边界

必须做：

- `geometry` 不依赖任何项目模块。
- `paint` 只依赖 `geometry` 和 paint primitive 数据类型。
- `tree` 和 `renderer` 可以依赖 `geometry`、`paint`。
- `renderer` 原公开类型路径保持可用，例如 `crate::renderer::RectStyle`、`crate::renderer::PathData` 继续通过 re-export 编译。
- 所有新增 public API 有明确语义和测试。

不能做：

- 不在 Phase 1 启用 rotate 的可见行为。
- 不迁移 `tree::paint`、`tree::hit` 到 DisplayList 主路径。
- 不让 `paint` import `crate::renderer::*`。
- 不在 `geometry` 中引用 `PathData`。`transform_path` 的具体实现归属 `paint::path`，否则会破坏 `geometry -> no project dependency`。
- 不新增 dummy backend、silent fallback 或只为编译存在的 TODO 空壳。
- 不把旧 `PaintTransform`、`PaintOp`、`DrawCommand` 作为新模块事实来源。

## 3. 依赖结构

目标依赖方向：

```text
geometry
  -> no project dependency

paint
  -> geometry

tree
  -> geometry + paint
  -> renderer only through temporary legacy paint target until Phase 3

renderer
  -> geometry + paint
```

Phase 1 需要接受一个现实：当前 `Point`、`Rect`、`PathData`、`RectStyle`、`TextStyle`、`ImageStyle` 等类型在 `renderer` 下。如果新 `paint` 直接复用这些路径，就会形成 `paint -> renderer` 债务。推荐在 Phase 1 同步做纯数据类型迁移：

```text
renderer/types.rs       -> re-export geometry::Point/Rect and paint::Color
renderer/style.rs       -> re-export paint::style::*
renderer/path.rs        -> re-export paint::path::*
renderer/image.rs       -> re-export paint::image::*
tree/layout/types.rs    -> re-export paint::TextureHandle
```

这些迁移只移动数据定义，不改变 renderer backend 行为。

## 4. 文件结构

新增和调整后的 Phase 1 文件结构：

```text
gui/src/geometry/
  mod.rs
  types.rs        # Point, Vector, Rect 和基础 rect/point helper
  affine.rs       # Affine2D 和矩阵运算
  transform.rs    # TransformSpec, TransformOrigin

gui/src/paint/
  mod.rs
  style.rs        # Color, Border, Stroke, Fill, TextStyle, Shadow 等纯样式
  path.rs         # PathData, PathCommand, PathStyle, path bounds/transform helper
  image.rs        # TextureSize, ImageStyle, resolve_image_draw 等纯 image 语义
  resource.rs     # TextureHandle 和后续 RenderResources 用的 stable handle
  command.rs      # PaintCommand 和各 primitive paint structs
  display_list.rs # DisplayList, ResolvedPaintCommand, DisplayListBuilder
  target.rs       # PaintTarget trait 和 convenience helpers
  state.rs        # PaintState, transform/clip stack 维护
  clip.rs         # ClipShape, ClipId, ResolvedClip
  layer.rs        # LayerPaint
  recording.rs    # RecordingPaintTarget

gui/src/renderer/
  types.rs        # 兼容 re-export，不再拥有 Point/Rect/Color 定义
  style.rs        # 兼容 re-export，不再拥有样式定义
  path.rs         # 兼容 re-export，不再拥有 path 定义
  image.rs        # 兼容 re-export，backend 可继续调用纯函数

gui/src/lib.rs
  pub mod geometry;
  pub mod paint;
```

命名约束：

- 新稳定接口使用 `crate::paint::PaintTarget`。
- 旧接口 `crate::tree::paint_target::PaintTarget` Phase 1 仍保留，但禁止新增使用点。Phase 2 迁移 tree 后删除或改成 shim。
- 旧 recording `crate::tree::paint_ops::RecordingPaintTarget` Phase 1 不扩展能力。新的 recording 只放在 `crate::paint::recording`。

## 5. `geometry` 设计

### 5.1 基础类型

`geometry::types`：

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
```

`Rect` 必须提供：

```text
min_x / min_y / max_x / max_y
is_empty
contains(Point)
corners() -> [Point; 4]
from_points([Point; 4])
union(Rect)
translate(Vector)
inflate(f32)
```

`renderer::types::Point` 和 `renderer::types::Rect` 不再重复定义，改为 re-export `geometry` 类型。

### 5.2 Affine2D

`geometry::affine`：

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine2D {
    pub xx: f32,
    pub xy: f32,
    pub yx: f32,
    pub yy: f32,
    pub tx: f32,
    pub ty: f32,
}
```

坐标和矩阵语义：

```text
GUI logical pixels
x 向右，y 向下
rotate 单位为 radians
正角度在 y-down 坐标中表现为顺时针旋转

x' = xx * x + xy * y + tx
y' = yx * x + yy * y + ty
```

必须实现：

```text
const IDENTITY
identity()
translation(tx, ty)
scale(scale)
scale_non_uniform(sx, sy)
rotation_radians(angle)
skew(skew_x, skew_y)
compose(parent, child)
then(self, next)
inverse() -> Option<Affine2D>
transform_point(Point) -> Point
transform_vector(Vector) -> Vector
transform_rect_corners(Rect) -> [Point; 4]
transformed_bounds(Rect) -> Rect
approx_uniform_scale() -> f32
is_similarity(epsilon) -> bool
is_axis_aligned(epsilon) -> bool
is_finite() -> bool
```

组合顺序固定：

```text
compose(parent, child) = parent * child
```

也就是先应用 `child`，再应用 `parent`。实现公式必须写单测锁死。

`rotation_radians(angle)`：

```text
xx = cos(angle)
xy = -sin(angle)
yx = sin(angle)
yy = cos(angle)
```

`skew(skew_x, skew_y)`：

```text
x' = x + tan(skew_x) * y
y' = tan(skew_y) * x + y
```

`inverse()`：

- determinant 绝对值小于 `1e-6` 返回 `None`。
- 非 finite 矩阵返回 `None`。
- 不 panic，不 silent substitute identity。

`approx_uniform_scale()`：

```text
sx = length(transform_vector(Vector { x: 1, y: 0 }))
sy = length(transform_vector(Vector { x: 0, y: 1 }))
return (sx + sy) * 0.5
```

它只用于兼容旧 scale-only 逻辑和估算，不作为 affine 几何精确替代。

`is_axis_aligned(epsilon)` 应覆盖普通轴对齐和 90 度旋转：

```text
(xy ~= 0 && yx ~= 0) || (xx ~= 0 && yy ~= 0)
```

### 5.3 TransformSpec

`geometry::transform`：

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformSpec {
    pub translate: [f32; 2],
    pub scale: [f32; 2],
    pub rotate: f32,
    pub skew: [f32; 2],
    pub origin: TransformOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransformOrigin {
    Point(Point),
    Percent { x: f32, y: f32 },
}
```

默认值：

```text
translate = [0, 0]
scale = [1, 1]
rotate = 0
skew = [0, 0]
origin = Percent { x: 0, y: 0 }
```

helper：

```text
TransformOrigin::top_left()
TransformOrigin::center()
TransformOrigin::resolve(bounds: Rect) -> Point
TransformSpec::identity()
TransformSpec::to_affine(bounds: Rect) -> Affine2D
```

`to_affine(bounds)` 矩阵顺序固定：

```text
T(translate) * T(origin) * R(rotate) * K(skew) * S(scale) * T(-origin)
```

Phase 1 不在 `geometry` 里实现 `From<tree::layout::Transform>`，因为这会让 `geometry` 依赖 `tree`。旧 Transform 适配放到 Phase 2 的 `tree::transform`。

## 6. `paint` primitive 设计

### 6.1 样式和颜色

`paint::style` 接管当前 `renderer/style.rs` 的纯数据类型：

```text
Color
Border
Stroke
LineCap
LineJoin
Fill
FillRule
RectStyle
TextFamily
TextWeight
TextStyle
Shadow
```

`paint::Color` 成为颜色事实来源。`renderer::Color` 继续 re-export，保护当前调用点。

### 6.2 Path

`paint::path` 接管当前 `renderer/path.rs`：

```text
PathCommand
PathData
PathStyle
PathRequest
```

新增：

```text
PathData::bounds() -> Option<Rect>
PathData::transformed(Affine2D) -> PathData
```

注意：这是原总体计划里 `transform_path` 的具体落点。不能把它放进 `geometry::Affine2D`，否则 `geometry` 会依赖 `paint`。

`PathData::translated_scaled` 在 Phase 1 可以保留在 re-export 后的旧 API 中，但新增代码不得调用。Phase 4 再退场。

### 6.3 Image 和资源句柄

`paint::image` 接管当前 `renderer/image.rs` 的纯数据类型和纯函数：

```text
TextureSize
ImageFit
ImageFilter
ImageOpacity
ImageSourceRect
ImageStyle
ResolvedImageDraw
resolve_image_draw
```

`paint::resource`：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u64);
```

`tree::layout::TextureHandle` 改为 re-export `paint::TextureHandle`，避免 `PaintCommand::Image` 依赖 tree。

## 7. DisplayList 和 PaintCommand

### 7.1 PaintCommand

`paint::command`：

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PaintCommand {
    Rect(RectPaint),
    Path(PathPaint),
    Circle(CirclePaint),
    Image(ImagePaint),
    Text(TextPaint),
    Shadow(ShadowPaint),
    SvgRaster(SvgRasterPaint),
    Layer(LayerPaint),
}
```

推荐数据结构：

```rust
pub struct RectPaint {
    pub rect: Rect,
    pub style: RectStyle,
}

pub struct PathPaint {
    pub data: PathData,
    pub style: PathStyle,
}

pub struct CirclePaint {
    pub center: Point,
    pub radius: f32,
    pub fill: Option<Color>,
    pub stroke: Option<Stroke>,
}

pub struct ImagePaint {
    pub rect: Rect,
    pub texture: TextureHandle,
    pub style: ImageStyle,
}

pub struct TextPaint {
    pub pos: Point,
    pub text: String,
    pub style: TextStyle,
    pub bounds: Option<Rect>,
}

pub struct ShadowPaint {
    pub rect: Rect,
    pub radius: [f32; 4],
    pub shadow: Shadow,
}
```

`SvgRasterPaint` 在 Phase 1 只定义跨层稳定语义，不接 renderer raster cache：

```rust
pub struct SvgSourceKey {
    pub id: String,
}

pub struct SvgRasterPaint {
    pub rect: Rect,
    pub source: SvgSourceKey,
    pub color: Option<Color>,
}
```

后续 renderer backend 可以把 `SvgSourceKey` 映射到 `renderer::svg::SvgSource`，但 `paint` 不依赖 renderer svg。

### 7.2 LayerPaint

`paint::layer`：

```rust
pub struct LayerPaint {
    pub bounds: Rect,
    pub opacity: f32,
    pub content: Box<DisplayList>,
}
```

`opacity` 必须通过 constructor clamp 到 `[0, 1]`，非 finite 值按 `1.0` 处理。Phase 1 不实现 offscreen backend，只冻结数据语义。

### 7.3 ClipShape

`paint::clip`：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipId(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub enum ClipShape {
    Rect(Rect),
    RoundedRect { rect: Rect, radius: [f32; 4] },
    Path { data: PathData, fill_rule: FillRule },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedClip {
    pub id: ClipId,
    pub shape: ClipShape,
    pub transform: Affine2D,
    pub parent: Option<ClipId>,
}
```

`ClipShape::bounds()`：

- `Rect` 返回 rect。
- `RoundedRect` 返回 rect。
- `Path` 使用 `PathData::bounds()`。

### 7.4 DisplayList

`paint::display_list`：

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayList {
    pub commands: Vec<ResolvedPaintCommand>,
    pub clips: Vec<ResolvedClip>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPaintCommand {
    pub command: PaintCommand,
    pub transform: Affine2D,
    pub clips: Vec<ClipId>,
}
```

每个 command 的几何保持 local-space；`transform` 是 draw 时刻的 resolved affine；`clips` 是 draw 时刻完整 clip stack 的 id 列表。

## 8. DisplayListBuilder 和 PaintTarget

### 8.1 PaintState

`paint::state`：

```rust
pub struct PaintState {
    transform_stack: Vec<Affine2D>,
    clip_stack: Vec<ClipId>,
}
```

不变量：

- `transform_stack` 初始为 `[Affine2D::IDENTITY]`。
- `clip_stack` 初始为空。
- `current_transform()` 永远返回 stack 最后一个 transform。
- push transform 接收 relative transform，新 current 为 `compose(current, relative)`。
- pop transform 不能弹出 identity 根。

### 8.2 Builder 错误处理

为了避免 silent fallback，builder 需要记录 stack 错误：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintBuildError {
    TransformStackUnderflow,
    ClipStackUnderflow,
    UnbalancedTransformStack { depth: usize },
    UnbalancedClipStack { depth: usize },
}
```

`DisplayListBuilder::finish(self) -> Result<DisplayList, PaintBuildError>`：

- 发现 underflow 返回错误。
- transform stack 深度不是 1 返回错误。
- clip stack 非空返回错误。

### 8.3 DisplayListBuilder

```rust
pub struct DisplayListBuilder {
    state: PaintState,
    commands: Vec<ResolvedPaintCommand>,
    clips: Vec<ResolvedClip>,
    error: Option<PaintBuildError>,
}
```

行为：

- `push_transform(relative)` 复合当前 transform 后 push。
- `pop_transform()` underflow 时设置 error，不悄悄忽略。
- `push_clip(shape)` 用 current transform 生成 `ResolvedClip`，记录 parent clip id。
- `pop_clip()` underflow 时设置 error。
- `draw(command)` 记录 local command、current transform、当前 clip id stack。
- `finish()` 做最终 stack 校验。

### 8.4 PaintTarget

`paint::target`：

```rust
pub trait PaintTarget {
    fn push_transform(&mut self, transform: Affine2D);
    fn pop_transform(&mut self);

    fn push_clip(&mut self, clip: ClipShape);
    fn pop_clip(&mut self);

    fn draw(&mut self, command: PaintCommand);

    fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32);
}
```

Convenience helpers 可以作为 default methods，但只能包装 `draw(PaintCommand::...)`：

```text
draw_rect(Rect, RectStyle)
draw_path(PathData, PathStyle)
draw_circle(CirclePaint)
draw_image(Rect, TextureHandle, ImageStyle)
draw_text(Point, &str, TextStyle)
draw_text_clipped(Point, &str, TextStyle, Rect)
draw_shadow(Rect, [f32; 4], Shadow)
draw_svg_raster(Rect, SvgSourceKey, Option<Color>)
```

禁止把 helper 变成稳定核心入口。后续新增 primitive 只能扩展 `PaintCommand`，不能继续扩 trait 的 required methods。

### 8.5 RecordingPaintTarget

`paint::recording`：

```rust
pub struct RecordingPaintTarget {
    builder: DisplayListBuilder,
    measure: Box<dyn FnMut(&str, &TextStyle) -> (f32, f32)>,
}
```

方法：

```text
new()
with_measure(...)
display_list(self) -> Result<DisplayList, PaintBuildError>
commands(&self) -> &[ResolvedPaintCommand]
clips(&self) -> &[ResolvedClip]
```

Recording 测试必须断言：

- local geometry 不被提前变换。
- resolved affine transform 正确。
- clip id stack 正确。
- style 原样保留。

## 9. Phase 1 执行步骤

### Step 0: 基线确认

运行：

```text
cargo test -p gui
cargo check --workspace
```

如果已有失败，记录失败用例，不在 Phase 1 混入无关修复。

### Step 1: 新增 `geometry`

修改：

```text
gui/src/lib.rs
gui/src/geometry/mod.rs
gui/src/geometry/types.rs
gui/src/geometry/affine.rs
gui/src/geometry/transform.rs
```

实现基础类型、Affine2D、TransformSpec 和测试。

本步验收：

```text
cargo test -p gui geometry::
```

### Step 2: 迁移 `Point` / `Rect`

修改：

```text
gui/src/renderer/types.rs
```

把 `Point`、`Rect` 改为 `pub use crate::geometry::{Point, Rect};`。`Color` 暂时仍留在 `renderer/types.rs`，后续 Step 3 再迁到 `paint::style` 并从 renderer re-export。

本步验收：

```text
cargo test -p gui
```

### Step 3: 迁移 paint primitive 类型

新增：

```text
gui/src/paint/mod.rs
gui/src/paint/style.rs
gui/src/paint/path.rs
gui/src/paint/image.rs
gui/src/paint/resource.rs
```

调整：

```text
gui/src/renderer/style.rs
gui/src/renderer/path.rs
gui/src/renderer/image.rs
gui/src/renderer/types.rs
gui/src/tree/layout/types.rs
```

做法：

- `paint::style` 拥有 `Color` 和 style 类型。
- `paint::path` 拥有 path 类型，并增加 `PathData::transformed(Affine2D)`。
- `paint::image` 拥有 image style 和 `resolve_image_draw`。
- `paint::resource` 拥有 `TextureHandle`。
- renderer 和 tree 原路径只做 re-export shim。

本步验收：

```text
cargo test -p gui
rg "crate::renderer" gui/src/paint
```

第二条命令应无输出。

### Step 4: 新增 command/clip/layer 数据结构

新增：

```text
gui/src/paint/command.rs
gui/src/paint/clip.rs
gui/src/paint/layer.rs
```

实现所有 enum/struct、constructor 和基础 helper。

本步验收：

```text
cargo test -p gui paint::command
cargo test -p gui paint::clip
```

### Step 5: 新增 DisplayListBuilder 和状态机

新增：

```text
gui/src/paint/state.rs
gui/src/paint/display_list.rs
```

实现 transform stack、clip stack、builder error 和 `finish()` 校验。

本步验收：

```text
cargo test -p gui paint::display_list
cargo test -p gui paint::state
```

### Step 6: 新增 PaintTarget 和 RecordingPaintTarget

新增：

```text
gui/src/paint/target.rs
gui/src/paint/recording.rs
```

实现 trait、default helper、RecordingPaintTarget。

本步验收：

```text
cargo test -p gui paint::recording
```

### Step 7: 模块导出整理

调整：

```text
gui/src/paint/mod.rs
gui/src/renderer/mod.rs
gui/src/lib.rs
```

推荐 `paint/mod.rs` re-export：

```text
Affine-facing paint contract:
  PaintTarget
  PaintCommand
  DisplayList
  DisplayListBuilder
  ResolvedPaintCommand
  ClipShape
  ClipId
  ResolvedClip
  RecordingPaintTarget

Primitive:
  Color
  RectStyle
  TextStyle
  PathData
  PathStyle
  ImageStyle
  TextureHandle
```

`renderer/mod.rs` 继续 re-export 旧路径，减少本阶段 blast radius。

### Step 8: 全量验证

运行：

```text
cargo test -p gui
cargo check --workspace
cargo clippy -p gui
```

如果 clippy 暂时存在仓库既有问题，只记录和 Phase 1 有关的新问题，不混入无关清理。

## 10. 测试矩阵

`geometry::affine`：

```text
identity leaves point unchanged
translation transforms point but not vector
scale_non_uniform transforms point/vector
rotation_radians_positive_is_clockwise_y_down
compose_applies_child_then_parent
inverse_round_trips_point
inverse_returns_none_for_singular_matrix
transformed_bounds_covers_rotated_rect_corners
approx_uniform_scale_matches_uniform_scale
is_similarity_rejects_skew
is_axis_aligned_accepts_90_degree_rotation
```

`geometry::transform`：

```text
default_is_identity_top_left
origin_percent_resolves_against_bounds
origin_center_rotates_around_rect_center
to_affine_uses_declared_component_order
```

`paint::path`：

```text
path_bounds_for_line_quad_cubic_close
path_transformed_uses_affine_for_all_points
translated_scaled_is_not_used_by_new_paint_modules
```

`paint::display_list`：

```text
draw_records_local_command_and_current_transform
nested_transform_stack_composes_parent_child_order
push_clip_records_shape_transform_parent
draw_under_nested_clips_records_full_clip_stack
pop_transform_underflow_returns_error_on_finish
pop_clip_underflow_returns_error_on_finish
unbalanced_transform_stack_returns_error_on_finish
unbalanced_clip_stack_returns_error_on_finish
```

`paint::recording`：

```text
recording_rect_preserves_local_rect
recording_text_uses_measure_callback
recording_clip_and_transform_are_resolved_once
```

兼容 re-export：

```text
existing_renderer_point_rect_imports_compile
existing_renderer_style_imports_compile
existing_tree_texture_handle_imports_compile
```

## 11. 验收标准

Phase 1 完成必须满足：

```text
cargo test -p gui 通过
cargo check --workspace 通过
cargo clippy -p gui 无 Phase 1 新增问题
rg "crate::renderer" gui/src/paint 无输出
rg "PaintTransform" gui/src/geometry gui/src/paint 无输出
rg "DrawCommand" gui/src/paint 无输出
```

行为验收：

- tree paint 现有视觉路径未改。
- hit test 现有行为未改。
- renderer dispatch 现有行为未改。
- 新 DisplayList recording 只能记录 local primitive + affine + clip state。
- 新模块没有 TODO placeholder、未实现的 public 方法或 identity fallback。

## 12. 后续阶段接口对接点

Phase 2 接 tree 时只允许新增 adapter，不改 Phase 1 contract：

```text
tree::transform
  old tree::layout::Transform -> geometry::TransformSpec
  TransformSpec + node rect -> Affine2D

tree::paint_space
  隔离当前 Transform 父节点 child local-space 习惯

tree::paint
  从旧 tree::paint_target::PaintTarget 迁到 paint::PaintTarget
```

Phase 3 接 renderer 时只消费：

```text
paint::DisplayList
paint::ResolvedPaintCommand
paint::ClipShape
geometry::Affine2D
```

renderer 内部仍可使用 `DrawCommand`、`DrawOp`、pipeline request，但这些只能是 backend 内部类型。

## 13. 债务控制清单

Phase 1 不要求删除旧债，但不能新增债。必须遵守：

- 新 `paint` 模块不 import renderer。
- 新 `geometry` 模块不 import paint/tree/renderer。
- 旧 `tree::paint_target::PaintTarget` 不新增跨模块使用点。
- 旧 `tree::paint_ops::PaintOp` 不作为新 recording 的数据结构。
- 旧 `PathData::translated_scaled` 不被新模块调用。
- 旧 `DrawCommand` 不出现在 `paint` API 中。
- builder stack 错误不能 silent ignore。
- 任何为了兼容保留的 re-export shim 都只在旧模块中存在，新代码直接使用 `geometry` 或 `paint` 路径。
