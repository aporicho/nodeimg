# GUI Primitive Rendering Gaps

Date: 2026-04-23

## TODO Tracking

| Status | Item | Goal | Notes |
| --- | --- | --- | --- |
| DONE | Connect `LeafKind::Line` | A declared line leaf must paint through the renderer | Implemented through `PathData::line` and the vector path pipeline. |
| DONE | Connect `LeafKind::Curve` | A declared curve leaf must paint through the renderer | Implemented through `PathData::cubic`; connection leaves now use the same vector path route. |
| DONE | Connect `LeafKind::CustomPaint` | The custom paint escape hatch must actually paint | Implemented through `PaintTarget` and `CustomPaintCx`; it no longer depends on the concrete GPU `Renderer`. |
| TODO | Decide image tint behavior | `LeafKind::Image.tint` must either work or be removed | Current paint path ignores `tint`. |
| DONE | Design common `Stroke` model | Lines, curves, paths, and borders should not each invent stroke fields | `Stroke` now covers width, color, cap, join, and miter limit. Dash remains a future extension. |
| DONE | Design `PathData` primitive | Support general vector paths | `PathData` supports move/line/quad/cubic/close; `PathStyle` supports fill, stroke, and fill rule. |
| TODO | Connect icon rendering | `LeafKind::Icon` must render through an icon/SVG resource path | `renderer::pipeline::svg::SvgCache` exists but is not connected to `LeafKind::Icon`. |
| TODO | Align paint and hit transforms | Paint and hit testing must agree on rotate/scale/translate | Hit supports rotate; paint currently ignores rotate. |
| TODO | Define scale behavior policy | Primitive dimensions need explicit world-vs-screen scale semantics | Current paint scales radius, border, shadow, text size, and curve width by transform scale. |
| TODO | Add shape-aware hit testing where needed | Thin lines, curves, circles, and paths should not rely only on rectangular bounds forever | Rectangular hit is acceptable for basic UI but not for precise graph interactions. |
| DONE | Add paint-op recording tests | Leaf declarations should be testable without a real GPU renderer | `RecordingPaintTarget` records `PaintOp` values for leaf paint assertions. |

## Current Bottom-Up Rendering Path

The current GUI rendering path is broadly layered correctly:

```text
widget / canvas / app
  -> tree::Desc
  -> tree layout
  -> tree::paint
  -> tree::PaintTarget
  -> renderer::DrawCommand
  -> renderer::prepare
  -> renderer::dispatch
  -> renderer::pipeline
```

Relevant modules:

- `gui/src/tree/desc.rs`: lightweight view description tree.
- `gui/src/tree/layout/types.rs`: layout style and `LeafKind` primitive declarations.
- `gui/src/tree/paint.rs`: converts the laid out tree into renderer commands.
- `gui/src/tree/paint_target.rs`: paint target interface, renderer adapter, and custom paint context.
- `gui/src/tree/paint_ops.rs`: recording paint target and testable paint operations.
- `gui/src/renderer/command.rs`: renderer command enum.
- `gui/src/renderer/renderer.rs`: public draw methods used by paint.
- `gui/src/renderer/prepare.rs`: batching, tessellation, and clip preparation.
- `gui/src/renderer/vector_tessellator.rs`: CPU vector path tessellation and cache.
- `gui/src/renderer/pipeline/*`: GPU pipelines.

## What Works Today

The implementation already supports enough primitives for the current UI and node card work:

- Container decoration renders rectangular and rounded-rect backgrounds.
- Rounded rectangles use the quad pipeline and Figma-style corner smoothing.
- Text supports family, size, weight, italic, line height, clipping, ellipsis, and horizontal alignment.
- Images render through texture handles.
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
Icon
```

The remaining unconnected primitive declaration is `Icon`. `Image` renders, but its `tint` field is still ignored.

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

`LeafKind::Icon` exists but is not painted. `renderer::pipeline::svg::SvgCache` exists, but there is no complete path from icon id to texture to `draw_image`.

Needed pieces:

```text
IconRegistry
Icon resource lookup
LeafKind::Icon paint branch
SVG cache integration
```

### Image Tint And Fit

`LeafKind::Image` has `tint: Option<Color>`, but paint currently ignores it.

Future image basics should include:

```text
tint
opacity
fit: contain / cover / stretch
source rect / crop
filter: nearest / linear
```

### Custom Paint Escape Hatch

`CustomPainter` and `LeafKind::CustomPaint` are connected through the generic paint target interface.

Current shape:

```text
CustomPainter::paint(&mut dyn PaintTarget, CustomPaintCx)
```

That avoids forcing custom painters to depend directly on the full GPU renderer, and lets tests record custom paint output through `RecordingPaintTarget`.

### Transform Consistency

Hit testing applies rotate in its inverse transform logic. Paint only composes translate and scale and explicitly ignores rotate.

This can create mismatches:

```text
hit sees rotated geometry
paint shows unrotated geometry
```

Paint and hit must share the same transform semantics.

### Scale Behavior

Paint currently scales radius, border width, shadow metrics, text size, and curve width by transform scale.

That is correct for world-space canvas content, but some UI affordances may need screen-space constant sizing.

The system needs an explicit policy, for example:

```text
ScaleBehavior::ScaleWithWorld
ScaleBehavior::FixedScreen
```

or an equivalent distinction between world units and screen units.

### Shape-Aware Hit Testing

Hit testing currently uses rectangular bounds. This is enough for basic UI containers but weak for precise graph interactions and thin vector shapes.

Eventually needed:

```text
circle hit test by radius
line/curve hit test by stroke distance
path hit test by fill/stroke
```

### Paint Command Recording

`tree::paint` now targets `PaintTarget`. The runtime path uses `RendererPaintTarget`; tests can use `RecordingPaintTarget`.

Current route:

```text
tree::paint -> PaintTarget
  -> RendererPaintTarget -> Renderer
  -> RecordingPaintTarget -> PaintOp
```

Then tests can assert:

```text
LeafKind::Line produces a line/curve paint op
LeafKind::Icon resolves to an image paint op
LeafKind::CustomPaint invokes the custom painter
```

## Recommended Order

1. Make declared primitives trustworthy:
   - image `tint` decision

2. Connect icon/SVG:
   - icon registry
   - `LeafKind::Icon`
   - `SvgCache`

3. Unify transform and scale behavior:
   - rotate support in paint or remove rotate from hit until paint supports it
   - explicit world-vs-screen scale behavior

4. Improve testability:
   - command recorder or `PaintTarget`
   - unit tests for every `LeafKind` branch
