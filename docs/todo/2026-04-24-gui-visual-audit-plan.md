# GUI Visual Audit Plan

Date: 2026-04-24

## Goal

Lock the GUI rendering stack by auditing visible output from the lowest
primitive layer up to real workspace scenes:

```text
primitive graphics
-> basic widgets
-> framework widgets and panels
-> canvas node cards
-> workspace scenes
```

The purpose is not to redesign the UI. The purpose is to prove that every
declared primitive, control, panel, and node-card state renders correctly through
the current tree -> paint DisplayList -> renderer path, and to fix defects at
the lowest responsible layer.

## Rules

- Fix primitive defects in `paint`, `tree::paint`, `renderer`, or `theme`, not in
  one-off widget call sites.
- Fix repeated control defects in the widget implementation, not in a specific
  panel or page.
- Fix node-card defects in `canvas::node_card`, metrics, or node-card theme
  helpers, not in workspace-specific adapters.
- Do not add visual-only compatibility paths that bypass `paint::DisplayList`.
- Every manual audit finding must become either a code fix, a test case, or an
  explicit deferred enhancement with owner and reason.
- Keep screenshots and notes organized by layer so regressions can be compared
  later.

## Audit Matrix

| Status | Layer | Scope | Notes |
| --- | --- | --- | --- |
| TODO | Primitive graphics | Rect, rounded rect, border, shadow | Verify fill, border width, per-corner radius, shadow offset/blur/spread, opacity, clip interaction. |
| TODO | Primitive graphics | Text | Verify font size, weight, line height, clipping, ellipsis, alignment, selection/caret overlays. |
| TODO | Primitive graphics | Image | Verify fit modes, source rect, tint, opacity, filter, transformed rendering. |
| TODO | Primitive graphics | SVG/Icon | Verify common built-in icons, vector path rendering, stroke/fill overrides, raster fallback behavior. |
| TODO | Primitive graphics | Circle, line, curve, path | Verify fill/stroke, caps/joins, miter, affine transform, hit shape alignment. |
| TODO | Primitive graphics | Grid, connection, pending connection | Verify camera zoom/pan, endpoint resolution, stroke scaling, hit shape alignment. |
| TODO | Primitive graphics | Clip, layer, opacity, transform | Verify nested clips, rounded clips, rotated/scaled content, flattened layer opacity. |
| TODO | Basic widgets | Button, icon button | Verify all sizes, density, hover/pressed/disabled, leading icon, text fit. |
| TODO | Basic widgets | Checkbox, radio, toggle | Verify checked/unchecked/disabled/hover/pressed states and label alignment. |
| TODO | Basic widgets | Slider | Verify track, fill, thumb, drag state, min/max, disabled state. |
| TODO | Basic widgets | Dropdown | Verify field, chevron, selected option, popup placement, highlighted option. |
| TODO | Basic widgets | TextInput, NumberInput | Verify focus, caret, selection, scroll, invalid state, formatting. |
| TODO | Basic widgets | Label, truncated text, separator | Verify typography variants, ellipsis, alignment, density. |
| TODO | Basic widgets | ColorSwatch, ImageViewer, PathInput | Verify media and value presentation, focus/disabled states. |
| TODO | Framework widgets | Panel | Verify titlebar, resize handles, drag area, close button, focused/unfocused states. |
| TODO | Framework widgets | ScrollArea, ListView | Verify clipping, scroll offset, content bounds, nested controls. |
| TODO | Framework widgets | Group, Collapsible | Verify header, content spacing, collapse icon, collapsed layout. |
| TODO | Overlay systems | Overlay, popup, dropdown menu | Verify placement, dismiss behavior, layering, clipping, keyboard selection. |
| TODO | Canvas node card | Shell and header | Verify card radius, title, category color, icon/dot, selected/hovered/pressed states. |
| TODO | Canvas node card | Port rows and port groups | Verify side order, port dots, labels, group trigger, expanded/collapsed layout. |
| TODO | Canvas node card | Param rows | Verify sliders, toggles, numbers, colors, text inputs inside node cards. |
| TODO | Canvas graph | Connections | Verify normal, hovered, pending, invalid target, zoomed, and transformed states. |
| TODO | Workspace scenes | Initial workspace | Verify default panels, canvas, node library, inspector, status surfaces. |
| TODO | Workspace scenes | Editing workflows | Verify add node, drag node, connect ports, edit params, open overlay, panel resize/scroll. |

## Execution Order

1. Build or extend a deterministic visual audit developer page for each layer.
2. Audit primitive graphics first, because all higher layers depend on them.
3. Audit basic widgets with all meaningful states.
4. Audit framework widgets and panels.
5. Audit canvas node cards and graph interactions.
6. Audit real workspace workflows.
7. For every defect, decide the lowest responsible module before editing.
8. After fixes, rerun the relevant visual scene and the logic test suite.

## Suggested Test Fixtures

Prefer extending existing fixtures before adding new systems:

- `app/src/visual_audit/` for primitive and widget visual scenes.
- `app/src/workspace/showcase_node.rs` for node-card param/control coverage.
- `app/src/workspace/view.rs` for full workspace composition checks.
- `gui/src/tree/paint` recording tests for low-level primitive assertions.
- `gui/src/tree/hit` and `gui/src/tree/hit_shape` tests for paint/hit alignment.

## Progress Log

- 2026-04-24: Added a `Visual Audit` developer page backed by
  `app/src/visual_audit/`. The first deterministic primitive fixture covers
  rect/radius/shadow, text clip/ellipsis/alignment, image fit/tint/source rect,
  SVG icon, circle, line, curve, path, grid, connection, and transform samples.
  Manual visual review is still required before marking the primitive rows DONE.

## Acceptance

This audit is complete when:

- Every row in the audit matrix is manually checked and marked DONE.
- Every defect found during audit is either fixed or explicitly documented as a
  future enhancement.
- No fix introduces a one-off rendering path outside `paint::DisplayList`.
- `cargo fmt --check`, `cargo test -p gui`, `cargo test -p app`,
  `cargo check --workspace`, and `cargo clippy -p gui` pass after the final fix.
- The final notes identify the stable API path for building UI without touching
  renderer internals.
