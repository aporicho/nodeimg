# GUI API Boundaries

The app crate must treat `gui` as a framework with explicit facade modules.
Do not reach into implementation modules to solve feature work.

## Stable App-Facing Entrypoints

- `gui::context::Context` and its capability APIs:
  `scene()`, `query()`, `input()`, `rendering()`, `canvas()`,
  `canvas_mut()`, `panel()`, `panel_mut()`, `controls()`,
  `controls_mut()`, `overlay()`, `overlay_mut()`, and `resources_mut()`.
- `gui::scene` for scene mutation types such as `SceneMutation`,
  `MutationError`, and `StylePatch`.
- `gui::layout` for layout/style value types shared with app code.
- `gui::control` for control-facing value types such as `ControlRole`,
  `ControlIntrinsic`, `ParamControlSpec`, `ParamControlMap`, and `ResizeEdge`.
- Public domain modules that are intentionally part of the framework surface:
  `gui::canvas`, `gui::panel`, `gui::renderer`, `gui::shell`,
  `gui::theme`, `gui::template`, `gui::output`, and diagnostics modules.

## Forbidden App Imports

App code must not import these GUI internals:

- `gui::tree`
- `gui::ui`
- `gui::runtime`
- `gui::event`
- `gui::text`
- `gui::widget`
- `gui::control::state`
- `gui::control::systems`
- `gui::control::painter`

If app code needs a value currently hidden behind one of those paths, expose it
through a facade first. Prefer `Context` capability APIs for behavior, and
`gui::scene`, `gui::layout`, or `gui::control` for data types.

## Template And Legacy Boundaries

`gui::template` exposes template IDs, slots, payloads, and registry operations
needed by retained scene code. Compiled template internals and generated helper
constructors stay crate-private.

Full-tree legacy `Desc` reconciliation and `WidgetProps::build()` are removed.
Production code and tests must use retained templates plus `TreeMutation`
through scene controllers.

Semantic roles are typed with `ControlRole`; string semantic roles are not part
of the API.

Control intrinsics are keyed by `ControlIntrinsic::control_id`. Parameter
control internals use the `::content` stable-id segment; the removed `::widget`
segment must not be reintroduced.

## Automated Gate

Run this before pushing GUI boundary changes:

```bash
scripts/check_gui_api_boundaries.sh
```

CI runs the same gate on Linux. The gate checks app imports, public template
internals, legacy facade reintroduction, string semantic roles, old direct
`Context` calls that bypass capability APIs, and full-tree legacy `Desc`
reconcile paths.
