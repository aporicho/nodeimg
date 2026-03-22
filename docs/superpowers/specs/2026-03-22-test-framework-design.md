# Test Framework Design

Issue: #28

## Problem

48 unit tests, zero integration tests. 31 builtin nodes untested (curves is a stub that registers nothing). CI runs `cargo test` on ubuntu (no GPU). No safety net for refactoring.

## Goals

- Full coverage: every builtin node has an execution test
- CI gate: PR must pass tests to merge
- Low cost: helper functions make writing node tests 5-10 lines each
- GPU graceful: tests auto-skip without GPU, full run with GPU

## Architecture

Considered three approaches:
- **A. Dedicated `nodeimg-test-utils` crate** -- clean isolation but adds a workspace member for ~50 lines of code
- **B. Public modules in existing crates** -- zero new crate, tools near usage, dependencies already exist
- **C. Pure `#[cfg(test)]`** -- zero API pollution but cross-crate tools impossible

**Chose B.** The test utilities are too small to justify a new crate, and `#[cfg(test)]` modules can't be imported across crate boundaries.

To avoid shipping test code in production, use **Cargo feature flags**: `pollster` and test_utils are behind a `test-helpers` feature, enabled only in `[dev-dependencies]`.

### Test Utilities

**`nodeimg-gpu::test_utils`** (behind `test-helpers` feature)
- `try_create_headless_context() -> Option<Arc<GpuContext>>` via pollster + wgpu headless adapter
- Returns None when no GPU (CI auto-skip)

```toml
# nodeimg-gpu/Cargo.toml
[features]
test-helpers = ["dep:pollster"]

[dependencies]
pollster = { version = "0.4", optional = true }
```

```toml
# nodeimg-engine/Cargo.toml
[dev-dependencies]
nodeimg-gpu = { path = "../nodeimg-gpu", features = ["test-helpers"] }
```

**`nodeimg-processing::test_helpers`** (behind `test-helpers` feature)
- `make_test_image(w, h, r, g, b, a) -> DynamicImage`
- `max_pixel_diff(a, b) -> Result<u8, String>`
- `assert_images_equal(a, b)`
- `assert_images_similar(a, b, tolerance)`

**`nodeimg-engine/tests/helpers/mod.rs`** (test-only, not shipped)
- `run_node_test(type_id, params, input, gpu_ctx) -> HashMap<String, Value>`
  - Internally: create NodeRegistry + register_all, create DataTypeRegistry, build single-node graph, call EvalEngine::evaluate, return cache output
- `run_pipeline_test(nodes_with_params, connections, target, gpu_ctx) -> HashMap<String, Value>`
  - Same but for multi-node graphs

### GpuImage Assertion Pattern

```rust
// Standard pattern for checking GPU node output
match result.get("image") {
    Some(Value::GpuImage(tex)) => {
        assert_eq!(tex.width, expected_w);
        assert_eq!(tex.height, expected_h);
    }
    other => panic!("expected GpuImage, got {:?}", other),
}
```

## Test Layers

### Layer 1: Registration (no GPU, CI must pass)

File: `crates/nodeimg-engine/tests/builtin_registration.rs`

| Test | Validates |
|------|-----------|
| `test_all_builtins_register_without_panic` | register_all() safe |
| `test_expected_node_count` | 31 nodes registered (curves is stub) |
| `test_all_nodes_have_process_or_is_sink` | every node has process/gpu_process, or is a known sink (save_image) |
| `test_no_builtin_is_ai_node` | all have process or gpu_process |
| `test_instantiate_all_builtins_with_defaults` | instantiation + param count |

Notes:
- save_image is a sink node (no outputs, no preview, CPU-only process). The test accounts for this.
- 4 nodes are CPU-only (load_image, preview, histogram, save_image), the other 27 are GPU-only.

### Layer 2: Single Node GPU Execution (auto-skip without GPU)

File: `crates/nodeimg-engine/tests/gpu_nodes.rs`

**27 GPU-only nodes** -- one test each, auto-skip without GPU:
1. Create GPU context via `try_create_headless_context()`, early return if None
2. Call `run_node_test(type_id, params, input_image)`
3. Assert output is `Value::GpuImage` with correct dimensions

Selected GPU nodes get pixel-level verification (readback via `GpuTexture::to_dynamic_image`):
- `invert`: red(1,0,0) -> cyan(0,1,1)
- `solid_color`: verify pixel color matches param
- `resize`: verify output dimensions change
- `flip`/`rotate`: verify pixel positions
- `threshold`: verify binarization

**4 CPU-only nodes** -- tested separately (no GPU needed, always runs in CI):
- `load_image`: load from file path, verify output is `Value::Image`
- `histogram`: input image, verify output dimensions (256x128)
- `preview`: input image, verify pass-through
- `save_image`: sink node, verify execution completes without error

### Layer 3: Pipeline Tests (auto-skip without GPU)

File: `crates/nodeimg-engine/tests/pipelines.rs`

| Test | Pipeline | Validates |
|------|----------|-----------|
| `test_two_node_pipeline` | solid_color -> invert | basic chaining |
| `test_three_node_chain` | solid_color -> blur -> sharpen | longer chain |
| `test_cache_invalidation` | modify param, re-eval | fresh output after change |
| `test_missing_input_error` | node with required input disconnected | error propagation |

### Layer 4: Unit Test Supplements

In existing `#[cfg(test)]` modules:

**eval.rs**: `test_evaluate_disconnected_node`, `test_topo_sort_single_node`

**cache.rs**: `test_invalidate_all`, `test_cache_overwrite`

**registry.rs**: `test_list_by_category`, `test_instantiate_nonexistent_returns_none`

## File Layout

```
crates/nodeimg-gpu/
    Cargo.toml              (add pollster as optional dep, test-helpers feature)
    src/test_utils.rs       (NEW, behind cfg feature)
    src/lib.rs              (add conditional pub mod test_utils)

crates/nodeimg-processing/
    Cargo.toml              (add test-helpers feature)
    src/test_helpers.rs     (NEW, behind cfg feature)
    src/lib.rs              (add conditional pub mod test_helpers)

crates/nodeimg-engine/
    Cargo.toml              (add dev-deps with test-helpers features)
    tests/
        helpers/mod.rs      (NEW - run_node_test, run_pipeline_test)
        builtin_registration.rs  (NEW - Layer 1)
        gpu_nodes.rs        (NEW - Layer 2)
        pipelines.rs        (NEW - Layer 3)
    src/eval.rs             (add 2 unit tests)
    src/cache.rs            (add 2 unit tests)
    src/registry.rs         (add 2 unit tests)
```

## CI Strategy

**This PR**: No CI changes. GPU tests auto-skip on ubuntu. Registration tests always run.

**Future PR**: Add macOS runner job for GPU tests. Add visual regression tests.

## New Node Convention

When adding a builtin node, also add a test in `gpu_nodes.rs`:

```rust
#[test]
fn test_my_new_node() {
    let Some(ctx) = nodeimg_gpu::test_utils::try_create_headless_context() else {
        eprintln!("SKIPPED: no GPU");
        return;
    };
    let input = nodeimg_processing::test_helpers::make_test_image(64, 64, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "my_new_node",
        HashMap::from([("amount".into(), Value::Float(0.5))]),
        Some(&input),
        &ctx,
    );
    match result.get("image") {
        Some(Value::GpuImage(tex)) => {
            assert_eq!(tex.width, 64);
            assert_eq!(tex.height, 64);
        }
        other => panic!("expected GpuImage, got {:?}", other),
    }
}
```

## Deferred (not in this PR)

| Issue #28 requirement | Reason deferred |
|---|---|
| Visual regression tests (reference image comparison) | Needs solved GPU CI + reference image storage strategy |
| GPU/CPU consistency tests | 27 builtins are GPU-only (no CPU path to compare); 4 CPU-only nodes have no GPU path |
| Serialization roundtrip tests | Already covered by existing tests in serial.rs; graph-level roundtrip depends on #57 Transport trait |
| Test fixtures directory (tests/fixtures/) | Not needed until visual regression; current tests use programmatic test images |
| processing/* unit tests | processing crate only has color.rs (already has 6 tests); other algorithms were deleted in #59 GPU-only migration |
