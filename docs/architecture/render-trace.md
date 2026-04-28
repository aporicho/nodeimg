# Render Trace Diagnostics

Retained UI render diagnostics use the existing `tracing` subscriber and are
grouped under stable targets:

- `nodeimg::render_trace`: one summary per key CPU stage.
- `nodeimg::render_trace::node`: per-node and high-frequency interaction detail.
- `nodeimg::render_trace::gpu`: renderer preparation, pass planning, uploads, and submit.

Common filters:

```bash
RUST_LOG=nodeimg::render_trace=debug
RUST_LOG=nodeimg::render_trace=debug,nodeimg::render_trace::node=trace
RUST_LOG=nodeimg::render_trace=debug,nodeimg::render_trace::gpu=trace
```

Each redraw receives a `frame_id` in `gui::diagnostics::render_trace`.
The same frame flows through app update, scene sync, mutation, dirty
propagation, layout flush, text runtime sync, paint flush, display-list
lowering, renderer prepare/dispatch, and present.

Logging rules:

- Debug level is for stage summaries and should stay low-volume.
- Trace level is for node, command, resize, hit-test, and pass-level detail.
- Text payloads are redacted to byte/character counts.
- Stable-id lookup must continue to use `TreeIndex`; diagnostics must not add
  full-tree scans.
- Retained diagnostics must not call `Desc`, `WidgetProps::build()`, or
  `reconcile`.
