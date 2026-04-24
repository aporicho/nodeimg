# Affine Display List Rendering Phase 7 Design

Phase 7 removes the remaining compatibility surface from the affine DisplayList
migration. It does not add renderer features. The renderer, tree paint, hit
testing, custom paint, connection endpoints, and workspace camera already use
the affine path; this phase makes that path the only live contract.

## Stable Boundaries

```text
geometry        -> TransformSpec + Affine2D
tree style      -> Option<TransformSpec>
tree paint/hit  -> PaintSpace using TransformSpec::to_affine
paint paths     -> PathData::transformed(Affine2D)
renderer        -> DisplayList consumer, unchanged
```

`TransformSpec` is now the single public transform declaration. Tree layout no
longer defines a parallel `Transform` type or a legacy adapter. The convenience
constructors `translate_scale` and `translate_scale_rotate` preserve the old
workspace-camera ergonomics while keeping rotate active in the affine matrix.

## Removed Debt

The following live-code debt is removed in this phase:

```text
tree::layout::Transform
tree::transform legacy adapter
tree::paint_ops compatibility shim
PathData::translated_scaled
rotate reserved/inert comments
```

Renderer-internal backend commands stay internal. In particular,
`BackendCommand::PushClip` is not a cross-layer tree-to-renderer API and is not
part of this cleanup.

## Verification

Required logic coverage:

```text
TransformSpec::translate_scale preserves old top-left translate/scale semantics
TransformSpec::translate_scale_rotate includes rotate in Affine2D
PaintSpace uses TransformSpec for paint and hit coordinate conversion
workspace canvas_root transform equals camera pan/zoom state
PathData affine transformation remains covered through transformed(Affine2D)
```

Required commands:

```text
cargo fmt --check
cargo check -p gui
cargo test -p gui
cargo test -p app
cargo check --workspace
cargo clippy -p gui
```

Required live-code scans:

```text
rg -n "PaintTransform|inverse_supported_transform|enum PaintOp|struct PaintOp|DrawCommand|translated_scaled|screen_rect|screen-space PaintOp|reserved/inert|等待 renderer|保留字段" gui/src app/src
rg -n "pub mod paint_ops|tree::paint_ops|legacy_transform" gui/src app/src
```
