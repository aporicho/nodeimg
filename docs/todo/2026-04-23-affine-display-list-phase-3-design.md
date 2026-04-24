# Affine Display List Rendering Phase 3 设计和执行计划

依据 `docs/todo/2026-04-23-affine-display-list-rendering-plan.md` 和已经落地的 Phase 1/2，Phase 3 的目标是让 renderer 主路径直接消费 `paint::DisplayList`，并删除 tree 层的临时 legacy replay 桥。

Phase 3 不是启用 rotate 视觉渲染，也不是改 GPU pipeline。这个阶段的验收核心是：

```text
tree 只生产 DisplayList
renderer 直接接收 DisplayList
DisplayList -> renderer internal command 的 lowering 只存在于 renderer 内部
旧 DrawCommand 不再是 tree -> renderer 合约
identity / translate / uniform scale 的视觉行为不回归
非 Phase 3 可支持的 affine command 显式 report unsupported，不能 silent fallback
```

## 1. 阶段目标

Phase 3 交付以下能力：

1. 新增 renderer 侧 `display_backend` 边界，负责把 `paint::DisplayList` lowering 为 renderer 内部命令。
2. 新增 renderer 侧资源解析接口，把 `TextureHandle`、`SvgSourceKey` 解析为 renderer 可消费的 texture / SVG source。
3. `Renderer` 新增 `draw_display_list` 入口，成为 frame 内绘制的主入口。
4. `tree::paint` 不再持有 `Renderer`，也不再 import `TextureResource` / `IconRegistry` / `LegacyDisplayListRenderer`。
5. 删除或停用 `tree::legacy_paint_replay`，把 Phase 2 临时桥的报告类型替换为 renderer-owned report。
6. `renderer::command::DrawCommand` 降级为 renderer 内部 `BackendCommand` / `RenderCommand`，不再作为跨层合同。
7. `renderer::prepare` 和 `renderer::dispatch` 只接受 renderer 内部命令或 prepared frame，不接受 tree 产生的 screen-space 命令。
8. 保持现有 pipeline / shader / prepare 行为不变，确保 translate / scale 视觉不回归。

Phase 3 完成后，Phase 4 才开始让基础图元 backend 真正支持 rotate / non-axis-aligned affine。

## 2. 严格边界

必须做：

- `tree` 只能依赖 `paint` / `geometry`，不能依赖 renderer backend 类型。
- renderer 的公开帧绘制入口必须以 `DisplayList` 为输入。
- resource resolving 只能发生在 renderer 边界，不能塞回 tree。
- `DisplayList` 的 command order、clip stack、transform 必须完整进入 renderer lowering。
- `DrawCommand` 如果保留，只能是 renderer 内部类型。
- 非 translate + uniform scale 的 command 在 Phase 3 必须显式 report unsupported。
- Phase 3 的 report 需要包含 command index 和原因，便于测试和后续 Phase 4 定点消除。
- `cargo test -p gui` 中必须有无 GPU 的 pure lowering tests。

不能做：

- 不改 renderer pipeline / shader 来支持 rotate。
- 不做 Phase 4 的 path / rect / image / stencil affine tessellation。
- 不做 Phase 5 的 rotated text / shadow / SVG raster / layer 完整支持。
- 不做 per-primitive unit switching / fixed-size affordance。
- 不做 unrelated layout refactor。
- 不把 `tree::legacy_paint_replay` 挪到 renderer 后继续长期保留为同名 legacy bridge。
- 不新增 silent fallback，例如 rotate command 被当成 identity 画出来。

## 3. 当前状态

Phase 2 后的当前链路是：

```text
Context::render
-> tree::paint(tree, root, renderer, PaintCx)
-> RecordingPaintTarget
-> paint::DisplayList
-> tree::legacy_paint_replay::LegacyDisplayListRenderer
-> Renderer::draw_rect / draw_text / draw_image / draw_path / push_clip
-> renderer::DrawCommand
-> renderer::prepare
-> renderer::dispatch
-> pipeline
```

当前临时桥的位置是问题核心：

```text
tree::paint
  imports Renderer
  imports TextureResource
  imports IconRegistry
  imports LegacyDisplayListRenderer
```

这违反 Phase 3 目标中的依赖方向：

```text
tree -> paint + geometry
renderer -> paint + geometry
```

当前 renderer 内部路径仍然合理，可以复用：

```text
Renderer::begin_frame
-> collect commands
-> Renderer::end_frame
-> dispatch::dispatch
-> prepare::prepare_frame
-> DrawOp
-> pipeline upload/draw
```

Phase 3 不重写这条内部 pipeline，只把输入端从 tree replay 改成 renderer-owned DisplayList lowering。

## 4. 目标链路

Phase 3 完成后的链路固定为：

```text
Context::render
-> tree::build_display_list(tree, root, PaintCx, text_measure)
-> Renderer::draw_display_list(&DisplayList, DisplayRenderResources)
-> renderer::display_backend::lower_display_list
-> renderer::command::BackendCommand
-> renderer::prepare::prepare_frame
-> renderer::dispatch
-> pipeline
```

关键分层：

```text
tree:
  只负责遍历 Tree、布局坐标、生成 DisplayList。

paint:
  继续作为 DisplayList / PaintCommand / ClipShape / Resource Handle 的稳定 contract。

renderer::display_backend:
  唯一负责 DisplayList -> renderer internal command lowering。

renderer::prepare:
  继续负责 internal command -> PreparedFrame。

renderer::dispatch:
  继续负责 PreparedFrame -> GPU render passes。
```

## 5. 文件结构

新增：

```text
gui/src/renderer/
  display_backend.rs       # DisplayList lowering, report, transform compatibility checks
  display_resources.rs     # Display resource resolver interface and default registry adapter
```

可选新增，如果 SVG lowering 逻辑变长：

```text
gui/src/renderer/svg/
  paint.rs                 # paint::SvgStyle -> renderer IconStyle / vector-raster resolve helper
```

调整：

```text
gui/src/renderer/
  mod.rs                   # 导出 draw_display_list 所需的 renderer-private resource adapter
  renderer.rs              # 新增 draw_display_list；commands 重命名为 backend_commands
  command.rs               # DrawCommand -> BackendCommand 或 RenderCommand
  prepare.rs               # 接收 BackendCommand
  dispatch.rs              # 接收 BackendCommand 或 PreparedFrame

gui/src/tree/
  paint.rs                 # 只 build DisplayList；删除 renderer/resource/icon 依赖
  mod.rs                   # 删除 legacy_paint_replay module
  legacy_paint_replay.rs   # 删除

gui/src/context.rs
  render                   # 组装 DisplayList 和 DisplayRenderResources，调用 renderer
```

不调整：

```text
gui/src/renderer/pipeline/*
gui/src/renderer/shaders/*
gui/src/tree/layout/*
gui/src/tree/hit.rs
```

## 6. Renderer Public API

`Renderer` 新增主入口：

```rust
impl Renderer {
    pub fn draw_display_list(
        &mut self,
        list: &DisplayList,
        resources: DisplayRenderResources<'_>,
    ) -> DisplayRenderReport;
}
```

语义：

- 必须在 `begin_frame` 和 `end_frame` 之间调用。
- 可以在同一 frame 内调用多次，追加到同一 `backend_commands` 队列。
- lowering 立即发生，`resources` 只在调用期间借用，不存入 `Renderer`。
- 返回 report；调用方可以选择记录 warning/debug，但 renderer 不 panic。
- report 里出现 unsupported command 时，该 command 被跳过，不能错误绘制。

`Renderer::begin_frame`：

```text
clear backend_commands
reset stencil
store FrameState
```

`Renderer::end_frame`：

```text
dispatch(&backend_commands, ...)
evict shadow cache
clear frame state
```

旧 `draw_rect` / `draw_text` / `draw_image` 等方法处理方式：

1. Phase 3 内部实现可以暂时保留，作为 renderer 内部测试或兼容 wrapper。
2. 新增代码禁止调用这些方法。
3. 如果没有 crate 外部使用点，应把它们降为 `pub(crate)`。
4. 如果为了 API 兼容必须保留 public，它们必须标注为 compatibility wrappers，并通过同一个 backend command 队列，而不能成为第二条主路径。

## 7. Display Resource Interface

`DisplayList` 只包含稳定 handle：

```text
ImagePaint.texture: TextureHandle
SvgPaint.source: SvgSourceKey
SvgRasterPaint.source: SvgSourceKey
```

renderer 需要把这些 handle 解析成 backend resource。新增：

```rust
pub(crate) struct DisplayRenderResources<'a> {
    pub textures: Option<&'a HashMap<TextureHandle, TextureResource>>,
    pub icons: Option<&'a IconRegistry>,
}
```

或更接口化：

```rust
pub(crate) trait DisplayResourceResolver {
    fn texture(&self, handle: TextureHandle) -> Option<TextureResource>;
    fn svg_source(&self, key: &SvgSourceKey) -> Option<SvgSource>;
}
```

推荐采用 trait + 默认 adapter：

```rust
pub(crate) struct RegistryDisplayResources<'a> {
    textures: &'a HashMap<TextureHandle, TextureResource>,
    icons: &'a IconRegistry,
}

impl DisplayResourceResolver for RegistryDisplayResources<'_> { ... }
```

原因：

- unit test 可以构造 empty resolver，不需要 wgpu texture。
- 后续 runtime resource registry 不需要暴露 HashMap。
- tree 不需要知道 texture / SVG 实际存储位置。
- renderer lowering 不依赖 `Context`。

resolver 的返回值必须是 owned clone：

```text
TextureResource       # 内含 Arc<TextureView>
SvgSource             # 内含 Arc<[u8]>
```

这样 `Renderer::draw_display_list` 可以立即 lowering 并把 renderer internal command 存入 frame 队列，不持有外部 borrow。

## 8. Renderer Internal Command

`renderer::command::DrawCommand` 改名为：

```rust
pub(super) enum BackendCommand {
    Shadow(ShadowRequest),
    Rect(QuadRequest),
    Circle(CircleRequest),
    Text(TextRequest),
    Image {
        rect: Rect,
        view: Arc<wgpu::TextureView>,
        size: TextureSize,
        style: ImageStyle,
    },
    SvgRaster(SvgRasterDraw),
    Path(PathRequest),
    PushClip {
        rect: Rect,
        radius: f32,
    },
    PopClip,
}
```

Phase 3 中 `BackendCommand` 仍然可以是 screen-space / axis-aligned 内部命令。关键是它不再离开 renderer 层。

命名约束：

- 不再叫跨层 `DrawCommand`。
- 不从 `renderer::mod.rs` re-export。
- `tree`、`paint`、`context` 不能 import。
- tests 如果需要，只放在 `renderer` module 内部。

Phase 4 再把 `BackendCommand` 扩展为 transform-aware 或替换为 affine prepared input。

## 9. Display Backend Lowering

新增 `renderer::display_backend`：

```rust
pub(super) struct DisplayBackend<'a, R> {
    resources: &'a R,
    svg_vector_cache: &'a mut SvgVectorCache,
}

pub(super) struct DisplayBackendOutput {
    pub commands: Vec<BackendCommand>,
    pub report: DisplayRenderReport,
}

pub(super) fn lower_display_list<R: DisplayResourceResolver>(
    list: &DisplayList,
    resources: &R,
    svg_vector_cache: &mut SvgVectorCache,
) -> DisplayBackendOutput;
```

### 9.1 Report

替换 Phase 2 的 `LegacyReplayReport`：

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplayRenderReport {
    pub unsupported: Vec<UnsupportedDisplayCommand>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnsupportedDisplayCommand {
    pub index: usize,
    pub reason: UnsupportedDisplayReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnsupportedDisplayReason {
    NonUniformTransform,
    NonSimilarityTransform,
    NonAxisAlignedTransform,
    MissingTexture(TextureHandle),
    MissingSvgSource(String),
    UnsupportedCommand(&'static str),
    UnsupportedClip(&'static str),
}
```

Report 规则：

- `index` 对应 `DisplayList.commands` 的 index。
- clip push 失败时，记录当前 command index。
- missing resource 不 panic。
- unsupported command 不生成 backend command。
- report 记录后继续处理后续 command。
- 不支持的 transform 不能用 identity 或 bounds 代替。

### 9.2 Transform Compatibility

Phase 3 只支持旧视觉路径等价的 transform：

```text
translate + uniform positive scale
```

判断 helper：

```rust
fn legacy_translate_uniform_scale(transform: Affine2D) -> Option<LegacyTransform>
```

规则：

- matrix finite。
- `xy == 0` 且 `yx == 0`。
- `xx == yy`。
- `xx >= 0`。
- epsilon 固定在 renderer backend 内部，例如 `1e-5`。

不支持：

- rotate。
- skew。
- non-uniform scale。
- negative scale / reflection。
- singular matrix。

Phase 3 支持的 screen-space transform helper：

```rust
struct LegacyTransform {
    tx: f32,
    ty: f32,
    scale: f32,
}

impl LegacyTransform {
    fn rect(self, rect: Rect) -> Rect;
    fn point(self, point: Point) -> Point;
    fn scalar(self, value: f32) -> f32;
}
```

这些 helper 只存在于 `renderer::display_backend`，不能放回 `tree`。

### 9.3 Command Lowering 表

| PaintCommand | Phase 3 lowering | Unsupported 条件 |
| --- | --- | --- |
| `Rect` | transform rect + scale border/radius/shadow，输出 `BackendCommand::Rect`，有 shadow 时输出 `Shadow` | non-uniform / rotated transform |
| `Path` | transform path points + scale stroke，输出 `Path` | non-uniform / rotated transform |
| `Circle` | transform center + scale radius，输出 one/two `Circle` | non-similarity transform |
| `Image` | resolve texture，transform rect，输出 `Image` | missing texture / non-uniform / rotated transform |
| `Text` | transform pos/bounds，scale text size，输出 `Text` | non-uniform / rotated transform |
| `Shadow` | transform rect + radius + shadow fields，输出 `Shadow` | non-uniform / rotated transform |
| `Svg` | resolve source，vector-first，fallback raster，输出 `Path` or `SvgRaster` | missing source / non-uniform / rotated transform |
| `SvgRaster` | resolve source，output `SvgRaster` directly | missing source / non-uniform / rotated transform |
| `Layer` | report unsupported | always in Phase 3 |

注意：

- `Path` 虽然可以用 `PathData::transformed(Affine2D)` 处理 rotate，但 Phase 3 不允许悄悄提前支持一部分 rotate。否则 path 视觉和 rect/image/text/clip 会出现不一致。
- `Circle` 只接受 similarity transform。non-uniform scale 应该等 Phase 4 用 path fallback 或 ellipse backend。
- `Svg` vector-first 解析可以复用现有 `SvgVectorCache` 和 `resolve_svg_icon_paths`，但 helper 应放在 renderer 内部。
- `SvgRaster` 的 raster pixel size 仍按 transformed axis-aligned rect 计算；rotated raster 等 Phase 5。

### 9.4 Clip Lowering

`DisplayList` 中 command 携带完整 clip stack：

```rust
ResolvedPaintCommand {
    command,
    transform,
    clips: Vec<ClipId>,
}
```

`DisplayList.clips` 保存每个 `ResolvedClip`：

```rust
ResolvedClip {
    id,
    shape,
    transform,
    parent,
}
```

Phase 3 lowering 使用与 legacy replay 相同的 sync 策略，但实现放在 renderer：

```text
active_clips: Vec<ClipId>
desired_clips: command.clips

1. 找 active 和 desired 的 common prefix。
2. pop active 中多余 clip，输出 BackendCommand::PopClip。
3. push desired 中新增 clip，输出 BackendCommand::PushClip。
4. lowering command。
5. list 结束后 pop_all_clips。
```

支持：

- `ClipShape::Rect`
- `ClipShape::RoundedRect` 且四角半径相同

暂不支持：

- `ClipShape::Path`
- per-corner rounded rect clip
- rotated / non-uniform clip transform

unsupported clip 规则：

- 记录当前 command index。
- 当前 command 不绘制。
- active clip stack 不应进入半同步状态。
- lowering 后续 command 时可以重新尝试 sync。

## 10. Tree Paint API 迁移

`tree::paint` 当前负责 build list + replay。Phase 3 改成只 build list：

```rust
pub(crate) struct PaintCx<'a> {
    pub(crate) interaction: Option<&'a InteractionState>,
    pub(crate) text_inputs: Option<&'a TextInputStore>,
    pub(crate) theme: &'a Theme,
}

pub(crate) fn build_display_list(
    tree: &Tree,
    root: NodeId,
    cx: PaintCx<'_>,
    measure_text: impl FnMut(&str, &TextStyle) -> (f32, f32),
) -> Result<DisplayList, PaintBuildError>
```

或者保留现有文件名和内部函数名：

```rust
pub(crate) fn paint_to_display_list(...) -> Result<DisplayList, PaintBuildError>
```

关键要求：

- `tree::paint.rs` 不再 import `Renderer`。
- `tree::paint.rs` 不再 import `TextureResource`。
- `tree::paint.rs` 不再 import `IconRegistry`。
- `PaintCx` 删除 `textures` / `icons` 字段。
- `paint_to_target` 继续作为测试 helper 保留。
- `RecordingPaintTarget` 仍是 tree paint 的 list builder，不直接接 renderer。

## 11. Context Render 迁移

`Context::render` 改为：

```text
1. root 不存在时 return。
2. 调用 tree::build_display_list，text measure 使用 renderer.text_measurer()。
3. 如果 DisplayList build error，warn 后 return。
4. 构造 renderer DisplayRenderResources。
5. 调用 renderer.draw_display_list(&list, resources)。
6. 如果 report 非空，debug/warn 记录 unsupported 数量。
```

示意：

```rust
let list = tree::build_display_list(
    &self.tree,
    root,
    tree::PaintCx {
        interaction: Some(&self.interaction),
        text_inputs: Some(self.systems.text_input_store()),
        theme,
    },
    |text, style| renderer.text_measurer().measure_with_style(text, style),
)?;

let report = renderer.draw_display_list(
    &list,
    DisplayRenderResources::new(self.resources.textures(), &self.icons),
);
```

`Context` 是资源组装层，但不是 lowering 层。

## 12. Prepare / Dispatch 迁移

`renderer::prepare` 当前：

```rust
pub fn prepare_frame(
    commands: &[DrawCommand],
    vector_tessellator: &mut VectorTessellator,
) -> PreparedFrame
```

Phase 3 改为：

```rust
pub(super) fn prepare_frame(
    commands: &[BackendCommand],
    vector_tessellator: &mut VectorTessellator,
) -> PreparedFrame
```

内部逻辑保持不变：

- quad batch flush。
- circle batch flush。
- vector batch flush。
- text request split。
- image draw resolve。
- SVG raster draw op。
- stencil write / clear。

`renderer::dispatch` 当前：

```rust
pub fn dispatch(commands: &[DrawCommand], ...)
```

Phase 3 改为：

```rust
pub(super) fn dispatch(commands: &[BackendCommand], ...)
```

或者更进一步：

```rust
let prepared = prepare_frame(commands, vector_tessellator);
dispatch_prepared(prepared, ...);
```

推荐 Phase 3 只做最小必要迁移，不拆 `dispatch_prepared`，避免混入额外 refactor。Phase 4/5 如果需要 renderer backend 更复杂，再拆。

## 13. SVG Lowering 设计

当前 `Renderer::draw_svg_icon` 同时做三件事：

```text
resolve vector cache
convert SVG paths to PathRequest
fallback SvgRasterDraw
```

Phase 3 应把这段逻辑移动到 renderer display backend helper：

```rust
fn lower_svg(
    paint: &SvgPaint,
    transform: LegacyTransform,
    resources: &dyn DisplayResourceResolver,
    svg_vector_cache: &mut SvgVectorCache,
    out: &mut Vec<BackendCommand>,
    report: &mut DisplayRenderReport,
)
```

style conversion helper：

```rust
fn icon_style_from_svg(style: SvgStyle) -> IconStyle
```

如果 helper 超过一个小段，放到：

```text
gui/src/renderer/svg/paint.rs
```

并从 `renderer/svg/mod.rs` 只做 `pub(crate)` 导出。

SVG resource 解析规则：

```text
paint::SvgSourceKey.id
-> IconRegistry / DisplayResourceResolver
-> renderer::svg::SvgSource
```

不能让 tree 直接拿 `renderer::svg::SvgSource`。

## 14. 详细执行步骤

### Step 0: 基线确认

执行：

```text
cargo test -p gui
cargo check --workspace
```

记录已有失败，不在 Phase 3 混入无关修复。

债务扫描：

```text
rg -n "LegacyDisplayListRenderer|legacy_paint_replay" gui/src
rg -n "DrawCommand" gui/src/tree gui/src/paint gui/src/context.rs
rg -n "Renderer, TextureResource|IconRegistry" gui/src/tree/paint.rs
```

本步不改代码。

### Step 1: 新增 renderer resource interface

新增：

```text
gui/src/renderer/display_resources.rs
```

内容：

- `DisplayResourceResolver` trait。
- `RegistryDisplayResources<'a>` adapter。
- empty resolver for tests，放在 `#[cfg(test)]`。

调整：

```text
gui/src/renderer/mod.rs
```

导出给 `Context` 使用的最小类型：

```rust
pub(crate) use display_resources::RegistryDisplayResources;
```

验收：

```text
cargo test -p gui renderer::display_resources
```

无新增 tree dependency。

### Step 2: 新增 renderer display backend

新增：

```text
gui/src/renderer/display_backend.rs
```

实现：

- `DisplayRenderReport`
- `UnsupportedDisplayCommand`
- `UnsupportedDisplayReason`
- `lower_display_list`
- clip sync
- transform compatibility helper
- rect/path/circle/text/shadow/image/svg lowering

先只接入 unit tests，不接 `Renderer` 主路径。

测试重点：

- rect translate/scale lowering。
- text translate/scale lowering。
- path stroke width scaling。
- circle fill/stroke lowering。
- command clip stack 转 push/pop 顺序。
- final pop_all_clips。
- non-uniform transform report。
- rotate transform report。
- missing texture report。
- missing SVG source report。
- `Layer` report unsupported。

验收：

```text
cargo test -p gui renderer::display_backend
```

### Step 3: Rename internal command

调整：

```text
gui/src/renderer/command.rs
gui/src/renderer/prepare.rs
gui/src/renderer/dispatch.rs
gui/src/renderer/renderer.rs
```

把 `DrawCommand` 改名为：

```text
BackendCommand
```

或：

```text
RenderCommand
```

推荐 `BackendCommand`，因为它强调该类型不是 public draw contract。

验收：

```text
rg -n "DrawCommand" gui/src
```

结果应为空，或只剩历史文档 / changelog，不应剩代码使用。

### Step 4: Renderer 接入 DisplayList

调整：

```text
gui/src/renderer/renderer.rs
gui/src/renderer/mod.rs
```

改动：

- 字段 `commands` 重命名为 `backend_commands`。
- `begin_frame` 清空 `backend_commands`。
- 新增 `draw_display_list`。
- `draw_display_list` 调用 `display_backend::lower_display_list`。
- lowering output 的 `commands` append 到 `backend_commands`。
- `end_frame` dispatch `backend_commands`。

旧 `draw_*` 方法处理：

- 若保留，改为 `pub(crate)` 优先。
- 继续 append `BackendCommand`，但标注 compatibility。
- 不允许 tree 继续调用。

验收：

```text
cargo test -p gui renderer
```

### Step 5: Tree paint 删除 replay

调整：

```text
gui/src/tree/paint.rs
gui/src/tree/mod.rs
gui/src/tree/legacy_paint_replay.rs
```

改动：

- 删除 `LegacyDisplayListRenderer` 使用。
- 删除 `legacy_paint_replay` module。
- 删除 `PaintCx.textures`。
- 删除 `PaintCx.icons`。
- `paint` 改名或拆分为 `build_display_list`。
- `paint_to_target` 保留。
- tree paint 文件不 import renderer 类型。

验收：

```text
rg -n "LegacyDisplayListRenderer|legacy_paint_replay" gui/src/tree
rg -n "crate::renderer|Renderer|TextureResource|IconRegistry" gui/src/tree/paint.rs
```

第一条无输出；第二条无 renderer/resource/icon 相关输出。

### Step 6: Context render 改接新 renderer API

调整：

```text
gui/src/context.rs
```

改动：

- 调用 `tree::build_display_list`。
- 构造 `RegistryDisplayResources`。
- 调用 `renderer.draw_display_list`。
- report 非空时记录 `unsupported` 数量。

验收：

```text
cargo test -p gui
```

### Step 7: Prepare / dispatch tests 收敛

调整：

```text
gui/src/renderer/prepare.rs
gui/src/renderer/dispatch.rs
```

改动：

- 现有 prepare tests 继续针对 `BackendCommand`。
- 新增至少一个从 `DisplayList` lowering 后进入 `prepare_frame` 的测试。
- 确认 text index order、clip depth plan 不回归。

验收：

```text
cargo test -p gui renderer::prepare
cargo test -p gui renderer::dispatch
```

### Step 8: 全量验证和债务扫描

执行：

```text
cargo test -p gui
cargo check --workspace
cargo clippy -p gui
```

债务扫描：

```text
rg -n "LegacyDisplayListRenderer|LegacyReplayReport|UnsupportedPaintCommand" gui/src
rg -n "legacy replay|Temporary Phase 2 bridge|Remove in Phase 3" gui/src
rg -n "DrawCommand" gui/src
rg -n "draw_rect\\(|draw_text\\(|draw_image\\(|push_clip\\(" gui/src/tree gui/src/context.rs
rg -n "crate::renderer" gui/src/tree/paint.rs gui/src/tree/paint_target.rs
```

预期：

- 无 Phase 2 bridge 残留。
- tree 不调用 renderer draw wrapper。
- `DrawCommand` 代码使用清零，或已更名为 renderer-private `BackendCommand`。
- `paint` 模块仍不依赖 renderer。

## 15. 测试矩阵

### `renderer::display_resources`

必须覆盖：

- empty resolver returns missing texture/source。
- registry resolver resolves existing SVG alias。
- resolver clone resource does not borrow beyond call。

### `renderer::display_backend`

必须覆盖：

- rect command lower 为 `BackendCommand::Rect`。
- rect style border/radius/shadow 按 uniform scale 缩放。
- shadow command lower 为 `BackendCommand::Shadow`。
- path command points transform，stroke width 缩放。
- circle fill/stroke 输出符合当前视觉行为。
- text pos/bounds transform，text size 缩放。
- image missing texture report。
- image with texture smoke test 可放 GPU-gated test。
- SVG missing source report。
- SVG vector parse 成 path command。
- SVG unsupported vector fallback 成 raster command。
- `SvgRaster` direct raster command。
- clip stack sync push/pop 顺序。
- nested clip final pop。
- unsupported path clip report。
- per-corner clip report。
- rotate / skew / non-uniform scale report。
- `Layer` report unsupported。

### `renderer::prepare`

必须覆盖：

- `BackendCommand` batch 行为和旧 `DrawCommand` 行为一致。
- text index order 不变。
- image op 仍 resolve `ImageStyle` fit/source。
- SVG raster op 顺序不变。
- stencil write/clear 仍影响 render step clip depth。

### `Context::render`

以 unit/integration 层面覆盖：

- build display list error 会 warn/return。
- renderer report 非空不会 panic。
- 正常 root 能调用 `draw_display_list` 路径。

如果没有合适 mock renderer，不强行引入 dummy backend；优先用 `renderer::display_backend` pure tests。

## 16. 验收标准

Phase 3 完成必须满足：

```text
tree 不再 import Renderer / TextureResource / IconRegistry
tree::legacy_paint_replay 删除
Context::render 调用 Renderer::draw_display_list
Renderer frame 主路径从 DisplayList lowering 到 BackendCommand
DrawCommand 不再作为跨层 API 名称存在
prepare / dispatch 只消费 renderer internal command
identity / translate / uniform scale 视觉行为不变
unsupported affine transform 显式 report
Layer 仍显式 unsupported，等待 Phase 5
paint 模块不依赖 renderer
renderer 可以多次 draw_display_list append 到同一 frame
```

行为验收：

```text
cargo test -p gui
cargo check --workspace
cargo clippy -p gui
```

债务验收：

```text
rg -n "LegacyDisplayListRenderer|LegacyReplayReport|Temporary Phase 2 bridge" gui/src
```

无输出。

```text
rg -n "DrawCommand" gui/src
```

无输出，或只允许在迁移注释中短期出现；最终提交前应清零。

```text
rg -n "crate::renderer" gui/src/tree/paint.rs
```

无输出。

## 17. 风险和处理

| 风险 | 处理 |
| --- | --- |
| Phase 3 被做成 Phase 2 bridge 搬家 | 文件名和类型名禁止使用 Legacy；新的 lowering 位于 renderer::display_backend，并删除 tree bridge |
| resource resolver 让 renderer 借用 Context 生命周期 | `draw_display_list` 立即 lowering，internal command 持有 owned clone，不保存 resolver borrow |
| rotate 被部分图元提前支持导致视觉不一致 | Phase 3 对 rotate 统一 report unsupported，Phase 4 再整体推进基础图元 affine backend |
| SVG 解析逻辑继续挂在 Renderer public draw method | 移到 display_backend 或 renderer::svg::paint helper，Renderer public API 只暴露 draw_display_list |
| 旧 draw_* wrapper 成为第二主路径 | 降为 pub(crate) 或 compatibility wrapper；新增代码和 tree 禁止调用 |
| prepare/dispatch 改动面扩散 | Phase 3 只做类型输入迁移，不改 batching、stencil、text split 策略 |
| image lowering pure test 难构造 TextureView | pure test 覆盖 missing texture；成功路径用 GPU-gated test_support 或保留在 integration smoke |
| clip sync 出错导致后续 command clip stack 污染 | unsupported clip 不修改 active stack；每个 command sync 前从 active/desired prefix 重新计算 |

## 18. 建议提交拆分

推荐拆成 5 个提交：

1. `Add renderer display resource resolver`
   - `renderer/display_resources.rs`
   - resolver tests

2. `Add DisplayList renderer lowering`
   - `renderer/display_backend.rs`
   - report types
   - pure lowering tests

3. `Make renderer commands backend-internal`
   - `DrawCommand` -> `BackendCommand`
   - update `prepare` / `dispatch` / renderer tests

4. `Route Renderer through DisplayList`
   - `Renderer::draw_display_list`
   - `Context::render`
   - frame command queue rename

5. `Remove Phase 2 legacy replay`
   - delete `tree/legacy_paint_replay.rs`
   - remove tree renderer/resource dependencies
   - debt scans and full validation

如果实际实现中希望 fewer commits，也必须保留上述顺序。先有 resolver 和 lowering tests，再切主路径，最后删 bridge。

## 19. 自查结论

架构一致性：

- `DisplayList` 继续是唯一跨层绘制 contract。
- renderer 通过 `draw_display_list` 接入，不把 backend command 暴露给 tree。
- resource resolver 是 renderer 边界接口，避免 tree 直接碰 wgpu texture / SVG source。

渲染管线完整性：

- Phase 3 复用现有 prepare / dispatch / pipeline，不混入 shader 和 tessellation 改造。
- text split、shadow prepare、stencil clip depth、SVG raster cache 都保留原运行方式。
- unsupported affine transform 进入 report，不产生错误画面。

代码债务控制：

- 删除 Phase 2 bridge，而不是改名长期保留。
- `DrawCommand` 更名并私有化为 renderer internal command。
- 旧 draw wrappers 不再允许成为 tree 主路径。
- 所有 Phase 3 临时限制都有明确 report 和 Phase 4/5 接续位置。
