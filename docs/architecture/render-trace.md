# Render Trace Diagnostics

Retained UI render diagnostics use the existing `tracing` subscriber and are
grouped under stable targets:

- `nodeimg::render_trace`: one summary per key CPU stage.
- `nodeimg::render_trace::node`: per-node and high-frequency interaction detail.
- `nodeimg::render_trace::gpu`: renderer preparation, pass planning, uploads, and submit.
- `nodeimg::render_trace::tree`: opt-in retained tree dumps.

Common filters:

```bash
RUST_LOG=nodeimg::render_trace=debug
RUST_LOG=nodeimg::render_trace=debug,nodeimg::render_trace::node=trace
RUST_LOG=nodeimg::render_trace=debug,nodeimg::render_trace::gpu=trace
RUST_LOG=nodeimg::render_trace::tree=debug NODEIMG_TREE_DUMP=once
```

Each redraw receives a `frame_id` in `gui::diagnostics::render_trace`.
The same frame flows through app update, scene sync, mutation, dirty
propagation, layout flush, text runtime sync, paint flush, display-list
lowering, renderer prepare/dispatch, and present.

`AppUpdate` and `SceneSync` summaries include the active workspace composition.
Developer mode defaults to `clean_room`, which keeps the full retained pipeline
but only feeds canvas grid, one diagnostic node, and the retained toolbar panel
into scene sync. User mode uses `full`, which feeds the engine/showcase nodes,
all panels, overlays, and connections.

Logging rules:

- Debug level is for stage summaries and should stay low-volume.
- Trace level is for node, command, resize, hit-test, and pass-level detail.
- Text payloads are redacted to byte/character counts.
- Stable-id lookup must continue to use `TreeIndex`; diagnostics must not add
  full-tree scans.
- Retained diagnostics must not call `Desc`, `WidgetProps::build()`, or
  `reconcile`.

Tree dump controls:

- `NODEIMG_TREE_DUMP=off|once|invalid|dirty|frames`
- `NODEIMG_TREE_DUMP_LEVEL=normal|full`
- `NODEIMG_TREE_DUMP_MAX_NODES=500|all` (`full` always dumps all nodes)
- `NODEIMG_TREE_DUMP_STAGES=SceneSync,LayoutFlush,PaintFlush`
- `NODEIMG_TREE_DUMP_FRAMES=1,2,3`

`normal` prints compact structure, rect, dirty, boundary, revision, and runtime
slot counts. `full` prints every retained-tree field available at runtime,
including raw text and runtime slot `Debug` values, so use it only for focused
bug captures.
