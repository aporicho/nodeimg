已按 24 个维度审查过，下面是收敛后的完整工程计划。核心判断是：这次不要做 “rotate 补丁”，而要做一次 **Affine Display List Rendering** 架构升级。阶段只是实施顺序，不是把 text/shadow/SVG raster 等留到未来；最终 milestone 验收前，当前所有图元都必须接入同一套 affine 语义。

**推荐总体方案**

目标链路固定为：

```text
tree/widget/canvas
-> tree layout
-> tree paint adapter
-> paint::DisplayList
-> renderer::display_backend
-> renderer::prepare
-> renderer pipeline
```

长期稳定的 API 只有这几层：

```text
geometry::Affine2D
geometry::TransformSpec
paint::PaintTarget
paint::PaintCommand
paint::DisplayList
paint::ClipShape
paint::ResolvedPaintCommand
```

renderer 内部如何 tessellate、batch、cache、用 shader matrix 还是 four-corners instance，都属于 backend 细节，后续可以改，不再影响 tree / paint / hit / recording / CustomPaint。

当前代码的关键问题是：`tree::paint` 先把图元变成 screen-space，`hit.rs` 自己维护 inverse，`PaintOp` 记录变换后浮点点位，renderer command 大量假设 axis-aligned rect。新方案统一改成：

```text
local primitive + resolved affine transform + clip state
```

明确不做：

```text
不做 per-primitive unit switching / scale policy
不做 screen-space fixed affordance
不做 unrelated layout refactor
Phase 2 当时未包含 shape-aware hit testing（后续已由 tree::hit_shape 完成叶子图元精确命中）
```

Phase 2 的 hit 必须支持 affine inverse；当前实现已在此基础上加入 leaf shape hit，支持 circle / line / curve / path / grid / connection / pending connection 的几何命中。

**文件结构**

新增稳定模块：

```text
gui/src/geometry/
  mod.rs
  types.rs
  affine.rs
  transform.rs

gui/src/paint/
  mod.rs
  command.rs
  display_list.rs
  target.rs
  state.rs
  clip.rs
  layer.rs
  recording.rs
```

调整现有模块职责：

```text
gui/src/tree/
  transform.rs       # tree Transform/TransformSpec -> Affine2D
  paint_space.rs     # 当前 layout rect 坐标习惯的适配层
  paint.rs           # 只生产 DisplayList
  hit.rs             # 使用同一 transform adapter 的 inverse

gui/src/renderer/
  display_backend.rs # DisplayList -> PreparedFrame
  prepare.rs
  transform.rs
  command.rs         # 降级为 renderer backend 内部类型，或最终删除
  pipeline/*
  shaders/*
```

依赖方向必须固定：

```text
geometry -> no project dependency
paint -> geometry + primitive style/path/image/text types
tree -> geometry + paint
renderer -> geometry + paint
```

tree 不依赖 renderer pipeline，renderer 不理解 tree node。

**核心数学 API**

`Affine2D` 放在 `gui/src/geometry/affine.rs`：

```rust
pub struct Affine2D {
    pub xx: f32,
    pub xy: f32,
    pub yx: f32,
    pub yy: f32,
    pub tx: f32,
    pub ty: f32,
}
```

坐标系固定为 GUI logical pixels，x 向右，y 向下。`rotate` 单位固定为 radians。正角度在 y-down 坐标中表现为顺时针旋转。

必须一次实现并测试：

```text
identity
translation
scale
scale_non_uniform
rotation_radians
skew
compose
inverse
transform_point
transform_vector
transform_path
transform_rect_corners
transformed_bounds
approx_uniform_scale
is_similarity
is_axis_aligned
```

矩阵语义：

```text
x' = xx * x + xy * y + tx
y' = yx * x + yy * y + ty
```

组合顺序固定为：

```text
parent_to_screen * child_to_parent
```

也就是 `compose(parent, child)` 表示先应用 child，再应用 parent。

**TransformSpec**

不要让旧的 `Transform { translate, scale, rotate }` 成为长期 API。新增稳定声明类型：

```rust
pub struct TransformSpec {
    pub translate: [f32; 2],
    pub scale: [f32; 2],
    pub rotate: f32,
    pub skew: [f32; 2],
    pub origin: TransformOrigin,
}
```

`TransformOrigin`：

```rust
pub enum TransformOrigin {
    Point(Point),
    Percent { x: f32, y: f32 },
}
```

默认值为 top-left：`Percent { x: 0.0, y: 0.0 }`。提供 `top_left()` 和 `center()` helper。

组件矩阵顺序固定为：

```text
T(translate) * T(origin) * R(rotate) * K(skew) * S(scale) * T(-origin)
```

旧 `Transform` 可以暂时保留为 compatibility wrapper：

```text
Transform { translate, scale: f32, rotate }
-> TransformSpec { translate, scale: [scale, scale], rotate, skew: [0,0], origin: top_left }
```

但 tree paint/hit 内部只能使用 `TransformSpec -> Affine2D`。

**Paint API**

`PaintTarget` 稳定为 transform/clip stack + 单一 draw 入口，不再把 trait 扩成一堆未来会不断变多的 `draw_*` 方法：

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

可以保留 convenience helpers，比如 `draw_rect()`、`draw_path()`，但它们必须只是 `draw(PaintCommand::...)` 的 wrapper，不能成为稳定核心接口。

`PaintCommand` 一次覆盖完整链路：

```rust
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

`DisplayListBuilder` 负责 resolve 当前 transform stack / clip stack：

```rust
pub struct DisplayList {
    pub commands: Vec<ResolvedPaintCommand>,
}

pub struct ResolvedPaintCommand {
    pub command: PaintCommand,
    pub transform: Affine2D,
    pub clips: Vec<ClipId>,
}
```

Recording tests 断言：

```text
local geometry
resolved affine transform
clip state
style
```

不再断言已经变换后的 screen-space 浮点坐标。

**ClipShape**

clip API 一次定型：

```rust
pub enum ClipShape {
    Rect(Rect),
    RoundedRect { rect: Rect, radius: [f32; 4] },
    Path { data: PathData, fill_rule: FillRule },
}
```

当前 backend 可以先主要实现 rounded rect，但 DisplayList contract 不再局限于 `PushClip { rect, radius }`。

**Renderer API**

主入口改为：

```rust
Renderer::draw_display_list(&mut self, list: DisplayList, resources: RenderResources)
```

现有 `Renderer::draw_rect/draw_path/draw_image/...` 可以保留，但只能构造 identity transform 的 DisplayList。renderer 主路径不再消费 tree 直接传来的 screen-space geometry。

`renderer::display_backend` 做：

```text
DisplayList
-> support classification
-> prepare_frame
-> DrawOp
-> pipeline upload/render
```

`DrawCommand` 不再是跨层 API，只能作为 renderer backend 内部类型，或迁移完成后删除。

**Primitive 完整方案**

| 图元 | 稳定语义 | 最终 backend |
|---|---|---|
| Rect / RoundedRect | local rect + style + affine | axis-aligned fast path；rotate/skew 转 rounded-rect path tessellation |
| Border | local stroke width + affine | 与 rect 同 shape stroke；non-uniform/skew 走 path stroke fallback |
| Path / Vector | `PathData` 永远 local-space | local tessellation cache；append 时 transform vertices；cache key 不含 transform |
| Circle | local center/radius + affine | similarity 下圆；non-uniform/skew 转 ellipse path |
| Image | local rect + uv/style + affine | contain/cover 先 local resolve，再 affine quad composite |
| Text | local origin/bounds + affine | axis-aligned glyphon 直绘；rotated/skewed offscreen layer -> affine texture quad |
| Clip / Stencil | `ClipShape + affine` | transformed clip path 写 stencil；nested clip depth 保持 |
| Shadow | local rect/radius/shadow + affine | local shadow mask cache -> affine composite；rotate 不 silent fallback |
| SVG Vector | SVG path 保持 SVG local/viewBox | IconLayout local transform 与 paint transform compose |
| SVG Raster | fallback texture + affine | transformed bounds 估算 raster size，结果 affine image composite |
| Grid | local grid primitive | 可先生成 circle/path commands，未来 backend 优化 |
| Connection | endpoint 从 paint-space index 求得 | port local center 转换到 connection local，生成 local path |
| CustomPaint | 只依赖 PaintTarget | `CustomPaintCx` 给 local rect、current transform、screen bounds |

`CustomPaintCx` 改为：

```rust
pub struct CustomPaintCx {
    pub local_rect: Rect,
    pub transform: Affine2D,
    pub screen_bounds: Rect,
}
```

Custom painter 不再拿错误的 screen rect 后自己猜坐标。

**Tree Paint / Hit 语义**

tree 层新增 `tree::transform` 和 `tree::paint_space`：

```text
node rect + TransformSpec + parent affine
-> node local-to-screen affine
-> child space affine
```

当前 layout 里 transform 节点的子节点已经有 local-space 特例，所以不要让这个历史行为泄漏到 paint API。用 `paint_space.rs` 隔离兼容逻辑。

hit test 使用同一 adapter：

```text
screen point
-> accumulated_affine.inverse()
-> local point
-> local bounds / ClipShape bounds check
```

hit chain 顺序、z-index、source order 不变。overflow hidden/scroll 使用同一 clip 语义。Phase 2 当时仍只做 local bounds / clip bounds；后续 shape-aware hit testing 已完成：`tree::hit_shape` 负责叶子图元几何命中，`hit.rs` 仍只负责 traversal、overflow clip、z-order 和 hittable 判定。

**完整实施计划**

| Phase | 目标 | 修改文件 | 验收 |
|---|---|---|---|
| 0 | 写正式设计文档和 invariants | `docs/todo/2026-04-23-affine-display-list-rendering-plan.md` | API、矩阵顺序、origin、clip、text/shadow/layer 路线全部定死 |
| 1 | 新增 `geometry` + `paint` 稳定模块 | 新增 `gui/src/geometry/*`, `gui/src/paint/*`, 更新 `gui/src/lib.rs` | `Affine2D`、`TransformSpec`、`DisplayListBuilder`、Recording 纯测试通过；视觉不变 |
| 2 | tree paint/hit 接入 DisplayList + Affine2D | `tree/paint.rs`, `tree/hit.rs`, `tree/transform.rs`, `tree/paint_space.rs`, `tree/paint_target.rs`, `tree/paint_ops.rs` | 删除 `PaintTransform` 事实来源；recording 断言 local command + affine；translate/scale 行为不变 |
| 3 | renderer 主路径消费 DisplayList | `renderer/renderer.rs`, `renderer/display_backend.rs`, `renderer/prepare.rs`, `renderer/command.rs` | 旧 `draw_*` 变 wrapper；identity/translate/scale 输出与旧路径一致 |
| 4 | 基础图元 affine backend | `quad.rs`, `vector_tessellator.rs`, `circle.rs`, `image.rs`, `stencil.rs`, shaders | rect/path/circle/image/clip/SVG vector 支持 rotate；path cache 不含 transform |
| 5 | Text / Shadow / SVG raster / Layer 完整接入 | `paint/layer.rs`, `pipeline/text.rs`, `pipeline/shadow.rs`, `svg/raster.rs`, image affine composite | rotated text、rotated shadow、SVG raster fallback 都正确 affine 输出 |
| 6 | CustomPaint / workspace camera / connection 完整回归 | `paint/target.rs`, `tree/paint.rs`, `canvas/camera.rs`, workspace tests | CustomPaintCx 正确；camera pan/zoom 不变；connection endpoint under affine 正确 |
| 7 | 清理旧债并正式启用 rotate | `geometry/transform.rs`, `tree/layout/types.rs`, `tree/paint_space.rs`, `path.rs`, docs/tests | 删除旧 `Transform`/legacy adapter/paint_ops shim/helper/comments；`TransformSpec::rotate` 正式生效 |

这些阶段属于同一个 affine milestone。Phase 5 不是未来事项，最终合并前必须完成。

**测试计划**

纯逻辑测试，不需要 GPU：

```text
Affine2D identity/compose/inverse/singular
Affine2D rotate 90/180
TransformSpec origin top-left/center/percent
transform_path and transformed_bounds
DisplayListBuilder transform stack
DisplayListBuilder clip stack
RecordingPaintTarget local geometry + transform
tree paint translate/scale/rotate recording
hit inverse affine consistency
overflow hidden/scroll transformed clip
path cache key excludes transform
SVG vector IconLayout transform order
CustomPaintCx local_rect/current transform
workspace Camera pan/zoom equivalence
```

prepare/backend 测试，优先无 GPU：

```text
rounded rect transformed tessellation vertices
circle non-uniform -> ellipse path fallback
image affine quad instance data
stencil transformed clip geometry
shadow transformed composite rect/corners
SVG raster transformed bounds pixel estimate
text layer command generation for rotated text
```

GPU smoke tests，用 `gui/src/renderer/test_support.rs`，无设备时 skip：

```text
image affine quad renders
stencil nested transformed clip renders
text offscreen affine composite renders
shadow affine composite renders
SVG raster fallback renders
```

最终命令：

```text
cargo test -p gui
cargo check --workspace
cargo clippy -p gui
```

**验收标准**

最终 milestone 必须满足：

```text
所有当前 LeafKind 都进入 DisplayList
renderer 主路径不再依赖 screen-space DrawCommand
paint 和 hit 共用同一 Affine2D adapter
rotate 对 rect/path/circle/image/text/clip/shadow/SVG/custom paint 行为一致
RecordingPaintTarget 能断言 local geometry + transform
workspace camera 行为不回归
无 silent unsupported fallback
```

允许 backend 内部 warning 的场景只能是资源失败，例如 SVG rasterization 失败；不能因为 transform 不支持而悄悄画错。

**代码债务清理清单**

最终必须删除或替换：

```text
PaintTransform { tx, ty, scale }
inverse_supported_transform
PathData::translated_scaled 作为主要 transform 路径
screen-space PaintOp
PushClip { rect, radius } 作为跨层 API
rotate ignored tests
rotate reserved/inert comments
renderer DrawCommand 作为 tree->renderer contract
CustomPaint 只拿 screen_rect
duplicated transform math
```

清理阶段：

```text
Phase 2: PaintTransform / inverse helper 不再作为事实来源
Phase 3: DrawCommand 降级为 renderer 内部
Phase 7: PathData::translated_scaled 退场
Phase 5: text/shadow/raster silent mismatch 消失
Phase 7: 删除旧 tests/comments/imports/API
```

**主要风险和处理**

| 风险 | 处理 |
|---|---|
| 改动面大 | 用 contract-first，Phase 1 只新增稳定 API 和测试 |
| layout 当前坐标习惯复杂 | 用 `tree/paint_space.rs` 隔离，不污染 paint/renderer |
| text rotated 实现成本高 | 从 DisplayList 一开始纳入 `LayerPaint`，Phase 5 完整接入 |
| shadow cache 被 transform 污染 | cache local mask，affine 只影响 composite |
| path cache 爆炸 | cache key 不含 transform，只缓存 local tessellation |
| old/new 双路径长期共存 | Phase 7 验收前必须删旧债，旧 `draw_*` 只能是 wrapper |

**最终结论**

这份计划的核心是：先冻结 `geometry` 和 `paint DisplayList` 两个模块的 contract，然后把 tree、hit、renderer、recording、custom paint 全部迁到这个 contract。当前所有图元都在同一个 milestone 内完成 affine 支持，包括 text、shadow、SVG raster 和 clip。完成后，未来优化 GPU pipeline、缓存策略、文字质量、阴影质量，都只在 renderer backend 内部发生，不再大改上层 API。
