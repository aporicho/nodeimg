# Transport 序列化重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 移除 app 对 NodeRegistry 的依赖，将序列化职责封装进 Transport trait。

**Architecture:** Transport trait 新增 `load_graph` 方法处理 JSON 解析+默认值填充。App 的 Serializer 改用已有的 `NodeTypeDef`（通过 `node_types()` 获取）代替 NodeRegistry。NodeViewer 中已有的 `node_type_defs` 缓存从 `Vec` 改为 `HashMap` 以支持高效查找。

**Tech Stack:** Rust, nodeimg workspace (types/engine/app crates)

**Spec:** `docs/superpowers/specs/2026-03-26-transport-serialization-design.md`

---

### Task 1: Transport trait 新增 `load_graph` 方法

**Files:**
- Modify: `crates/nodeimg-engine/src/transport/mod.rs:15-57` (ProcessingTransport trait)
- Modify: `crates/nodeimg-engine/src/transport/local.rs:205-463` (ProcessingTransport impl)

- [ ] **Step 1: 在 trait 中添加 `load_graph` 方法签名**

在 `crates/nodeimg-engine/src/transport/mod.rs` 顶部添加 import：

```rust
use nodeimg_types::serial_data::SerializedGraph;
```

在 `ProcessingTransport` trait 的 `would_create_cycle` 方法之后添加：

```rust
    /// Parse project file JSON and fill missing params with defaults.
    fn load_graph(&self, json: &str) -> Result<SerializedGraph, String>;
```

- [ ] **Step 2: 在 LocalTransport 中实现 `load_graph`**

在 `crates/nodeimg-engine/src/transport/local.rs` 的 `impl ProcessingTransport for LocalTransport` 块末尾（`would_create_cycle` 之后）添加：

```rust
    fn load_graph(&self, json: &str) -> Result<SerializedGraph, String> {
        let mut graph: SerializedGraph =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))?;

        let registry = self.registry.lock().unwrap();
        for node in &mut graph.nodes {
            if let Some(def) = registry.get(&node.type_id) {
                for param in &def.params {
                    if !node.params.contains_key(&param.name) {
                        if let Some(sv) = SerializedValue::from_value(&param.default) {
                            node.params.insert(param.name.clone(), sv);
                        }
                    }
                }
            }
        }

        Ok(graph)
    }
```

在 `local.rs` 顶部添加 import：

```rust
use nodeimg_types::serial_data::{SerializedGraph, SerializedValue};
```

- [ ] **Step 3: 确认编译通过**

Run: `cargo build --workspace 2>&1 | tail -5`
Expected: 编译成功

- [ ] **Step 4: 为 `load_graph` 添加测试**

在 `local.rs` 的 `#[cfg(test)] mod tests` 块末尾添加：

```rust
    #[test]
    fn test_load_graph_fills_defaults() {
        let transport = create_test_transport();
        register_test_source(&transport);

        // JSON with test_source node but missing "value" param
        let json = r#"{
            "version": 1,
            "nodes": [{
                "id": 0,
                "type_id": "test_source",
                "position": [0.0, 0.0],
                "params": {}
            }],
            "connections": []
        }"#;

        let graph = transport.load_graph(json).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        // "value" param should be filled with default (Float 0.0)
        assert!(graph.nodes[0].params.contains_key("value"));
    }

    #[test]
    fn test_load_graph_preserves_unknown_nodes() {
        let transport = create_test_transport();

        let json = r#"{
            "version": 1,
            "nodes": [{
                "id": 0,
                "type_id": "nonexistent_node",
                "position": [100.0, 200.0],
                "params": {"foo": {"type": "Float", "value": 1.0}}
            }],
            "connections": []
        }"#;

        let graph = transport.load_graph(json).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].type_id, "nonexistent_node");
        assert!(graph.nodes[0].params.contains_key("foo"));
    }

    #[test]
    fn test_load_graph_invalid_json() {
        let transport = create_test_transport();
        let result = transport.load_graph("not json");
        assert!(result.is_err());
    }
```

- [ ] **Step 5: 运行测试**

Run: `cargo test -p nodeimg-engine -- test_load_graph 2>&1 | tail -10`
Expected: 3 个测试通过

- [ ] **Step 6: Commit**

```bash
git add crates/nodeimg-engine/src/transport/mod.rs crates/nodeimg-engine/src/transport/local.rs
git commit -m "feat: Transport trait 新增 load_graph 方法 #73"
```

---

### Task 2: NodeViewer 的 `node_type_defs` 改为 HashMap

**Files:**
- Modify: `crates/nodeimg-app/src/node/viewer.rs:27` (字段类型)
- Modify: `crates/nodeimg-app/src/node/viewer.rs:49,54` (初始化)
- Modify: `crates/nodeimg-app/src/node/viewer.rs:68-70` (find_type_def)
- Modify: `crates/nodeimg-app/src/app.rs:114` (刷新缓存)

- [ ] **Step 1: 修改字段类型和初始化**

在 `viewer.rs` 中将字段声明改为：

```rust
    pub node_type_defs: HashMap<String, NodeTypeDef>,
```

修改 `NodeViewer::new()` 中的初始化：

```rust
        let node_type_defs: HashMap<String, NodeTypeDef> = transport
            .node_types()
            .unwrap_or_default()
            .into_iter()
            .map(|d| (d.type_id.clone(), d))
            .collect();
```

修改 `find_type_def`：

```rust
    fn find_type_def(&self, type_id: &str) -> Option<&NodeTypeDef> {
        self.node_type_defs.get(type_id)
    }
```

- [ ] **Step 2: 修改 app.rs 中的缓存刷新**

在 `app.rs:114` 将：

```rust
            viewer.node_type_defs = transport.node_types().unwrap_or_default();
```

改为：

```rust
            viewer.node_type_defs = transport
                .node_types()
                .unwrap_or_default()
                .into_iter()
                .map(|d| (d.type_id.clone(), d))
                .collect();
```

- [ ] **Step 3: 确认编译通过**

Run: `cargo build --workspace 2>&1 | tail -5`
Expected: 编译成功

- [ ] **Step 4: Commit**

```bash
git add crates/nodeimg-app/src/node/viewer.rs crates/nodeimg-app/src/app.rs
git commit -m "refactor: NodeViewer.node_type_defs 改为 HashMap 以支持高效查找 #73"
```

---

### Task 3: Serializer 改用 NodeTypeDef + app.rs 移除 with_registry

> serial.rs 和 app.rs 必须同时改，否则中间状态编译不过（签名不匹配）。

**Files:**
- Modify: `crates/nodeimg-app/src/node/serial.rs` (改造 Serializer)
- Modify: `crates/nodeimg-app/src/app.rs` (7 处 with_registry 调用)

- [ ] **Step 1: 改造 serial.rs — 修改 import**

替换 `serial.rs` 的 import：

```rust
use nodeimg_engine::transport::{NodeTypeDef, ParamValue};
use nodeimg_types::node_instance::NodeInstance;
use nodeimg_types::serial_data::*;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};
use std::collections::HashMap;
```

注意：移除 `use nodeimg_engine::NodeRegistry;`，新增 `use nodeimg_engine::transport::{NodeTypeDef, ParamValue};`。

- [ ] **Step 2: 改造 serial.rs — 移除 `load` 方法**

删除 `Serializer::load()` 方法（第 15-34 行）。这个逻辑已经在 Task 1 中移到了 `Transport::load_graph()`。

- [ ] **Step 3: 改造 serial.rs — `snapshot` 方法**

将 `snapshot` 的签名从 `registry: &NodeRegistry` 改为 `type_defs: &HashMap<String, NodeTypeDef>`：

```rust
    pub fn snapshot(
        snarl: &Snarl<NodeInstance>,
        type_defs: &HashMap<String, NodeTypeDef>,
    ) -> SerializedGraph {
        let mut nodes = Vec::new();
        for (node_id, pos, instance) in snarl.nodes_pos_ids() {
            let mut params = HashMap::new();
            for (k, v) in &instance.params {
                if let Some(sv) = SerializedValue::from_value(v) {
                    params.insert(k.clone(), sv);
                }
            }
            nodes.push(SerializedNode {
                id: node_id.0,
                type_id: instance.type_id.clone(),
                position: [pos.x, pos.y],
                params,
            });
        }

        let mut connections = Vec::new();
        for (out_pin, in_pin) in snarl.wires() {
            let from_name = snarl
                .get_node(out_pin.node)
                .and_then(|inst| type_defs.get(&inst.type_id))
                .and_then(|def| def.outputs.get(out_pin.output))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            let to_name = snarl
                .get_node(in_pin.node)
                .and_then(|inst| type_defs.get(&inst.type_id))
                .and_then(|def| def.inputs.get(in_pin.input))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            connections.push(SerializedConnection {
                from_node: out_pin.node.0,
                from_pin: from_name,
                to_node: in_pin.node.0,
                to_pin: to_name,
            });
        }

        SerializedGraph {
            version: FORMAT_VERSION,
            nodes,
            connections,
        }
    }
```

- [ ] **Step 4: 改造 serial.rs — `restore` 方法**

将 `restore` 的签名从 `registry: &NodeRegistry` 改为 `type_defs: &HashMap<String, NodeTypeDef>`。保留未知节点的 placeholder 路径：

```rust
    pub fn restore(
        graph: &SerializedGraph,
        type_defs: &HashMap<String, NodeTypeDef>,
    ) -> Snarl<NodeInstance> {
        let mut snarl = Snarl::new();
        let mut id_map: HashMap<usize, NodeId> = HashMap::new();

        for sn in &graph.nodes {
            let instance = match type_defs.get(&sn.type_id) {
                Some(def) => {
                    // Start with defaults from NodeTypeDef
                    let mut params: HashMap<String, nodeimg_types::value::Value> = def
                        .params
                        .iter()
                        .map(|p| (p.name.clone(), p.default.to_value()))
                        .collect();
                    // Override with saved values
                    for (k, sv) in &sn.params {
                        params.insert(k.clone(), sv.to_value());
                    }
                    NodeInstance {
                        type_id: sn.type_id.clone(),
                        params,
                    }
                }
                None => {
                    // Unknown node type — create placeholder
                    let mut params = HashMap::new();
                    for (k, sv) in &sn.params {
                        params.insert(k.clone(), sv.to_value());
                    }
                    NodeInstance {
                        type_id: sn.type_id.clone(),
                        params,
                    }
                }
            };
            let pos = eframe::egui::pos2(sn.position[0], sn.position[1]);
            let new_id = snarl.insert_node(pos, instance);
            id_map.insert(sn.id, new_id);
        }

        for sc in &graph.connections {
            let from_id = match id_map.get(&sc.from_node) {
                Some(&id) => id,
                None => continue,
            };
            let to_id = match id_map.get(&sc.to_node) {
                Some(&id) => id,
                None => continue,
            };

            let out_idx = snarl
                .get_node(from_id)
                .and_then(|inst| type_defs.get(&inst.type_id))
                .and_then(|def| def.outputs.iter().position(|p| p.name == sc.from_pin));
            let in_idx = snarl
                .get_node(to_id)
                .and_then(|inst| type_defs.get(&inst.type_id))
                .and_then(|def| def.inputs.iter().position(|p| p.name == sc.to_pin));

            if let (Some(out_idx), Some(in_idx)) = (out_idx, in_idx) {
                snarl.connect(
                    OutPinId {
                        node: from_id,
                        output: out_idx,
                    },
                    InPinId {
                        node: to_id,
                        input: in_idx,
                    },
                );
            }
        }

        snarl
    }
```

- [ ] **Step 5: 改造 app.rs — 添加 import**

在 `app.rs` 顶部添加：

```rust
use std::collections::HashMap;
use nodeimg_engine::transport::NodeTypeDef;
```

- [ ] **Step 6: 改造 app.rs — `try_auto_load`**

将 `try_auto_load` 改为：

```rust
    fn try_auto_load(transport: &LocalTransport, type_defs: &HashMap<String, NodeTypeDef>) -> Snarl<NodeInstance> {
        let path = Self::autosave_path();
        if path.exists() {
            if let Ok(json) = std::fs::read_to_string(&path) {
                match transport.load_graph(&json) {
                    Ok(graph) => {
                        let snarl = Serializer::restore(&graph, type_defs);
                        eprintln!("[project] Restored autosave ({} nodes)", graph.nodes.len());
                        return snarl;
                    }
                    Err(e) => {
                        eprintln!("[project] Failed to load autosave: {}", e);
                    }
                }
            }
        }
        Snarl::new()
    }
```

同时更新调用处，从 `Self::try_auto_load(&transport)` 改为 `Self::try_auto_load(&transport, &viewer.node_type_defs)`。

- [ ] **Step 7: 改造 app.rs — `auto_save`、`save_as`、`quick_save`**

三个方法中的 `with_registry` 调用统一改为：

```rust
        let graph = Serializer::snapshot(&self.snarl, &self.viewer.node_type_defs);
```

- [ ] **Step 8: 改造 app.rs — `open_file`**

将 `open_file` 中的两处 `with_registry` 调用改为：

```rust
                    let load_result = self.viewer.transport.load_graph(&json);
                    match load_result {
                        Ok(graph) => {
                            self.snarl = Serializer::restore(&graph, &self.viewer.node_type_defs);
```

- [ ] **Step 9: 确认编译通过**

Run: `cargo build --workspace 2>&1 | tail -5`
Expected: 编译成功

- [ ] **Step 10: 验证依赖已移除**

Run: `grep -r "with_registry" crates/nodeimg-app/`
Expected: 无输出

Run: `grep -r "NodeRegistry" crates/nodeimg-app/`
Expected: 无输出

- [ ] **Step 11: Commit**

```bash
git add crates/nodeimg-app/src/node/serial.rs crates/nodeimg-app/src/app.rs
git commit -m "refactor: Serializer 改用 NodeTypeDef，app 移除所有 with_registry 调用 #73"
```

---

### Task 4: 运行完整测试 + 验证

**Files:** 无新改动

- [ ] **Step 1: 运行全量测试**

Run: `cargo test --workspace 2>&1 | tail -20`
Expected: 所有测试通过

- [ ] **Step 2: 编译 release**

Run: `cargo build -p nodeimg-app --release 2>&1 | tail -5`
Expected: 编译成功

- [ ] **Step 3: Commit（如有修复）**

如果前面有测试失败并修复，在此提交修复。
