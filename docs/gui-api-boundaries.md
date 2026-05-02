# GUI API Boundaries

The app crate must treat `gui` as a framework with explicit facade modules.
Do not reach into implementation modules to solve feature work.

## Stable App-Facing Entrypoints

- `gui::context::Context` and its capability APIs:
  `scene()`, `query()`, `input()`, `rendering()`, `canvas()`,
  `canvas_mut()`, `panel()`, `panel_mut()`, `runtime()`,
  `runtime_mut()`, `overlay()`, `overlay_mut()`, and `resources_mut()`.
- `gui::scene` for scene mutation types such as `SceneMutation`,
  `MutationError`, and `StylePatch`.
- `gui::layout` for layout/style value types shared with app code.
- `gui::widget` for widget-facing value types such as `ParamControlSpec`,
  `ParamControlMap`, and `ResizeEdge`.
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
- `gui::widget::atoms`
- `gui::widget::frameworks`
- `gui::widget::mapping`
- `gui::widget::props`
- `gui::widget::desc`
- `gui::widget::param_control`
- `gui::widget::resize_edge`
- `gui::widget::state`
- `gui::widget::systems`

If app code needs a value currently hidden behind one of those paths, expose it
through a facade first. Prefer `Context` capability APIs for behavior, and
`gui::scene`, `gui::layout`, or `gui::widget` for data types.

## Template And Legacy Boundaries

`gui::template` exposes template IDs, slots, payloads, and registry operations
needed by retained scene code. Compiled template internals and generated helper
constructors stay crate-private.

Full-tree legacy `Desc` reconciliation is removed. Production code and tests
must use retained templates plus `TreeMutation` through scene controllers.
`Desc` may still exist as a crate-private widget implementation detail until
the remaining widget atoms are migrated to retained templates.

Semantic roles are typed with `WidgetRole`; string semantic roles are not part
of the API.

## Automated Gate

Run this before pushing GUI boundary changes:

```bash
scripts/check_gui_api_boundaries.sh
```

CI runs the same gate on Linux. The gate checks app imports, public template
internals, legacy facade reintroduction, string semantic roles, old direct
`Context` calls that bypass capability APIs, and full-tree legacy `Desc`
reconcile paths.
