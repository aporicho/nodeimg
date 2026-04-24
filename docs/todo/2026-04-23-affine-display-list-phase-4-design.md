# Affine Display List Rendering Phase 4 设计和执行计划

依据 `docs/todo/2026-04-23-affine-display-list-rendering-plan.md` 和已经落地的 Phase 1/2/3，Phase 4 的目标是让 renderer backend 对基础图元真正消费 affine transform，而不是在 `display_backend` 提前把 geometry 烘焙成 screen-space。

Phase 4 的核心验收是：

```text
rect / path / circle / image / clip 在 renderer prepare 阶段应用 affine
DisplayList lowering 保留 local primitive + resolved affine
vector tessellation cache 不把 transform 纳入 cache key
text / shadow / SVG raster 保持 Phase 3 的 translate+uniform-scale compatibility 路径
不存在旧 clip path / per-corner radius unsupported 分支
```

## 1. 阶段目标

Phase 4 交付以下能力：

1. 新增 renderer 内部 affine helper，集中处理 legacy translate+uniform-scale、similarity scale 和 rect corner transform。
2. 新增 renderer path geometry helper，统一 rounded rect、PathData -> lyon path、circle path 和 fill rule 映射。
3. `renderer::command` 的基础图元请求改为 affine request：local geometry + style + `Affine2D`。
4. `renderer::display_backend` 不再对 rect/path/circle/image/clip 做 screen-space geometry bake。
5. `renderer::prepare` 在 tessellation 或 instance 生成时应用 affine transform。
6. image pipeline instance 从 axis-aligned rect 改成四角坐标，支持旋转、skew 和 non-uniform scale。
7. stencil clip 支持 `Rect`、`RoundedRect`、`Path`，并保留 clip stack push/pop 顺序。
8. 保留 circle similarity fast path；非 similarity circle 使用 vector fallback。

## 2. 严格边界

必须做：

- `display_backend` 只负责资源解析和 command lowering，不负责基础 geometry transform。
- rect/path/circle/image/clip 的 transform 只能在 prepare 或 pipeline instance 阶段应用。
- vector tessellation cache key 只包含 local path data 和 style，不包含 affine transform。
- clip path 和 per-corner rounded clip 必须进入 stencil tessellation，不能 report unsupported。
- shadow、text、SVG raster 继续显式要求 translate + uniform scale，等待 Phase 5。

不能做：

- 不重写 text/shadow/SVG raster pipeline。
- 不引入第二套 matrix 类型。
- 不在 tree 或 paint 层加入 renderer backend 细节。
- 不把 non-similarity circle 当成 identity 或 bounds ellipse 错误绘制。
- 不留下只为编译存在的 legacy request 类型。

## 3. 文件结构

新增：

```text
gui/src/renderer/
  affine.rs          # renderer backend 兼容检查和四角转换
  path_geometry.rs   # lyon path 构建、rounded rect、circle path、fill rule 映射
```

调整：

```text
gui/src/renderer/
  command.rs
  display_backend.rs
  prepare.rs
  vector_tessellator.rs

gui/src/renderer/pipeline/
  image.rs
  quad.rs

gui/src/renderer/shaders/
  image.wgsl
```

## 4. Backend Command Contract

基础图元 command 统一使用 affine request：

```rust
AffineRectRequest {
    rect: Rect,
    style: RectStyle,
    transform: Affine2D,
}

AffinePathRequest {
    data: PathData,
    style: PathStyle,
    transform: Affine2D,
}

AffineCircleRequest {
    paint: CirclePaint,
    transform: Affine2D,
}

AffineImageRequest {
    rect: Rect,
    transform: Affine2D,
    view: Arc<TextureView>,
    size: TextureSize,
    style: ImageStyle,
}

AffineClipRequest {
    shape: ClipShape,
    transform: Affine2D,
}
```

`BackendCommand::Text`、`BackendCommand::Shadow`、`BackendCommand::SvgRaster` 暂时仍保持 screen-space request，因为这些 pipeline 在 Phase 4 不改。

## 5. Lowering 规则

`display_backend` 规则：

- rect：保留 local rect/style/transform。rect shadow 作为兼容分支单独 lowering；非 legacy shadow report `"shadow affine"`，但 rect 本体仍绘制。
- path：保留 local `PathData`、`PathStyle` 和 transform，stroke width 不提前缩放。
- circle：保留 `CirclePaint` 和 transform。
- image：只解析 texture resource，保留 local rect/style/transform。
- clip：保留 `ClipShape` 和 clip transform。
- text/shadow/SVG raster：继续要求 `translate_uniform_scale`。
- SVG vector：在 local rect 中解析 icon paths，然后作为 affine path command 输出。

## 6. Prepare 规则

`prepare` 是 Phase 4 的 affine 落点：

- rect：用 local rounded rect path tessellate fill/stroke，然后把每个 vertex position 乘以 request transform。
- path：vector tessellator 返回 cached local geometry，copy 到 frame buffer 时应用 transform。
- circle：similarity transform 生成原 circle fast-path request；non-similarity transform 生成 local filled circle path 后走 vector fallback。
- image：先在 local rect 上执行 `resolve_image_draw`，再把 resolved rect 四角乘以 transform。
- clip：`ClipShape::{Rect,RoundedRect,Path}` 都 tessellate 到 stencil vertices，并对 vertices 应用 clip transform。

## 7. Image Pipeline

image instance 数据从：

```text
rect: x, y, w, h
```

改为：

```text
p0p1: top-left, top-right
p2p3: bottom-right, bottom-left
uv_rect
modulate
```

shader 继续用 6 个 vertex index 绘制两个三角形，只是 position 从四角数组读取。UV 仍由 local fit/crop 结果决定。

## 8. 验收

必须通过：

```text
cargo fmt
cargo check -p gui
cargo test -p gui
cargo fmt --check
cargo check --workspace
cargo clippy -p gui
```

必须扫描：

```text
rg "paint\\.data\\.transformed\\(transform\\)|legacy\\.rect\\(paint\\.rect\\)" gui/src/renderer/display_backend.rs
rg "UnsupportedClip\\(\"per-corner radius\"|UnsupportedClip\\(\"path\"" gui/src/renderer
```

预期扫描结果为空。`translate_uniform_scale` 在 Phase 4 后只允许出现在 renderer affine helper、text/shadow/SVG raster 兼容路径，以及 rect shadow 兼容分支。
