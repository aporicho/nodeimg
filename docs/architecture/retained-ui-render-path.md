# Retained UI Render Path

Date: 2026-04-28

## Production Shape

The retained UI path has one ownership chain:

```text
app/workspace scene state
  -> scene diff
  -> TreeMutation
  -> Tree live instance tree
  -> dirty layout / paint / renderer caches
```

Static UI structure is registered as `CompiledTemplate` through `TemplateRegistry`.
Runtime state is patched through `TreeMutation` and retained runtime stores. New
production code must not create a `Desc` tree as an intermediate representation.

## Module Boundaries

- `gui/src/template`: compiled/static structure only. It owns template ids,
  revisions, slot schema, default node styles, stable-id suffixes, and boundary
  declarations. It does not store focus, hover, cursor, selection, scroll,
  layout measurement, paint fragments, renderer resources, or live `NodeId`s.
- `gui/src/tree`: live retained instance tree, index, mutation, dirty queues,
  retained runtime slots, and frame counters. Tree modifications from retained
  production code go through `TreeMutation`.
- `gui/src/widget/state`: control runtime state only. Text editor state is keyed
  by retained ids and emits mutations for tree-visible text changes.
- `gui/src/canvas`: scene model and scene diff. It converts canvas semantic
  changes into template mounts, unmounts, and mutations. It does not call engine
  internals or widget builders.
- `app/src/workspace`: maps engine/workspace snapshots into deterministic scene
  changes. Legacy full-tree view builders remain isolated until the workspace
  shell is fully retained.
- `gui/src/renderer`: backend command preparation and resource caches. Backend
  resource handles must not leak back into tree/template/widget modules.

## Compiled Template Contract

`CompiledTemplate` contains static node structure, default styles, stable-id
suffixes, slot schema, and declared relayout/repaint boundaries. The registry is
the only retained production instantiation entry:

```rust
registry.instantiate(tree, template_id, instance_id, parent, slot_values)
```

`SlotValues` may initialize text, rect, style, visibility, and z-index values at
mount time. Later changes are `TreeMutation` patches against live nodes. A
template revision change is treated as an incompatible registration in dev/test
so accidental parallel definitions fail early.

## Dirty Data Flow

- `SetRect` with position-only changes marks composite and hit.
- `SetRect` with size changes marks layout, hit, and paint.
- `SetText` marks text layout and paint for text leaves.
- `SetStyle` classifies style, layout, paint-order, hit, paint, and composite
  invalidation from explicit patch fields.
- `MountTemplate` and `Unmount` mark structure, layout, hit, and paint.

Dirty queues are the boundary between semantic updates and flushing. Layout,
paint, and render caches consume these queues; they do not rebuild the full tree
unless the legacy `Desc` path is explicitly used.

## Text Area Model

Text editor runtime owns cursor, selection, and edited text. The tree stores only
visible text leaves and layout-relevant static structure. Same-height edits mark
text layout and paint for the editor boundary. Height-changing edits additionally
mark the nearest relayout boundary.

## Panel And Node Resize

Panels and canvas node cards use the same retained container capabilities:
hittable roots, drag gestures, resize gestures, z-index patches, and rect
mutations. Resize is represented as `SetRect`; internal node/card content is not
rebuilt for a pure outer rect change.

## Legacy Boundaries

These APIs are legacy/prototype boundaries and are not retained production
entrypoints:

| API | Current owner | Deletion condition |
| --- | --- | --- |
| `Context::update(Desc, ...)` | UI engine migration | App shell composes root through retained scene mutations. |
| `WidgetProps::build()` | Legacy widget desc path | All controls used by production workspace have compiled templates or retained atoms. |
| `app::workspace::view::build_workspace_tree` | Workspace migration | `WorkspaceSceneController` drives canvas, panels, and overlays without `Desc`. |
| `gui::canvas::legacy_desc::node_card_from_render_view` | Canvas migration | Node cards are mounted and patched through `CanvasSceneModel`. |
| `tree::build_display_list` full-root calls | Paint migration | Retained paint fragment flushing covers all production paint paths. |
| `control_intrinsics` full scan | Text editor migration | Text runtime dirty intrinsic queue is the only production source. |

`app/build.rs` currently generates panel declaration collection code only. It is
not a second template/codegen path; when panels move to retained templates, that
generator must emit or register `CompiledTemplate` definitions through
`TemplateRegistry`.
