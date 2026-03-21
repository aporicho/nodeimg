# Rust Frontend SDXL Integration Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add BackendClient to Rust frontend that dynamically registers AI nodes from Python backend and sends graph execution requests.

**Architecture:** BackendClient fetches node type definitions from `GET /node_types` at startup and registers them as "remote" nodes in NodeRegistry (no local ProcessFn). On execution, AI subgraph is serialized to JSON and sent to `POST /execute`. Result image is decoded and passed to Preview/SaveImage nodes.

**Tech Stack:** Rust, eframe/egui 0.33, reqwest (blocking + json), base64, serde_json

**Spec:** `docs/superpowers/specs/2026-03-17-sdxl-nodes-design.md`
**Protocol:** `docs/protocol.md`

---

## Chunk 1: BackendClient and Dynamic Registration

### Task 1: Add reqwest dependency and BackendClient

**Files:**
- Modify: `Cargo.toml`
- Create: `src/node/backend.rs`
- Modify: `src/node/mod.rs`

- [ ] **Step 1: Add dependencies to `Cargo.toml`**

Add:
```toml
reqwest = { version = "0.12", features = ["blocking", "json"] }
base64 = "0.22"
```

- [ ] **Step 2: Create `src/node/backend.rs`**

```rust
use serde_json::Value as JsonValue;

pub struct BackendClient {
    client: reqwest::blocking::Client,
    base_url: String,
}

impl BackendClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(300)) // 5 min for long inference
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn health_check(&self) -> Result<JsonValue, String> {
        self.get("/health")
    }

    pub fn fetch_node_types(&self) -> Result<JsonValue, String> {
        self.get("/node_types")
    }

    pub fn execute_graph(&self, graph: &JsonValue) -> Result<JsonValue, String> {
        self.post("/execute", &serde_json::json!({"graph": graph}))
    }

    fn get(&self, endpoint: &str) -> Result<JsonValue, String> {
        let url = format!("{}{}", self.base_url, endpoint);
        self.client
            .get(&url)
            .send()
            .map_err(|e| format!("HTTP error: {}", e))?
            .json::<JsonValue>()
            .map_err(|e| format!("JSON parse error: {}", e))
    }

    fn post(&self, endpoint: &str, body: &JsonValue) -> Result<JsonValue, String> {
        let url = format!("{}{}", self.base_url, endpoint);
        self.client
            .post(&url)
            .json(body)
            .send()
            .map_err(|e| format!("HTTP error: {}", e))?
            .json::<JsonValue>()
            .map_err(|e| format!("JSON parse error: {}", e))
    }
}
```

- [ ] **Step 3: Add `pub mod backend;` to `src/node/mod.rs`**

- [ ] **Step 4: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/node/backend.rs src/node/mod.rs
git commit -m "feat: add BackendClient for Python backend communication"
```

---

### Task 2: Dynamic AI node registration

**Files:**
- Modify: `src/node/backend.rs`
- Modify: `src/node/category.rs`
- Modify: `src/theme/light.rs`
- Modify: `src/theme/dark.rs`

- [ ] **Step 1: Add "ai" category**

In `src/node/category.rs`, add to `with_builtins()`:
```rust
reg.register(CategoryId::new("ai"), "AI", 80);
```

- [ ] **Step 2: Add AI category color to both themes**

In `src/theme/light.rs` `category_color()`, add:
```rust
"ai" => Color32::from_rgb(220, 50, 90),  // vibrant red-pink
```

In `src/theme/dark.rs` `category_color()`, add:
```rust
"ai" => Color32::from_rgb(230, 70, 100),
```

- [ ] **Step 3: Add dynamic registration function to `backend.rs`**

```rust
use crate::node::registry::{NodeDef, NodeRegistry, PinDef, ParamDef};
use crate::node::types::DataTypeId;
use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;

impl BackendClient {
    /// Fetch node types from backend and register them in the NodeRegistry.
    /// Returns the number of nodes registered, or an error.
    pub fn register_remote_nodes(
        &self,
        node_registry: &mut NodeRegistry,
        type_registry: &mut crate::node::types::DataTypeRegistry,
    ) -> Result<usize, String> {
        let resp = self.fetch_node_types()?;
        let node_types = resp.get("node_types")
            .and_then(|v| v.as_object())
            .ok_or("Invalid node_types response")?;

        // Register AI data types if not already registered
        let ai_types = ["MODEL", "CLIP", "VAE", "CONDITIONING", "LATENT"];
        for t in &ai_types {
            let id = DataTypeId::new(&t.to_lowercase());
            if type_registry.get(&id).is_none() {
                type_registry.register(crate::node::types::DataTypeInfo {
                    id,
                    name: t.to_string(),
                });
            }
        }

        let mut count = 0;
        for (type_id, def_json) in node_types {
            let title = type_id.clone();

            let inputs: Vec<PinDef> = def_json.get("inputs")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|p| {
                    Some(PinDef {
                        name: p.get("name")?.as_str()?.to_string(),
                        data_type: DataTypeId::new(&p.get("type")?.as_str()?.to_lowercase()),
                    })
                }).collect())
                .unwrap_or_default();

            let outputs: Vec<PinDef> = def_json.get("outputs")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|p| {
                    Some(PinDef {
                        name: p.get("name")?.as_str()?.to_string(),
                        data_type: DataTypeId::new(&p.get("type")?.as_str()?.to_lowercase()),
                    })
                }).collect())
                .unwrap_or_default();

            let params: Vec<ParamDef> = def_json.get("params")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|p| {
                    let name = p.get("name")?.as_str()?.to_string();
                    let ptype = p.get("type")?.as_str()?;
                    let (data_type, constraint, default_val) = parse_param(p, ptype);
                    Some(ParamDef {
                        name,
                        data_type,
                        constraint,
                        default_value: default_val,
                        widget_override: None,
                    })
                }).collect())
                .unwrap_or_default();

            node_registry.register(NodeDef {
                type_id: type_id.clone(),
                title,
                category: CategoryId::new("ai"),
                inputs,
                outputs,
                params,
                process: None,  // No local ProcessFn — executed by backend
                has_preview: false,
            });
            count += 1;
        }

        Ok(count)
    }
}

fn parse_param(p: &JsonValue, ptype: &str) -> (DataTypeId, Constraint, Option<crate::node::types::Value>) {
    match ptype {
        "INT" => {
            let min = p.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let max = p.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0);
            let default = p.get("default").and_then(|v| v.as_i64()).map(|v| crate::node::types::Value::Int(v as i32));
            (DataTypeId::new("int"), Constraint::Range { min, max }, default)
        }
        "FLOAT" => {
            let min = p.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let max = p.get("max").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let default = p.get("default").and_then(|v| v.as_f64()).map(|v| crate::node::types::Value::Float(v as f32));
            (DataTypeId::new("float"), Constraint::Range { min, max }, default)
        }
        "ENUM" => {
            let options: Vec<String> = p.get("options")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let default = p.get("default").and_then(|v| v.as_str()).map(|s| crate::node::types::Value::String(s.to_string()));
            (DataTypeId::new("string"), Constraint::Enum(options), default)
        }
        _ => {
            let default = p.get("default").and_then(|v| v.as_str()).map(|s| crate::node::types::Value::String(s.to_string()));
            (DataTypeId::new("string"), Constraint::None, default)
        }
    }
}
```

NOTE: The exact field names of NodeDef, PinDef, ParamDef, Constraint etc. depend on the actual codebase. Read `src/node/registry.rs` and `src/node/constraint.rs` before implementing to match exact types.

- [ ] **Step 4: Verify it compiles**

Run: `cargo build 2>&1 | tail -10`
Expected: May need adjustments to match actual types. Fix any compile errors.

- [ ] **Step 5: Commit**

```bash
git add src/node/backend.rs src/node/category.rs src/theme/light.rs src/theme/dark.rs
git commit -m "feat: dynamic AI node registration from backend /node_types"
```

---

## Chunk 2: Graph Execution Integration

### Task 3: Wire BackendClient into App

**Files:**
- Modify: `src/app.rs`
- Modify: `src/node/viewer.rs`

- [ ] **Step 1: Add BackendClient to App**

In `src/app.rs`, add to App struct:
```rust
backend: Option<crate::node::backend::BackendClient>,
```

In `App::new()`:
```rust
let backend = crate::node::backend::BackendClient::new("http://localhost:8188");
// Try to register remote nodes (non-fatal if backend is offline)
match backend.register_remote_nodes(&mut viewer.node_registry_mut(), &mut viewer.type_registry_mut()) {
    Ok(n) => eprintln!("Registered {} AI nodes from backend", n),
    Err(e) => eprintln!("Backend offline, skipping AI nodes: {}", e),
}
```

NOTE: The exact way to access NodeRegistry/TypeRegistry through NodeViewer depends on current code. May need to add pub accessors.

- [ ] **Step 2: Add graph serialization for AI nodes**

In `src/node/backend.rs`, add a function to serialize AI subgraph from Snarl:

```rust
pub fn serialize_ai_subgraph(
    snarl: &Snarl<NodeInstance>,
    node_registry: &NodeRegistry,
    target_node_id: usize,
) -> Option<JsonValue> {
    // Walk upstream from target, collect all AI nodes (category == "ai")
    // Serialize to protocol.md format
    // Return None if no AI nodes found
    todo!("Implement based on actual Snarl API")
}
```

This function needs to:
1. Walk the graph from the target node upstream
2. Collect all connected AI nodes
3. Serialize them to the JSON format defined in protocol.md
4. Return the graph JSON

- [ ] **Step 3: Add execution trigger in evaluation**

When EvalEngine encounters an AI node (one without a ProcessFn), it should:
1. Collect the AI subgraph
2. Call `BackendClient::execute_graph()`
3. Parse the response (base64 image → Value::Image)
4. Feed the result to downstream nodes

- [ ] **Step 4: Verify it compiles and runs**

Run: `cargo build && cargo run --release`
Expected: App starts. If Python backend is running, AI nodes appear in the menu. If not, only local nodes appear.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/node/backend.rs src/node/viewer.rs
git commit -m "feat: wire BackendClient into App, AI subgraph execution"
```

---

### Task 4: End-to-end test

Requires Python backend running + SDXL model on disk.

- [ ] **Step 1: Start Python backend**

Run: `cd python && uvicorn server:app --host 0.0.0.0 --port 8188`

- [ ] **Step 2: Start Rust frontend**

Run: `cargo run --release`

- [ ] **Step 3: Create SDXL pipeline in UI**

1. Right-click → AI → Load Checkpoint (set checkpoint path)
2. Right-click → AI → CLIP Text Encode (connect clip, type positive prompt)
3. Right-click → AI → CLIP Text Encode (connect clip, type negative prompt)
4. Right-click → AI → Empty Latent Image
5. Right-click → AI → KSampler (connect all 4 inputs)
6. Right-click → AI → VAE Decode (connect vae + latent)
7. Add Preview node, connect to VAE Decode output

- [ ] **Step 4: Trigger execution and verify**

Expected: Image appears in Preview node after inference completes.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: SDXL txt2img pipeline working end-to-end"
```
