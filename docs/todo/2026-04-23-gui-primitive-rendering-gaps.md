# GUI Primitive Rendering Gaps

Date: 2026-04-23

## TODO Tracking

| Status | Item | Goal | Notes |
| --- | --- | --- | --- |
| TODO | Connect `LeafKind::Line` | A declared line leaf must paint through the renderer | Can initially lower to curve/stroke tessellation if no dedicated line pipeline is added. |
| TODO | Connect `LeafKind::Curve` | A declared curve leaf must paint through `Renderer::draw_curve` | Connection leaves already use curves through custom paint logic, but generic curve leaves are ignored. |
| TODO | Connect `LeafKind::CustomPaint` | The custom paint escape hatch must actually paint | Consider introducing a testable `PaintTarget` or `PaintCx` before exposing too much of `Renderer`. |
| TODO | Decide image tint behavior | `LeafKind::Image.tint` must either work or be removed | Current paint path ignores `tint`. |
| TODO | Design common `Stroke` model | Lines, curves, paths, and borders should not each invent stroke fields | Needs width, color, cap, join, dash, and miter policy. |
| TODO | Design `PathData` primitive | Support general vector paths | Needs move/line/quad/cubic/close, fill, stroke, fill rule. |
| TODO | Connect icon rendering | `LeafKind::Icon` must render through an icon/SVG resource path | `renderer::pipeline::svg::SvgCache` exists but is not connected to `LeafKind::Icon`. |
| TODO | Align paint and hit transforms | Paint and hit testing must agree on rotate/scale/translate | Hit supports rotate; paint currently ignores rotate. |
| TODO | Define scale behavior policy | Primitive dimensions need explicit world-vs-screen scale semantics | Current paint scales radius, border, shadow, text size, and curve width by transform scale. |
| TODO | Add shape-aware hit testing where needed | Thin lines, curves, circles, and paths should not rely only on rectangular bounds forever | Rectangular hit is acceptable for basic UI but not for precise graph interactions. |
| TODO | Add paint-op recording tests | Leaf declarations should be testable without a real GPU renderer | A command recording layer would make `LeafKind -> paint op` coverage explicit. |

## Current Bottom-Up Rendering Path

The current GUI rendering path is broadly layered correctly:

```text
widget / canvas / app
  -> tree::Desc
  -> tree layout
  -> tree::paint
  -> renderer::DrawCommand
  -> renderer::prepare
  -> renderer::dispatch
  -> renderer::pipeline
```

Relevant modules:

- `gui/src/tree/desc.rs`: lightweight view description tree.
- `gui/src/tree/layout/types.rs`: layout style and `LeafKind` primitive declarations.
- `gui/src/tree/paint.rs`: converts the laid out tree into renderer commands.
- `gui/src/renderer/command.rs`: renderer command enum.
- `gui/src/renderer/renderer.rs`: public draw methods used by paint.
- `gui/src/renderer/prepare.rs`: batching, tessellation, and clip preparation.
- `gui/src/renderer/pipeline/*`: GPU pipelines.

## What Works Today

The implementation already supports enough primitives for the current UI and node card work:

- Container decoration renders rectangular and rounded-rect backgrounds.
- Rounded rectangles use the quad pipeline and Figma-style corner smoothing.
- Text supports family, size, weight, italic, line height, clipping, ellipsis, and horizontal alignment.
- Images render through texture handles.
- Circles render through the circle pipeline.
- Cubic curves render through the curve pipeline when called directly by paint.
- Shadows render for rect styles.
- Rounded rectangular clips work through the stencil path.
- Paint and hit order respect `z_index` and source order.

## Main Architectural Gap

`LeafKind` currently declares more primitives than `tree::paint` actually renders.

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
Connection
PendingConnection
```

Ignored or not connected today:

```text
Icon
Line
Curve
Path
CustomPaint
```

This means the primitive interface is not fully trustworthy yet: some declared leaves can be constructed but will silently produce no pixels.

## Missing Common Primitive Capabilities

### Generic Lines And Curves

`LeafKind::Line` and `LeafKind::Curve` should be painted by the generic paint path. Connection rendering should not be the only way to reach the curve renderer.

Minimum expectation:

```text
LeafKind::Line -> Renderer command -> visible stroke
LeafKind::Curve -> Renderer::draw_curve -> visible stroke
```

### Shared Stroke Model

The renderer currently has `Border`, which is good for rectangular decoration but not enough for vector geometry.

A common stroke model should cover:

```text
width
color
line cap
line join
dash pattern
miter limit
```

This should be shared by line, curve, and future path primitives.

### General Path Primitive

`LeafKind::Path` is currently an empty placeholder. It should become structured data rather than an opaque enum variant.

Expected direction:

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

`CustomPainter` and `LeafKind::CustomPaint` exist, but paint does not invoke them yet. This escape hatch should either be connected or removed until it is ready.

Before exposing it widely, consider adding a testable abstraction:

```text
PaintTarget
PaintCx
```

That avoids forcing custom painters to depend directly on the full GPU renderer.

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

`tree::paint` currently calls `Renderer` directly. That works at runtime, but makes primitive coverage harder to test without GPU involvement.

A future recording layer would help:

```text
tree::paint -> PaintOp recorder -> renderer
```

Then tests can assert:

```text
LeafKind::Line produces a line/curve paint op
LeafKind::Icon resolves to an image paint op
LeafKind::CustomPaint invokes the custom painter
```

## Recommended Order

1. Make declared primitives trustworthy:
   - `Line`
   - `Curve`
   - `CustomPaint`
   - image `tint` decision

2. Introduce shared vector styling:
   - `Stroke`
   - cap/join/dash/miter

3. Add general vector paths:
   - `PathData`
   - fill/stroke/fill rule

4. Connect icon/SVG:
   - icon registry
   - `LeafKind::Icon`
   - `SvgCache`

5. Unify transform and scale behavior:
   - rotate support in paint or remove rotate from hit until paint supports it
   - explicit world-vs-screen scale behavior

6. Improve testability:
   - command recorder or `PaintTarget`
   - unit tests for every `LeafKind` branch

