# GUI Primitive Rendering Gaps

Date: 2026-04-23

## TODO Tracking

| Status | Item | Goal | Notes |
| --- | --- | --- | --- |
| DONE | Connect `LeafKind::Line` | A declared line leaf must paint through the renderer | Implemented through `PathData::line` and the vector path pipeline. |
| DONE | Connect `LeafKind::Curve` | A declared curve leaf must paint through the renderer | Implemented through `PathData::cubic`; connection leaves now use the same vector path route. |
| DONE | Connect `LeafKind::CustomPaint` | The custom paint escape hatch must actually paint | Implemented through `PaintTarget` and `CustomPaintCx`; it no longer depends on the concrete GPU `Renderer`. |
| DONE | Complete image primitive styling | Image leaves need trustworthy texture styling semantics | `LeafKind::Image` now carries `ImageStyle`; tint, opacity, source rect, fit, filter, and texture size metadata flow through paint and renderer. |
| DONE | Design common `Stroke` model | Lines, curves, paths, and borders should not each invent stroke fields | `Stroke` now covers width, color, cap, join, and miter limit. Dash remains a future extension. |
| DONE | Design `PathData` primitive | Support general vector paths | `PathData` supports move/line/quad/cubic/close; `PathStyle` supports fill, stroke, and fill rule. |
| DONE | Connect vector SVG icon rendering | `LeafKind::Icon` must render through an icon/SVG resource path | `IconRegistry` resolves SVG assets from the generated `assets/icons` registry; supported SVG icons render as vector paths with fill/stroke/stroke-width overrides, with raster fallback for complex SVG. |
| DONE | Align paint and hit transforms | Paint and hit testing must agree on rotate/scale/translate | Paint and hit now share the affine DisplayList transform model. |
| DONE | Define primitive unit policy | Primitive dimensions need one explicit unit model | All primitive dimensions are local paint units; scaling and movement come only from the DisplayList transform stack. |
| TODO | Add shape-aware hit testing where needed | Thin lines, curves, circles, and paths should not rely only on rectangular bounds forever | Rectangular hit is acceptable for basic UI but not for precise graph interactions. |
| DONE | Add paint recording tests | Leaf declarations should be testable without a real GPU renderer | `RecordingPaintTarget` records resolved DisplayList commands for leaf paint assertions. |

## Current Bottom-Up Rendering Path

The current GUI rendering path is layered through the affine DisplayList contract:

```text
widget / canvas / app
  -> tree::Desc
  -> tree layout
  -> tree::paint DisplayList recording
  -> renderer DisplayList lowering
  -> renderer affine prepare
  -> renderer::dispatch
  -> renderer::pipeline
```

Relevant modules:

- `gui/src/tree/desc.rs`: lightweight view description tree.
- `gui/src/tree/layout/types.rs`: layout style and `LeafKind` primitive declarations.
- `gui/src/tree/paint.rs`: converts the laid out tree into a paint `DisplayList`.
- `gui/src/paint/*`: local primitive and DisplayList contract.
- `gui/src/renderer/display_backend.rs`: lowers DisplayList commands to renderer backend commands.
- `gui/src/renderer/prepare.rs`: affine batching, tessellation, and clip preparation.
- `gui/src/renderer/vector_tessellator.rs`: CPU vector path tessellation and cache.
- `gui/src/renderer/pipeline/*`: GPU pipelines.

## What Works Today

The implementation already supports enough primitives for the current UI and node card work:

- Container decoration renders rectangular and rounded-rect backgrounds.
- Rounded rectangles use the quad pipeline and Figma-style corner smoothing.
- Text supports family, size, weight, italic, line height, clipping, ellipsis, and horizontal alignment.
- Images render through texture handles with `ImageStyle`.
- SVG icons render through `LeafKind::Icon`, `IconRegistry`, and vector-first SVG resolution.
- Circles render through the circle pipeline.
- Lines, cubic curves, and general paths render through the shared vector path pipeline.
- Custom paint leaves can render through the `PaintTarget` escape hatch.
- Paint output can be recorded without a GPU through `RecordingPaintTarget`.
- Shadows render for rect styles.
- Rounded rectangular clips work through the stencil path.
- Paint and hit order respect `z_index` and source order.

## Main Architectural Gap

Most declared `LeafKind` primitives now render through `tree::paint`.

Declared in `LeafKind`:

```text
Text
Image
Icon
Circle
Line
Curve
Path
Grid
Connection
PendingConnection
CustomPaint
```

Actually handled by `tree::paint` today:

```text
Text
Grid
Image
Icon
Circle
Line
Curve
Path
Connection
PendingConnection
CustomPaint
```

Ignored or not connected today:

```text
None
```

All declared primitive leaves now enter `tree::paint`. `Icon` resolves through SVG icon resources; `Image` renders through a structured style model instead of the removed ad hoc `tint` field.

## Missing Common Primitive Capabilities

### Generic Lines And Curves

`LeafKind::Line` and `LeafKind::Curve` are painted by the generic paint path. Connection rendering is no longer the only way to reach curve rendering.

Current route:

```text
LeafKind::Line -> Renderer command -> visible stroke
LeafKind::Curve -> PathData::cubic -> Renderer::draw_path -> visible stroke
```

### Shared Stroke Model

The renderer currently has `Border`, which is good for rectangular decoration but not enough for vector geometry.

The common `Stroke` model now covers:

```text
width
color
line cap
line join
miter limit
```

Dash pattern is still a future extension.

This should be shared by line, curve, and future path primitives.

### General Path Primitive

`LeafKind::Path` is structured data rather than an opaque enum variant.

Current shape:

```text
PathData
  move_to
  line_to
  quad_to
  cubic_to
  close

PathStyle
  fill
  stroke
  fill_rule
```

### Icon / SVG Rendering

`LeafKind::Icon` is now connected through a vector-first SVG icon path. Built-in
icons are generated from the workspace `assets/icons/*.svg` directory at compile
time; `gui/src/icon/builtin.rs` no longer contains hand-written SVG strings.
`IconName` constants provide typed access to generated asset ids, while `IconId`
remains available for dynamic/custom icon registration.

Current route:

```text
LeafKind::Icon
  -> PaintTarget::draw_icon(rect, spec)
  -> IconRegistry resolves icon_id to generated asset or registered custom SVG
  -> Renderer::draw_svg_icon(...)
  -> SvgVectorCache parses supported SVG path data
  -> PathData + PathStyle
  -> VectorPipeline
```

Compatibility aliases are centralized in `gui/src/icon/aliases.rs`:

```text
close -> xmark
chevron_down -> nav-arrow-down
chevron_right -> nav-arrow-right
```

New UI code should use generated canonical names such as `names::XMARK`,
`names::NAV_ARROW_DOWN`, `names::NAV_ARROW_RIGHT`, and `names::CHECK`.

Supported UI icon SVG subset:

```text
svg width/height/viewBox
path d
fill color
stroke color
stroke-width
stroke-linecap
stroke-linejoin
fill-rule
transform
```

Complex SVG features intentionally use raster fallback instead of silently dropping semantics:

```text
gradient
pattern
mask
filter
clipPath
image
text
dasharray
complex group opacity/blend/isolation
```

The fallback route is:

```text
SvgRasterCache
  -> resvg/tiny-skia rasterization
  -> TextureResource
  -> ImagePipeline
```

### Image Tint And Fit

`LeafKind::Image` now has a complete primitive style:

```text
ImageStyle
  source: ImageSourceRect
  fit: Stretch / Contain / Cover
  filter: Linear / Nearest
  tint: Option<Color>
  opacity: ImageOpacity
```

Texture registration now includes `TextureSize`, so `Contain` and `Cover` can be resolved from actual image aspect ratio instead of guessing.

Current route:

```text
LeafKind::Image
  -> PaintTarget::draw_image(rect, texture, style)
  -> TextureResource { view, size }
  -> Renderer::draw_image(rect, view, size, style)
  -> resolve_image_draw(...)
  -> ImagePipeline
```

### Custom Paint Escape Hatch

`CustomPainter` and `LeafKind::CustomPaint` are connected through the generic paint target interface.

Current shape:

```text
CustomPainter::paint(&mut dyn PaintTarget, CustomPaintCx)
```

That avoids forcing custom painters to depend directly on the full GPU renderer, and lets tests record custom paint output through `RecordingPaintTarget`.

### Transform Consistency

Paint and hit share the same affine transform policy:

```text
translate -> active
scale -> active
rotate -> active
```

The tree records local primitives and transform stacks; renderer preparation
maps those primitives into the frame. Hit testing uses the same affine inverse
path and still tests local rectangular bounds unless shape-aware hit testing is
implemented later.

### Scale Behavior

Primitive dimensions use one policy:

```text
All primitive dimensions are local paint units.
The DisplayList transform stack is the only source of scale and movement.
```

This includes border width, stroke width, text size, radius, shadow offset,
shadow blur, shadow spread, circle radius, clip radius, image rects, text bounds,
and path coordinates.

Viewport-fixed editor affordances, such as selection outlines or resize handles,
should reuse the same primitives from a viewport-coordinate DisplayList branch
that is not under the canvas camera transform. They must not introduce a second
unit system into primitive styles.

### Shape-Aware Hit Testing

Hit testing currently uses rectangular bounds. This is enough for basic UI containers but weak for precise graph interactions and thin vector shapes.

Eventually needed:

```text
circle hit test by radius
line/curve hit test by stroke distance
path hit test by fill/stroke
```

### Paint Command Recording

`tree::paint` targets `PaintTarget`; runtime builds a `DisplayList` through
`RecordingPaintTarget` and then sends that list to the renderer DisplayList
backend. Tests can inspect the same resolved commands without a GPU.

Current route:

```text
tree::paint -> PaintTarget
  -> RecordingPaintTarget -> DisplayList
  -> Renderer::draw_display_list
```

Then tests can assert:

```text
LeafKind::Line produces a local path command
LeafKind::Icon produces an SVG DisplayList command
LeafKind::CustomPaint invokes the custom painter
```

## Recommended Order

1. Make declared primitives trustworthy:
   - image primitive style

2. Connect icon/SVG:
   - vector-first SVG icon rendering

3. Keep transform and unit behavior unified:
   - local primitive dimensions scale through the DisplayList transform stack
   - viewport-fixed affordances reuse primitives outside the camera transform

4. Improve testability:
   - command recorder or `PaintTarget`
   - unit tests for every `LeafKind` branch
