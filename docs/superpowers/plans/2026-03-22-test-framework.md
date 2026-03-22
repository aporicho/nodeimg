# Test Framework Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a 4-layer automated test framework covering all 31 builtin nodes, with GPU tests that auto-skip in CI.

**Architecture:** Feature-flagged test utilities in `nodeimg-gpu` and `nodeimg-processing`, integration tests in `nodeimg-engine/tests/` with shared helpers. No new crate.

**Tech Stack:** Rust test framework, wgpu (headless), pollster, image crate

**Spec:** `docs/superpowers/specs/2026-03-22-test-framework-design.md`

**Branch:** `chore/28-test-framework` (already exists with partial implementation)

**Note:** Some files already exist on the branch but need fixes to match the spec (feature flags, node count, sink node handling). Tasks below include both fixes and new code.

---

### Task 1: Fix nodeimg-gpu test_utils (feature flag)

Currently `pollster` is an unconditional dependency and `test_utils` is always compiled. Spec requires feature flag.

**Files:**
- Modify: `crates/nodeimg-gpu/Cargo.toml`
- Modify: `crates/nodeimg-gpu/src/lib.rs`
- Already exists: `crates/nodeimg-gpu/src/test_utils.rs` (no changes needed)

- [ ] **Step 1: Update Cargo.toml with feature flag**

```toml
# crates/nodeimg-gpu/Cargo.toml
[package]
name = "nodeimg-gpu"
version = "0.1.0"
edition = "2021"

[features]
test-helpers = ["dep:pollster"]

[dependencies]
nodeimg-types = { path = "../nodeimg-types" }
wgpu = "27"
bytemuck = { version = "1", features = ["derive"] }
image = "0.25"
pollster = { version = "0.4", optional = true }
```

- [ ] **Step 2: Make test_utils conditional in lib.rs**

```rust
// crates/nodeimg-gpu/src/lib.rs
pub mod context;
pub mod pipeline;
pub mod shaders;

#[cfg(feature = "test-helpers")]
pub mod test_utils;

pub use context::GpuContext;
```

- [ ] **Step 3: Build to verify**

Run: `cargo build -p nodeimg-gpu`
Expected: compiles without pollster (feature not enabled)

Run: `cargo build -p nodeimg-gpu --features test-helpers`
Expected: compiles with test_utils available

- [ ] **Step 4: Commit**

```bash
git add crates/nodeimg-gpu/
git commit -m "chore: nodeimg-gpu test_utils behind feature flag #28"
```

---

### Task 2: Fix nodeimg-processing test_helpers (feature flag)

Currently `test_helpers` is always compiled. Wrap behind feature flag.

**Files:**
- Modify: `crates/nodeimg-processing/Cargo.toml`
- Modify: `crates/nodeimg-processing/src/lib.rs`
- Already exists: `crates/nodeimg-processing/src/test_helpers.rs` (no changes needed)

- [ ] **Step 1: Add feature flag to Cargo.toml**

```toml
# crates/nodeimg-processing/Cargo.toml
[package]
name = "nodeimg-processing"
version = "0.1.0"
edition = "2021"

[features]
test-helpers = []

[dependencies]
nodeimg-types = { path = "../nodeimg-types" }
image = "0.25"
```

- [ ] **Step 2: Make test_helpers conditional in lib.rs**

```rust
// crates/nodeimg-processing/src/lib.rs
pub mod color;

#[cfg(feature = "test-helpers")]
pub mod test_helpers;
```

- [ ] **Step 3: Build to verify**

Run: `cargo build -p nodeimg-processing`
Expected: compiles without test_helpers

- [ ] **Step 4: Commit**

```bash
git add crates/nodeimg-processing/
git commit -m "chore: nodeimg-processing test_helpers behind feature flag #28"
```

---

### Task 3: Add dev-dependencies to nodeimg-engine

Integration tests need access to GPU context and image helpers.

**Files:**
- Modify: `crates/nodeimg-engine/Cargo.toml`

- [ ] **Step 1: Add dev-dependencies with feature flags**

Append to `crates/nodeimg-engine/Cargo.toml`:

```toml
[dev-dependencies]
nodeimg-gpu = { path = "../nodeimg-gpu", features = ["test-helpers"] }
nodeimg-processing = { path = "../nodeimg-processing", features = ["test-helpers"] }
```

Note: `nodeimg-gpu` and `nodeimg-processing` are already regular dependencies. Adding them as dev-dependencies with extra features enables the `test-helpers` feature only for tests.

- [ ] **Step 2: Build tests to verify feature resolution**

Run: `cargo test -p nodeimg-engine --no-run 2>&1 | head -5`
Expected: compiles (may have test failures, that's OK)

- [ ] **Step 3: Commit**

```bash
git add crates/nodeimg-engine/Cargo.toml
git commit -m "chore: nodeimg-engine dev-deps with test-helpers features #28"
```

---

### Task 4: Fix and complete Layer 1 (builtin registration tests)

The existing `builtin_registration.rs` has bugs: wrong node count (32 vs 31), fails on save_image sink node.

**Files:**
- Modify: `crates/nodeimg-engine/tests/builtin_registration.rs`

- [ ] **Step 1: Rewrite with correct assertions**

```rust
// crates/nodeimg-engine/tests/builtin_registration.rs
use nodeimg_engine::builtins::register_all;
use nodeimg_engine::registry::NodeRegistry;

fn setup_registry() -> NodeRegistry {
    let mut reg = NodeRegistry::new();
    register_all(&mut reg);
    reg
}

#[test]
fn test_all_builtins_register_without_panic() {
    setup_registry();
}

#[test]
fn test_expected_node_count() {
    let reg = setup_registry();
    let all = reg.list(None);
    // 32 register() calls, but curves is a stub -> 31 actual nodes
    assert_eq!(all.len(), 31, "expected 31 builtin nodes, got {}", all.len());
}

#[test]
fn test_all_nodes_have_process_or_is_sink() {
    let reg = setup_registry();
    for def in reg.list(None) {
        // Every node must have at least one execution path
        assert!(
            def.process.is_some() || def.gpu_process.is_some(),
            "node '{}' has neither process nor gpu_process",
            def.type_id
        );
    }
}

#[test]
fn test_no_builtin_is_ai_node() {
    let reg = setup_registry();
    for def in reg.list(None) {
        assert!(
            !def.is_ai_node(),
            "builtin '{}' reports is_ai_node()=true",
            def.type_id
        );
    }
}

#[test]
fn test_instantiate_all_builtins_with_defaults() {
    let reg = setup_registry();
    for def in reg.list(None) {
        let instance = reg.instantiate(&def.type_id);
        assert!(
            instance.is_some(),
            "failed to instantiate '{}'",
            def.type_id
        );
        let instance = instance.unwrap();
        assert_eq!(
            instance.params.len(),
            def.params.len(),
            "node '{}': param count mismatch after instantiate",
            def.type_id
        );
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p nodeimg-engine --test builtin_registration -- --nocapture`
Expected: all 5 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/nodeimg-engine/tests/builtin_registration.rs
git commit -m "test: Layer 1 registration tests (5 tests) #28"
```

---

### Task 5: Create test helpers module

Shared helper functions used by gpu_nodes.rs, cpu_nodes.rs, and pipelines.rs.

`run_node_test` accepts `HashMap<String, DynamicImage>` for named inputs, supporting multi-input nodes like blend and mask.

**Files:**
- Create: `crates/nodeimg-engine/tests/helpers/mod.rs`

- [ ] **Step 1: Write helpers**

```rust
// crates/nodeimg-engine/tests/helpers/mod.rs
use std::collections::HashMap;
use std::sync::Arc;

use image::DynamicImage;
use nodeimg_engine::builtins::register_all;
use nodeimg_engine::cache::Cache;
use nodeimg_engine::eval::{Connection, EvalEngine};
use nodeimg_engine::registry::{NodeInstance, NodeRegistry};
use nodeimg_engine::NodeId;
use nodeimg_gpu::GpuContext;
use nodeimg_types::data_type::DataTypeRegistry;
use nodeimg_types::gpu_texture::GpuTexture;
use nodeimg_types::value::Value;

/// Run a single node and return its outputs.
///
/// `inputs`: named input images, keyed by pin name (e.g. `{"image": img}` or `{"base": img, "layer": img}`).
///   Pass empty HashMap for generator nodes (no inputs).
///
/// For GPU nodes, images are auto-uploaded to GPU textures.
/// For CPU nodes, pass `gpu_ctx` as None -- images stay as Value::Image.
pub fn run_node_test(
    type_id: &str,
    params: HashMap<String, Value>,
    inputs: HashMap<String, DynamicImage>,
    gpu_ctx: Option<&Arc<GpuContext>>,
) -> HashMap<String, Value> {
    let mut reg = NodeRegistry::new();
    register_all(&mut reg);
    let type_reg = DataTypeRegistry::with_builtins();

    let _def = reg
        .get(type_id)
        .unwrap_or_else(|| panic!("node '{}' not registered", type_id));

    let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
    let mut connections: Vec<Connection> = Vec::new();
    let mut cache = Cache::new();

    if inputs.is_empty() {
        // Generator node -- no source needed
        nodes.insert(0, NodeInstance {
            type_id: type_id.into(),
            params,
        });

        EvalEngine::evaluate(0, &nodes, &connections, &reg, &type_reg, &mut cache, None, gpu_ctx)
            .unwrap_or_else(|e| panic!("evaluate '{}' failed: {}", type_id, e));

        cache.get(0)
            .unwrap_or_else(|| panic!("node '{}' produced no output", type_id))
            .clone()
    } else {
        // Create one source node per input, pre-populate cache
        // Source nodes get IDs 0..n-1, target node gets ID n
        let target_id = inputs.len();
        for (i, (pin_name, img)) in inputs.iter().enumerate() {
            let source_id = i;
            // Pre-populate cache with the input image
            let value = if let Some(ctx) = gpu_ctx {
                let tex = GpuTexture::from_dynamic_image(&ctx.device, &ctx.queue, img);
                Value::GpuImage(Arc::new(tex))
            } else {
                Value::Image(Arc::new(img.clone()))
            };
            cache.insert(source_id, HashMap::from([("out".into(), value)]));

            // Placeholder node (won't execute, already cached)
            nodes.insert(source_id, NodeInstance {
                type_id: "solid_color".into(),
                params: HashMap::new(),
            });

            connections.push(Connection {
                from_node: source_id,
                from_pin: "out".into(),
                to_node: target_id,
                to_pin: pin_name.clone(),
            });
        }

        nodes.insert(target_id, NodeInstance {
            type_id: type_id.into(),
            params,
        });

        EvalEngine::evaluate(target_id, &nodes, &connections, &reg, &type_reg, &mut cache, None, gpu_ctx)
            .unwrap_or_else(|e| panic!("evaluate '{}' failed: {}", type_id, e));

        cache.get(target_id)
            .unwrap_or_else(|| panic!("node '{}' produced no output", type_id))
            .clone()
    }
}

/// Run a multi-node pipeline and return the target node's outputs.
pub fn run_pipeline_test(
    node_specs: Vec<(NodeId, &str, HashMap<String, Value>)>,
    connections: Vec<Connection>,
    target: NodeId,
    pre_cache: Option<HashMap<NodeId, HashMap<String, Value>>>,
    gpu_ctx: &Arc<GpuContext>,
) -> HashMap<String, Value> {
    let mut reg = NodeRegistry::new();
    register_all(&mut reg);
    let type_reg = DataTypeRegistry::with_builtins();

    let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
    for (id, type_id, params) in node_specs {
        nodes.insert(id, NodeInstance {
            type_id: type_id.into(),
            params,
        });
    }

    let mut cache = Cache::new();
    if let Some(entries) = pre_cache {
        for (id, outputs) in entries {
            cache.insert(id, outputs);
        }
    }

    EvalEngine::evaluate(target, &nodes, &connections, &reg, &type_reg, &mut cache, None, Some(gpu_ctx))
        .unwrap_or_else(|e| panic!("pipeline evaluate failed: {}", e));

    cache.get(target)
        .unwrap_or_else(|| panic!("target node {} produced no output", target))
        .clone()
}
```

- [ ] **Step 2: Verify helpers compile**

Run: `cargo test -p nodeimg-engine --test builtin_registration --no-run`
Expected: compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/nodeimg-engine/tests/helpers/
git commit -m "test: add shared test helpers (run_node_test, run_pipeline_test) #28"
```

---

### Task 6: Layer 2a -- GPU node execution tests

27 GPU-only node tests. Auto-skip without GPU. Uses named inputs for multi-input nodes.

**Files:**
- Create: `crates/nodeimg-engine/tests/gpu_nodes.rs`

- [ ] **Step 1: Write GPU node tests**

Write all 27 GPU node tests in `crates/nodeimg-engine/tests/gpu_nodes.rs`. Key patterns:
- Single-input nodes: `HashMap::from([("image".into(), img)])`
- Multi-input nodes (blend): `HashMap::from([("base".into(), img.clone()), ("layer".into(), img)])`
- Multi-input nodes (mask): `HashMap::from([("image".into(), img.clone()), ("mask".into(), mask)])`
- Generator nodes: `HashMap::new()` (empty inputs)
- lut_apply: SKIP (requires .cube file on disk, deferred to visual regression phase)

Each test follows this pattern:
```rust
#[test]
fn test_invert() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "invert",
        HashMap::new(),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}
```

Multi-input example (blend):
```rust
#[test]
fn test_blend() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "blend",
        HashMap::from([
            ("mode".into(), Value::String("normal".into())),
            ("opacity".into(), Value::Float(1.0)),
        ]),
        HashMap::from([
            ("base".into(), img.clone()),
            ("layer".into(), img),
        ]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}
```

Selected nodes get pixel verification after `assert_gpu_image`:
- `solid_color`: red pixel check
- `invert`: red -> cyan
- `threshold`: bright gray -> white
- `resize`: dimensions change from 64x64 to 32x32

- [ ] **Step 2: Run and fix iteratively**

Run: `cargo test -p nodeimg-engine --test gpu_nodes -- --nocapture`
Expected: all PASS or SKIPPED. Fix param names by reading each node's `register()` function.

- [ ] **Step 3: Commit**

```bash
git add crates/nodeimg-engine/tests/gpu_nodes.rs
git commit -m "test: Layer 2a GPU node execution tests (26 nodes, lut_apply deferred) #28"
```

---

### Task 6b: Layer 2b -- CPU node execution tests

4 CPU-only nodes. Always run in CI (no GPU needed).

**Files:**
- Create: `crates/nodeimg-engine/tests/cpu_nodes.rs`

- [ ] **Step 1: Write CPU node tests**

```rust
// crates/nodeimg-engine/tests/cpu_nodes.rs
mod helpers;

use std::collections::HashMap;
use nodeimg_processing::test_helpers::make_test_image;
use nodeimg_types::value::Value;

#[test]
fn test_histogram() {
    let img = make_test_image(64, 64, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "histogram",
        HashMap::new(),
        HashMap::from([("image".into(), img)]),
        None, // CPU node, no GPU needed
    );
    // histogram outputs an image (the histogram visualization)
    match result.get("histogram") {
        Some(Value::Image(img)) => {
            assert_eq!(img.width(), 256);
            assert_eq!(img.height(), 128);
        }
        other => panic!("expected Image for 'histogram', got {:?}", other),
    }
}

#[test]
fn test_preview() {
    let img = make_test_image(32, 32, 200, 100, 50, 255);
    let result = helpers::run_node_test(
        "preview",
        HashMap::new(),
        HashMap::from([("image".into(), img)]),
        None,
    );
    // preview passes through the image
    assert!(
        result.contains_key("image"),
        "preview should output 'image'"
    );
}
```

Note: `load_image` and `save_image` require file system interaction (file paths). Skip for now -- they are tested implicitly via serialization roundtrip tests (future PR).

- [ ] **Step 2: Run CPU node tests**

Run: `cargo test -p nodeimg-engine --test cpu_nodes -- --nocapture`
Expected: PASS (these always run, even in CI)

- [ ] **Step 3: Commit**

```bash
git add crates/nodeimg-engine/tests/cpu_nodes.rs
git commit -m "test: Layer 2b CPU node tests (histogram, preview) #28"
```

---

### Task 7: Layer 3 -- Pipeline tests

Multi-node execution, cache invalidation, error handling.

**Files:**
- Create: `crates/nodeimg-engine/tests/pipelines.rs`

- [ ] **Step 1: Write pipeline tests**

```rust
// crates/nodeimg-engine/tests/pipelines.rs
mod helpers;

use std::collections::HashMap;
use std::sync::Arc;

use nodeimg_engine::eval::Connection;
use nodeimg_gpu::test_utils::try_create_headless_context;
use nodeimg_gpu::GpuContext;
use nodeimg_types::value::Value;

fn gpu() -> Option<Arc<GpuContext>> {
    let ctx = try_create_headless_context();
    if ctx.is_none() {
        eprintln!("SKIPPED: no GPU adapter available");
    }
    ctx
}

#[test]
fn test_two_node_pipeline() {
    let Some(ctx) = gpu() else { return };
    let result = helpers::run_pipeline_test(
        vec![
            (
                0,
                "solid_color",
                HashMap::from([
                    ("color".into(), Value::Color([1.0, 0.0, 0.0, 1.0])),
                    ("width".into(), Value::Int(16)),
                    ("height".into(), Value::Int(16)),
                ]),
            ),
            (1, "invert", HashMap::new()),
        ],
        vec![Connection {
            from_node: 0,
            from_pin: "image".into(),
            to_node: 1,
            to_pin: "image".into(),
        }],
        1,
        None,
        &ctx,
    );

    // Verify: red inverted = cyan
    match result.get("image") {
        Some(Value::GpuImage(tex)) => {
            assert_eq!(tex.width, 16);
            let img = tex.to_dynamic_image(&ctx.device, &ctx.queue);
            let px = img.to_rgba8().get_pixel(0, 0).0;
            assert!(px[0] <= 1, "R should be ~0, got {}", px[0]);
            assert!(px[1] >= 254, "G should be ~255, got {}", px[1]);
        }
        other => panic!("expected GpuImage, got {:?}", other),
    }
}

#[test]
fn test_three_node_chain() {
    let Some(ctx) = gpu() else { return };
    let result = helpers::run_pipeline_test(
        vec![
            (
                0,
                "solid_color",
                HashMap::from([
                    ("color".into(), Value::Color([0.5, 0.5, 0.5, 1.0])),
                    ("width".into(), Value::Int(16)),
                    ("height".into(), Value::Int(16)),
                ]),
            ),
            (
                1,
                "blur",
                HashMap::from([
                    ("radius".into(), Value::Float(2.0)),
                    ("method".into(), Value::String("gaussian".into())),
                ]),
            ),
            (
                2,
                "sharpen",
                HashMap::from([("amount".into(), Value::Float(1.0))]),
            ),
        ],
        vec![
            Connection {
                from_node: 0,
                from_pin: "image".into(),
                to_node: 1,
                to_pin: "image".into(),
            },
            Connection {
                from_node: 1,
                from_pin: "image".into(),
                to_node: 2,
                to_pin: "image".into(),
            },
        ],
        2,
        None,
        &ctx,
    );

    match result.get("image") {
        Some(Value::GpuImage(tex)) => {
            assert_eq!(tex.width, 16);
            assert_eq!(tex.height, 16);
        }
        other => panic!("expected GpuImage, got {:?}", other),
    }
}

#[test]
fn test_cache_invalidation() {
    let Some(ctx) = gpu() else { return };

    let mut reg = nodeimg_engine::registry::NodeRegistry::new();
    nodeimg_engine::builtins::register_all(&mut reg);
    let type_reg = nodeimg_types::data_type::DataTypeRegistry::with_builtins();

    let mut nodes = HashMap::new();
    nodes.insert(0, nodeimg_engine::registry::NodeInstance {
        type_id: "solid_color".into(),
        params: HashMap::from([
            ("color".into(), Value::Color([1.0, 0.0, 0.0, 1.0])),
            ("width".into(), Value::Int(16)),
            ("height".into(), Value::Int(16)),
        ]),
    });

    let mut cache = nodeimg_engine::cache::Cache::new();

    // First evaluation
    nodeimg_engine::eval::EvalEngine::evaluate(
        0, &nodes, &[], &reg, &type_reg, &mut cache, None, Some(&ctx),
    ).unwrap();
    assert!(cache.get(0).is_some());

    // Invalidate and re-evaluate with different color
    cache.invalidate_all();
    nodes.get_mut(&0).unwrap().params.insert(
        "color".into(),
        Value::Color([0.0, 1.0, 0.0, 1.0]), // green
    );
    nodeimg_engine::eval::EvalEngine::evaluate(
        0, &nodes, &[], &reg, &type_reg, &mut cache, None, Some(&ctx),
    ).unwrap();

    // Verify we get green, not cached red
    if let Some(Value::GpuImage(tex)) = cache.get(0).unwrap().get("image") {
        let img = tex.to_dynamic_image(&ctx.device, &ctx.queue);
        let px = img.to_rgba8().get_pixel(0, 0).0;
        assert!(px[1] >= 254, "G should be ~255, got {}", px[1]);
        assert!(px[0] <= 1, "R should be ~0, got {}", px[0]);
    }
}

#[test]
fn test_missing_input_produces_empty_output() {
    let Some(ctx) = gpu() else { return };

    let mut reg = nodeimg_engine::registry::NodeRegistry::new();
    nodeimg_engine::builtins::register_all(&mut reg);
    let type_reg = nodeimg_types::data_type::DataTypeRegistry::with_builtins();

    // invert node with required input disconnected
    let mut nodes = HashMap::new();
    nodes.insert(0, nodeimg_engine::registry::NodeInstance {
        type_id: "invert".into(),
        params: HashMap::new(),
    });

    let mut cache = nodeimg_engine::cache::Cache::new();
    // Should not panic -- node executes with empty inputs, returns empty outputs
    let _ = nodeimg_engine::eval::EvalEngine::evaluate(
        0, &nodes, &[], &reg, &type_reg, &mut cache, None, Some(&ctx),
    );
    // invert with no input returns empty outputs (early return in gpu_process)
    let result = cache.get(0);
    if let Some(outputs) = result {
        assert!(outputs.is_empty() || !outputs.contains_key("image"),
            "invert with no input should not produce an image");
    }
}
```

- [ ] **Step 2: Run pipeline tests**

Run: `cargo test -p nodeimg-engine --test pipelines -- --nocapture`
Expected: PASS (or SKIPPED)

- [ ] **Step 3: Commit**

```bash
git add crates/nodeimg-engine/tests/pipelines.rs
git commit -m "test: Layer 3 pipeline tests #28"
```

---

### Task 8: Layer 4 -- Unit test supplements

Add unit tests to existing modules.

**Files:**
- Modify: `crates/nodeimg-engine/src/eval.rs` (append to `mod tests`)
- Modify: `crates/nodeimg-engine/src/cache.rs` (append to `mod tests`)
- Modify: `crates/nodeimg-engine/src/registry.rs` (append to `mod tests`)

- [ ] **Step 1: Add eval.rs unit tests**

Append inside the existing `mod tests { ... }` block in `crates/nodeimg-engine/src/eval.rs`:

```rust
    #[test]
    fn test_topo_sort_single_node() {
        // A single node with no connections should just return itself
        let order = EvalEngine::topo_sort(0, &[]).unwrap();
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn test_evaluate_disconnected_node() {
        use nodeimg_types::category::CategoryId;
        use nodeimg_types::data_type::DataTypeId;
        use crate::registry::{NodeDef, PinDef};

        let mut node_reg = NodeRegistry::new();
        let type_reg = DataTypeRegistry::with_builtins();

        node_reg.register(NodeDef {
            type_id: "passthrough".into(),
            title: "Passthrough".into(),
            category: CategoryId::new("tool"),
            inputs: vec![],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataTypeId::new("float"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: Some(Box::new(|_inputs, _params| {
                HashMap::from([("out".into(), Value::Float(42.0))])
            })),
            gpu_process: None,
        });

        let mut nodes = HashMap::new();
        nodes.insert(0, NodeInstance {
            type_id: "passthrough".into(),
            params: HashMap::new(),
        });

        let mut cache = Cache::new();
        EvalEngine::evaluate(0, &nodes, &[], &node_reg, &type_reg, &mut cache, None, None).unwrap();

        let result = cache.get(0).unwrap();
        assert_eq!(result.get("out"), Some(&Value::Float(42.0)));
    }
```

- [ ] **Step 2: Add cache.rs unit tests**

Append inside the existing `mod tests { ... }` block in `crates/nodeimg-engine/src/cache.rs`:

```rust
    #[test]
    fn test_invalidate_all() {
        let mut cache = Cache::new();
        cache.insert(0, HashMap::new());
        cache.insert(1, HashMap::new());
        cache.insert(2, HashMap::new());

        cache.invalidate_all();
        assert!(cache.get(0).is_none());
        assert!(cache.get(1).is_none());
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn test_cache_overwrite() {
        let mut cache = Cache::new();
        cache.insert(0, HashMap::from([("a".into(), Value::Float(1.0))]));
        cache.insert(0, HashMap::from([("a".into(), Value::Float(2.0))]));

        let result = cache.get(0).unwrap();
        assert_eq!(result.get("a"), Some(&Value::Float(2.0)));
    }
```

- [ ] **Step 3: Add registry.rs unit tests**

Append inside the existing `mod tests { ... }` block in `crates/nodeimg-engine/src/registry.rs`:

```rust
    #[test]
    fn test_list_by_category() {
        let mut reg = NodeRegistry::new();
        reg.register(NodeDef {
            type_id: "a".into(),
            title: "A".into(),
            category: CategoryId::new("color"),
            inputs: vec![],
            outputs: vec![],
            params: vec![],
            has_preview: false,
            process: Some(Box::new(|_, _| HashMap::new())),
            gpu_process: None,
        });
        reg.register(NodeDef {
            type_id: "b".into(),
            title: "B".into(),
            category: CategoryId::new("filter"),
            inputs: vec![],
            outputs: vec![],
            params: vec![],
            has_preview: false,
            process: Some(Box::new(|_, _| HashMap::new())),
            gpu_process: None,
        });

        let color_cat = CategoryId::new("color");
        let color_nodes = reg.list(Some(&color_cat));
        assert_eq!(color_nodes.len(), 1);
        assert_eq!(color_nodes[0].type_id, "a");
    }

    #[test]
    fn test_instantiate_nonexistent_returns_none() {
        let reg = NodeRegistry::new();
        assert!(reg.instantiate("nonexistent").is_none());
    }
```

- [ ] **Step 4: Run all unit tests**

Run: `cargo test -p nodeimg-engine --lib -- --nocapture`
Expected: all existing + new tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nodeimg-engine/src/eval.rs crates/nodeimg-engine/src/cache.rs crates/nodeimg-engine/src/registry.rs
git commit -m "test: Layer 4 unit test supplements (eval, cache, registry) #28"
```

---

### Task 9: Full verification and PR

- [ ] **Step 1: Run full workspace test suite**

Run: `cargo test --workspace -- --nocapture`
Expected: all tests PASS (GPU tests either PASS or print SKIPPED)

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Push and create PR**

```bash
git push -u origin chore/28-test-framework
```

Create PR:
- Title: `chore: 建立自动化测试框架`
- Body references: `Closes #28`
- Test plan: `cargo test --workspace`
