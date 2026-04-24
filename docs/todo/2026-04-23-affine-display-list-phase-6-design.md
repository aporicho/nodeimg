# Affine Display List Rendering Phase 6 Design

Phase 6 closes the upper-layer affine regressions after the renderer backend
became affine-complete. It does not add renderer primitives. The goal is to lock
the tree/canvas contracts that feed the DisplayList renderer.

## Stable Boundaries

```text
workspace camera -> tree Transform on canvas_root
tree paint       -> PaintSpace / CustomPaintCx / connection endpoint resolver
paint target     -> DisplayList local primitive + Affine2D
renderer         -> unchanged Phase 5 backend
```

`Camera` remains a canvas/workspace model. It is not coupled to tree layout or
renderer types beyond the workspace view translating `x/y/zoom` into a canvas
root `Transform`.

## CustomPaint

`CustomPaintCx` is the custom painter contract:

```text
local_rect    node-local bounds
transform     node local-to-screen Affine2D
screen_bounds transformed local_rect bounds
```

Custom painters still draw through `PaintTarget`; they do not receive renderer
commands or tree nodes. Phase 6 verifies this context under parent affine
transforms so custom paint no longer relies on an implicit screen-space rect.

## Connection Endpoints

Connection endpoint lookup is a tree-internal service:

```text
tree::connection_endpoint::node_screen_center(tree, stable_id)
```

It traverses the tree in paint order, uses `PaintSpace` to compose node
transforms, and returns the target node's local center in screen space. The
connection leaf then maps those screen endpoints into its own local space before
recording a local `PathData`.

This keeps traversal and affine endpoint resolution out of `paint_leaf`, while
preserving the existing center-anchor behavior. It intentionally does not do
shape-aware or side-aware port anchoring.

## Workspace Camera

Camera semantics remain:

```text
screen_to_canvas(s) = (s - camera_offset) / zoom
canvas_to_screen(c) = c * zoom + camera_offset
```

`zoom_at` preserves the canvas point under the cursor, including min/max clamp
cases. Workspace view maps the camera to:

```text
Transform { translate: [camera.x, camera.y], scale: camera.zoom, rotate: 0.0 }
```

## Verification

Logic tests cover:

```text
connection endpoint lookup under affine parent transform
connection path endpoints in connection-local space
pending connection cursor mapping under transformed canvas space
CustomPaintCx transform and screen_bounds under affine parent transform
Camera screen/canvas roundtrip after pan and zoom
Camera pan semantic equivalence
Camera zoom clamp cursor stability
workspace canvas_root transform equals camera state
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
