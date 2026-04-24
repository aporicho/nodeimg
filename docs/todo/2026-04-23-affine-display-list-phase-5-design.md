# Affine Display List Rendering Phase 5 Design

Phase 5 closes the remaining affine gaps for renderer primitives that cannot be
handled by direct CPU tessellation alone: text, shadow, SVG raster fallback, and
layer grouping. The stable contract remains:

```text
paint::DisplayList local primitive + resolved Affine2D
-> renderer::display_backend affine backend commands
-> renderer::prepare draw ops
-> renderer::dispatch GPU resources and passes
```

No tree, paint, hit-test, or widget code depends on renderer pipeline details.

## Backend Command Contract

`renderer::command` keeps every backend command in local geometry plus an
explicit `Affine2D`:

```text
AffineTextRequest { pos, text, style, bounds, transform }
AffineShadowRequest { rect, radius, shadow, transform }
AffineSvgRasterRequest { rect, source, style, transform }
```

The old translate/scale-only path is now an optimization, not a semantic
boundary. A command must never report unsupported only because its transform is
rotated, skewed, or non-uniform.

## Text

Text uses two renderer-private paths:

```text
translate + uniform scale -> existing glyphon main-pass TextRequest
general affine            -> offscreen glyphon raster -> affine image quad
```

`prepare::push_text_op` classifies transforms with
`translate_uniform_scale`. The fast path scales `TextStyle::size` and keeps
existing batching behavior. The affine path emits `DrawOp::AffineText`.

`dispatch::resolve_deferred_ops` resolves `DrawOp::AffineText` before the main
render pass:

1. compute local text bounds from explicit bounds or `TextMeasurer`;
2. allocate an `OffscreenTarget` sized from transformed local edge lengths;
3. render glyphon text into the offscreen target using a raster scale derived
   from target pixels and local bounds;
4. replace the op with `DrawOp::Image` using `PreparedImageDraw::from_rect`.

The offscreen texture, MSAA texture, and depth texture are kept alive until the
submitted command buffer is finished.

## Shadow

Shadow rendering is split into local mask generation and affine composition:

```text
AffineShadowRequest
-> ShadowRequest local cache key
-> blurred local mask texture
-> PreparedImageDraw local destination rect + affine transform
```

The cache key intentionally excludes the affine transform. Rotation, skew, and
scale affect only the image composite, so transform changes do not invalidate
the local shadow mask. Opacity from layers is folded into the shadow color before
the cache key is built.

## SVG Raster

Vector SVG keeps the existing local path resolution path. Raster fallback now
stays affine:

```text
AffineSvgRasterRequest
-> transformed_rect_pixel_size
-> SvgRasterCache
-> PreparedImageDraw::from_resolved(..., transform)
```

The raster size estimate uses transformed local edge lengths instead of an
axis-aligned bounding box. This keeps a rotated icon from being rasterized at an
oversized diagonal envelope while preserving the final affine orientation.

## Layer

`LayerPaint` is lowered as a renderer-private grouping primitive, not as a
backend draw op. `display_backend` recursively lowers the layer content, then
applies the layer transform and opacity to each nested backend command:

```text
nested.transform = layer.transform * nested.transform
nested visual alpha *= layer.opacity
clip transforms are composed; clip opacity is ignored
```

This keeps Layer from becoming an unimplemented no-op in `dispatch` and avoids
exposing a partially implemented offscreen group command. The current
`LayerPaint` contract does not define blend isolation, filters, masks, or
readback effects; if those semantics are added later they should be introduced
as a new isolated layer backend primitive instead of overloading this flattened
path.

## Module Ownership

```text
renderer/offscreen.rs          offscreen target allocation and affine pixel size
renderer/display_backend.rs    DisplayList lowering, Layer flattening, opacity folding
renderer/prepare.rs            text fast-path classification and deferred affine ops
renderer/dispatch.rs           deferred text/SVG/shadow resource resolution
renderer/pipeline/shadow.rs    local mask cache + affine image draw generation
renderer/pipeline/image.rs     affine textured quad draw data
renderer/svg/raster.rs         raster cache only; no transform policy
```

## Verification

Required non-GPU coverage:

```text
display_backend lowers rotated text/shadow/SVG raster as affine commands
display_backend flattens layer transform and opacity into nested commands
prepare keeps translate-scale text on the legacy path
prepare defers rotated text to affine raster path
offscreen pixel sizing uses transformed edge lengths
```

Required command checks before commit:

```text
cargo fmt --check
cargo check -p gui
cargo test -p gui
cargo check --workspace
cargo clippy -p gui
```
