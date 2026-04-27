# Phase 4: Renderer Hot Path

Date: 2026-04-28

## Goal

在 Phase 3 retained paint fragment 稳定之后，优化 renderer/backend 热路径，让 render prepare 和 GPU upload 成本跟 dirty resources 相关，而不是跟整棵 scene 大小线性相关。

重点问题：

```text
grid 不应展开成大量 circle/path commands
text 不应每个元素单独 prepare/render
rounded rect/path 不应每帧 retessellate
pan/zoom/drag 不应重新上传稳定几何
cache eviction 不应散落在各 painter 内部
```

## File Structure

新增和调整：

```text
gui/src/renderer/
  scene_prepare.rs
  grid.rs
  text_batch.rs
  geometry_cache.rs
  upload_arena.rs
  cache_lifecycle.rs

gui/src/paint/
  backend_command.rs
```

规则：

- renderer 只消费 retained scene/paint fragments/prepared commands。
- renderer 不访问 widget runtime、tree mutation、layout internals。
- wgpu/glyphon 等 backend 类型不泄漏到 `tree`、`template`、`widget`。

## Standard APIs

### Prepared Scene

```rust
pub struct RetainedScene {
    pub fragments: Vec<FragmentRef>,
    pub viewport: ViewportState,
    pub frame_revision: Revision,
}

pub struct PreparedFrame {
    pub passes: Vec<RenderPassPlan>,
    pub stats: RendererPrepareStats,
}

impl Renderer {
    pub fn prepare(
        &mut self,
        scene: &RetainedScene,
        caches: &mut RendererCaches,
    ) -> PreparedFrame;
}
```

### Cache Lifecycle

```rust
pub trait RendererCacheLifecycle {
    fn begin_frame(&mut self, frame: FrameId);
    fn mark_used(&mut self, key: CacheKey);
    fn end_frame(&mut self);
}
```

统一接入：

- text buffers
- geometry meshes
- SVG/image resources
- shadows
- grid resources
- upload arena ranges

### Grid Command

```rust
pub struct GridCommand {
    pub viewport: Rect,
    pub spacing: f32,
    pub radius: f32,
    pub color: Color,
    pub transform: Transform2D,
}
```

规则：

- grid 是一个高层 backend command。
- pan/zoom 更新 uniform/transform。
- 不在 paint 阶段展开成每个 dot 的 primitive。

### Text Batch

```rust
pub struct TextBatchKey {
    pub render_target: RenderTargetId,
    pub clip_stack: ClipStackKey,
    pub transform_class: TextTransformClass,
    pub font_context: FontContextId,
}

pub enum TextTransformClass {
    AxisAligned,
    UniformScale,
    AffineFallback,
}
```

规则：

- axis-aligned translate/scale 走正常 text batch。
- rotation/skew 可以先走 offscreen/raster fallback，但必须单独统计。
- text layout cache 来自 Phase 2，不在 renderer 重新 wrap 文本。

### Geometry Cache

```rust
pub struct GeometryCacheKey {
    pub shape: ShapeKind,
    pub radii: CornerRadii,
    pub stroke_width: LayoutScalar,
    pub tolerance: LayoutScalar,
}
```

规则：

- x/y/transform 不进入 geometry key。
- color 尽量作为 material/uniform，不触发 retessellate。
- rounded rect、border、stroke、connection curve 优先缓存。

## Migration Checklist

### 1. Renderer Stats

- [ ] 扩展 `FrameStats` 或新增 `RendererPrepareStats`。
- [ ] 记录 backend command count。
- [ ] 记录 text batch count。
- [ ] 记录 render pass count。
- [ ] 记录 tessellated vertex count。
- [ ] 记录 upload bytes。
- [ ] 记录 cache hits/misses。
- [ ] 记录 grid command expansion count。

验收：

- [ ] debug/perf 日志能区分 UI dirty、paint dirty、renderer prepare dirty。
- [ ] unchanged frame 的 renderer cache hit 可观测。

### 2. Grid Renderer

- [ ] 新增 `renderer/grid.rs`。
- [ ] 将 canvas grid 从大量 primitive 替换为 `GridCommand`。
- [ ] grid shader 或 instanced path 只使用 viewport、spacing、radius、color、transform。
- [ ] pan/zoom 只更新 transform/uniform。
- [ ] 删除生产路径中的 per-dot command expansion。

验收：

- [ ] 大画布 pan 时 grid command count 保持常数级。
- [ ] zoom 时不重建所有 grid dots。
- [ ] grid 视觉和旧实现一致。

### 3. Text Batching

- [ ] 新增 `renderer/text_batch.rs`。
- [ ] renderer prepare 阶段收集 text fragment commands。
- [ ] 按 `TextBatchKey` 分组。
- [ ] text cache `begin_frame/mark_used/end_frame` 统一管理。
- [ ] 删除 painter 内部重复 mark/evict text cache 的逻辑。
- [ ] affine fallback 有明确统计和限制。

验收：

- [ ] 多 TextArea 场景 text render batches 明显少于 text element 数量。
- [ ] 光标/选区变化不导致所有 text buffer 重新 prepare。
- [ ] axis-aligned pan/scale 不落入 affine fallback。

### 4. Geometry Cache

- [ ] 新增 `renderer/geometry_cache.rs`。
- [ ] rounded rect mesh 缓存。
- [ ] border/stroke mesh 缓存。
- [ ] connection curve geometry 缓存或局部更新。
- [ ] style color change 不 retessellate geometry。
- [ ] transform/position change 只更新 instance/uniform。

验收：

- [ ] 拖拽 100 个 node card 不重复 tessellate rounded rect。
- [ ] hover color change 不重建 geometry mesh。
- [ ] connection endpoint 不变时 connection geometry cache hit。

### 5. Upload Arena

- [ ] 新增 `renderer/upload_arena.rs`。
- [ ] 动态 frame buffer 使用 reusable/ring/grow-only 策略。
- [ ] 稳定 geometry 上传后复用 handle。
- [ ] per-frame upload bytes 进入 stats。
- [ ] 避免每帧 create/destroy GPU buffer。

验收：

- [ ] unchanged frame upload bytes 接近 0 或固定小常数。
- [ ] pan/zoom 不重新上传节点稳定几何。
- [ ] resize 只上传 affected geometry 或 instance data。

### 6. Clip/Pass Planning

- [ ] renderer prepare 根据 clip stack 分组。
- [ ] 保持 paint order 正确，不能为了 batching 破坏混合顺序。
- [ ] render pass plan 统计 clip batch 和 pass count。
- [ ] shadow/SVG/image cache 接入统一 lifecycle。

验收：

- [ ] nested clip 下视觉不回归。
- [ ] pass count 在稳定场景中不随 node 数线性增长。
- [ ] cache eviction 不发生在 painter 热路径中。

## Cleanup Gates

```text
rg "Grid" gui/src/paint gui/src/renderer gui/src/canvas
rg "mark_used" gui/src
rg "evict" gui/src/renderer gui/src/widget gui/src/paint
rg "tessell" gui/src
rg "create_buffer" gui/src/renderer
rg "glyphon" gui/src
```

禁止留下：

- [ ] grid 在生产路径展开成大量 dot primitives。
- [ ] 每个 text element 单独 render pass。
- [ ] painter 内部各自管理 backend cache eviction。
- [ ] position/transform change 触发 geometry retessellation。
- [ ] backend 类型泄漏到 widget/tree 层。

## Test Plan

- [ ] `grid_prepare_emits_single_grid_command`
- [ ] `grid_pan_updates_uniform_not_primitives`
- [ ] `text_elements_share_batches_by_compatible_clip`
- [ ] `axis_aligned_text_transform_uses_normal_batch`
- [ ] `rotated_text_uses_counted_affine_fallback`
- [ ] `rounded_rect_geometry_cache_reuses_mesh_on_drag`
- [ ] `style_color_change_does_not_retessellate_geometry`
- [ ] `upload_arena_reuses_stable_geometry_buffers`
- [ ] `unchanged_scene_has_no_dynamic_upload_spike`
- [ ] `clip_batching_preserves_paint_order`

## Performance Acceptance

- [ ] 1000 grid dots visible: backend command count remains constant-level for grid。
- [ ] 100 text fields: text batch count bounded by clip/transform groups。
- [ ] 100 node cards drag: rounded rect tessellation count does not scale with cards。
- [ ] pan/zoom large canvas: upload bytes do not scale with node count。
- [ ] unchanged frame: renderer cache hit rate high and render prepare work minimal。

## Assumptions

- Phase 4 不修 UI invalidation 语义；如果 Phase 1-3 counters 显示仍全量 dirty，必须回前置阶段修。
- Phase 4 可以先保留现有 backend，只调整 command prepare/cache/batch 层。
- pixel-perfect screenshot 不是唯一验收，必须同时有结构化 stats 验收。
