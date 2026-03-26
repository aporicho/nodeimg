# show_body 迁移实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 show_body 从直接访问 engine 内部类型迁移到 Transport 接口，删除 NodeViewer 的重复状态字段。

**Architecture:** Transport trait 新增 4 个细粒度方法（evaluate_local_sync、get_cached、pending_ai_execution、would_create_cycle），LocalTransport 实现委托给内部 EvalEngine/Cache。ExecutionManager 扩展为支持多并发任务，替代 AiExecutor。最后收紧 engine 导出。

**Tech Stack:** Rust, egui/eframe, nodeimg-engine, nodeimg-app

**Spec:** `docs/superpowers/specs/2026-03-26-show-body-migration-design.md`

---

### Task 0: 让 LocalTransport::execute() 支持 AI 节点

**Files:**
- Modify: `crates/nodeimg-engine/src/transport/local.rs`

当前 `LocalTransport::execute()` 遇到 AI 节点时只打日志跳过。旧的 `AiExecutor` 通过直接调用 `BackendClient::execute_graph()` 执行 AI 节点。迁移后 `ExecutionManager.submit()` 调用 `transport.execute()`，所以 transport 必须能执行 AI 节点。

LocalTransport 已持有 `backend: Option<BackendClient>` 字段（当前 `#[allow(dead_code)]`），只需启用它。

- [ ] **Step 1: �� execute() 中启用 AI 节点执行**

将 `local.rs` execute() 方法中 AI 节点跳过部分（约 lines 326-332）：

```rust
} else if def.gpu_process.is_none() {
    // AI node: skip with log (full AI integration is future work)
    eprintln!(
        "[local] node {} ({}) is an AI node — skipped",
        node_id, def.title
    );
}
```

替换为：

```rust
} else if def.gpu_process.is_none() {
    // AI node: send to backend if available
    if let Some(ref client) = self.backend {
        EvalEngine::execute_ai_subgraph(
            node_id,
            &nodes,
            &connections,
            &registry,
            &mut self.cache.lock().unwrap(),
            client,
            &mut sent_to_backend,
        )?;
    } else {
        eprintln!(
            "[local] node {} ({}) is an AI node but no backend available — skipped",
            node_id, def.title
        );
    }
}
```

注意：这需要在 execute() 方法中引入 `sent_to_backend: HashSet<NodeId>` 变量（参考 `EvalEngine::evaluate()` 的实现），并在循环开头检查 `sent_to_backend.contains(&node_id)`。同时需要将 Phase 3 中持有的 cache lock 模式调整为与 evaluate() 一致（AI 子图执行需要 `&mut Cache`）。

具体改动：
1. 在 Phase 3 循环前添加 `let mut sent_to_backend: HashSet<NodeId> = HashSet::new();`
2. 在循环开头的 cache 检查中添加 `|| sent_to_backend.contains(&node_id)`
3. 需要 `use std::collections::HashSet;`（如果还没有）
4. 删除 `backend` 字段上的 `#[allow(dead_code)]` 注释

- [ ] **Step 2: 运行测试**

Run: `cargo test -p nodeimg-engine`
Expected: 全部通过（没有 backend 的测试仍然跳过 AI 节点）

- [ ] **Step 3: 提交**

```bash
git add crates/nodeimg-engine/src/transport/local.rs
git commit -m "feat: LocalTransport::execute() 支持 AI 节点执行 #65"
```

---

### Task 1: Transport trait 新增 4 个方法 + LocalTransport 实现

**Files:**
- Modify: `crates/nodeimg-engine/src/transport/mod.rs`
- Modify: `crates/nodeimg-engine/src/transport/local.rs`

- [ ] **Step 1: 在 ProcessingTransport trait 中新增 4 个方法签名**

在 `crates/nodeimg-engine/src/transport/mod.rs` 的 `ProcessingTransport` trait 中，在 `node_types()` 之后添加：

```rust
/// Synchronous local evaluation (skips AI nodes). Results written to internal cache.
fn evaluate_local_sync(&self, request: GraphRequest) -> Result<(), String>;

/// Query cached output for a node (does not trigger execution).
fn get_cached(&self, node_id: NodeId) -> Option<HashMap<String, Value>>;

/// Check for pending AI execution in the graph.
/// Returns (ai_node_id, serialized_subgraph_json) if found.
fn pending_ai_execution(
    &self,
    request: GraphRequest,
) -> Option<(NodeId, serde_json::Value)>;

/// Check if adding connections would create a cycle.
/// Uses ConnectionRequest (already defined in this module) as the connection type.
fn would_create_cycle(
    &self,
    target: NodeId,
    connections: &[ConnectionRequest],
) -> bool;
```

注意：`get_cached` 返回 `Option<HashMap<String, Value>>`（clone），因为 Cache 在 Mutex 后面。`evaluate_local_sync` 返回 `Result<(), String>`，结果通过 `get_cached` 读取。`would_create_cycle` 复用已有的 `ConnectionRequest` 类型，不需要新增 `ConnectionInfo`。

- [ ] **Step 2: 在 LocalTransport 中实现 evaluate_local_sync**

在 `crates/nodeimg-engine/src/transport/local.rs` 的 `impl ProcessingTransport for LocalTransport` 块中添加：

```rust
fn evaluate_local_sync(&self, request: GraphRequest) -> Result<(), String> {
    // Phase 1: Build internal types from request (lock registry)
    let (nodes, connections, order) = {
        let registry = self.registry.lock().unwrap();
        let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
        for (&node_id, req) in &request.nodes {
            let mut params: HashMap<String, Value> = HashMap::new();
            if let Some(def) = registry.get(&req.type_id) {
                for p in &def.params {
                    params.insert(p.name.clone(), p.default.clone());
                }
            }
            for (key, pv) in &req.params {
                params.insert(key.clone(), pv.to_value());
            }
            nodes.insert(node_id, NodeInstance { type_id: req.type_id.clone(), params });
        }

        let connections: Vec<Connection> = request.connections.iter().map(|c| Connection {
            from_node: c.from_node, from_pin: c.from_pin.clone(),
            to_node: c.to_node, to_pin: c.to_pin.clone(),
        }).collect();

        let order = EvalEngine::topo_sort(request.target_node, &connections)?;
        (nodes, connections, order)
    };

    // Phase 2: Evaluate using EvalEngine (backend=None to skip AI nodes)
    let registry = self.registry.lock().unwrap();
    let type_registry = self.type_registry.lock().unwrap();
    let mut cache = self.cache.lock().unwrap();

    cache.clear_downstream();
    for conn in &connections {
        cache.set_downstream(conn.from_node, conn.to_node);
    }

    EvalEngine::evaluate(
        request.target_node,
        &nodes,
        &connections,
        &registry,
        &type_registry,
        &mut cache,
        None, // no backend — skip AI nodes
        self.gpu_ctx.as_ref(),
    )
}
```

注意：这里和现有 `execute()` 有重复的"准备阶段"代码。可以在后续重构中提取为共享 helper，但本次 PR 不做——先让功能工作。

- [ ] **Step 3: 实现 get_cached**

```rust
fn get_cached(&self, node_id: NodeId) -> Option<HashMap<String, Value>> {
    let cache = self.cache.lock().unwrap();
    cache.get(node_id).cloned()
}
```

- [ ] **Step 4: 实现 pending_ai_execution**

```rust
fn pending_ai_execution(
    &self,
    request: GraphRequest,
) -> Option<(NodeId, serde_json::Value)> {
    let registry = self.registry.lock().unwrap();

    let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
    for (&node_id, req) in &request.nodes {
        let mut params: HashMap<String, Value> = HashMap::new();
        if let Some(def) = registry.get(&req.type_id) {
            for p in &def.params {
                params.insert(p.name.clone(), p.default.clone());
            }
        }
        for (key, pv) in &req.params {
            params.insert(key.clone(), pv.to_value());
        }
        nodes.insert(node_id, NodeInstance { type_id: req.type_id.clone(), params });
    }

    let connections: Vec<Connection> = request.connections.iter().map(|c| Connection {
        from_node: c.from_node, from_pin: c.from_pin.clone(),
        to_node: c.to_node, to_pin: c.to_pin.clone(),
    }).collect();

    let cache = self.cache.lock().unwrap();
    EvalEngine::pending_ai_execution(
        request.target_node,
        &nodes,
        &connections,
        &registry,
        &cache,
    )
}
```

- [ ] **Step 5: 实现 would_create_cycle**

```rust
fn would_create_cycle(
    &self,
    target: NodeId,
    connections: &[ConnectionRequest],
) -> bool {
    let internal_connections: Vec<Connection> = connections.iter().map(|c| Connection {
        from_node: c.from_node, from_pin: c.from_pin.clone(),
        to_node: c.to_node, to_pin: c.to_pin.clone(),
    }).collect();
    EvalEngine::topo_sort(target, &internal_connections).is_err()
}
```

- [ ] **Step 6: 写测试 — evaluate_local_sync**

在 `crates/nodeimg-engine/src/transport/local.rs` 的 `#[cfg(test)] mod tests` 中添加：

```rust
#[test]
fn test_evaluate_local_sync() {
    let transport = create_test_transport();
    register_test_source(&transport);
    register_test_add_one(&transport);

    let request = GraphRequest {
        nodes: HashMap::from([
            (0, NodeRequest {
                type_id: "test_source".into(),
                params: HashMap::from([("value".into(), ParamValue::Float(10.0))]),
            }),
            (1, NodeRequest {
                type_id: "test_add_one".into(),
                params: HashMap::new(),
            }),
        ]),
        connections: vec![ConnectionRequest {
            from_node: 0, from_pin: "value".into(),
            to_node: 1, to_pin: "value".into(),
        }],
        target_node: 1,
    };

    transport.evaluate_local_sync(request).unwrap();

    // Results should be in cache
    let cached = transport.get_cached(1).unwrap();
    match cached.get("result") {
        Some(Value::Float(v)) => assert_eq!(*v, 11.0),
        other => panic!("expected Float(11.0), got {:?}", other),
    }
}
```

- [ ] **Step 7: 写测试 — get_cached 空缓存**

```rust
#[test]
fn test_get_cached_empty() {
    let transport = create_test_transport();
    assert!(transport.get_cached(999).is_none());
}
```

- [ ] **Step 8: 写测试 — would_create_cycle**

```rust
#[test]
fn test_would_create_cycle_no_cycle() {
    let transport = create_test_transport();
    let connections = vec![ConnectionRequest {
        from_node: 0, from_pin: "out".into(),
        to_node: 1, to_pin: "in".into(),
    }];
    assert!(!transport.would_create_cycle(1, &connections));
}

#[test]
fn test_would_create_cycle_with_cycle() {
    let transport = create_test_transport();
    let connections = vec![
        ConnectionRequest {
            from_node: 0, from_pin: "out".into(),
            to_node: 1, to_pin: "in".into(),
        },
        ConnectionRequest {
            from_node: 1, from_pin: "out".into(),
            to_node: 0, to_pin: "in".into(),
        },
    ];
    assert!(transport.would_create_cycle(0, &connections));
}
```

- [ ] **Step 9: 运行测试**

Run: `cargo test -p nodeimg-engine`
Expected: 全部通过，包括新增的 4 个测试

- [ ] **Step 10: 提交**

```bash
git add crates/nodeimg-engine/src/transport/mod.rs crates/nodeimg-engine/src/transport/local.rs
git commit -m "feat: Transport trait 新增 evaluate_local_sync/get_cached/pending_ai_execution/would_create_cycle #65"
```

---

### Task 2: 扩展 ExecutionManager 支持多并发任务

**Files:**
- Modify: `crates/nodeimg-app/src/execution.rs`

- [ ] **Step 1: 重构 ExecutionManager 为多任务模式**

将 `crates/nodeimg-app/src/execution.rs` 中的 `ExecutionManager` 从单任务改为多任务：

```rust
use eframe::egui;
use nodeimg_engine::transport::{ExecuteProgress, GraphRequest, ProcessingTransport};
use nodeimg_engine::NodeId;
use std::sync::mpsc;
use std::sync::Arc;
use std::collections::HashMap;

struct TaggedSender {
    gen: u64,
    trigger: NodeId,
    tx: mpsc::Sender<(NodeId, u64, ExecuteProgress)>,
}

impl TaggedSender {
    fn into_sender(self) -> mpsc::Sender<ExecuteProgress> {
        let (inner_tx, inner_rx) = mpsc::channel();
        let tagged_tx = self.tx;
        let gen = self.gen;
        let trigger = self.trigger;
        std::thread::spawn(move || {
            while let Ok(progress) = inner_rx.recv() {
                if tagged_tx.send((trigger, gen, progress)).is_err() {
                    break;
                }
            }
        });
        inner_tx
    }
}

/// Per-trigger-node task state.
struct TaskState {
    generation: u64,
}

/// App-side execution dispatcher. Supports multiple concurrent tasks (one per trigger node).
pub struct ExecutionManager {
    transport: Arc<dyn ProcessingTransport>,
    progress_tx: mpsc::Sender<(NodeId, u64, ExecuteProgress)>,
    progress_rx: mpsc::Receiver<(NodeId, u64, ExecuteProgress)>,
    tasks: HashMap<NodeId, TaskState>,
    repaint: Option<egui::Context>,
}

impl ExecutionManager {
    pub fn new(transport: Arc<dyn ProcessingTransport>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            transport,
            progress_tx: tx,
            progress_rx: rx,
            tasks: HashMap::new(),
            repaint: None,
        }
    }

    pub fn set_repaint_ctx(&mut self, ctx: egui::Context) {
        self.repaint = Some(ctx);
    }

    /// Submit an execution request for a specific trigger node.
    /// If a task is already running for this trigger, it is implicitly cancelled
    /// (generation bump — stale results will be discarded).
    pub fn submit(&mut self, trigger_node: NodeId, request: GraphRequest) {
        let gen = self.tasks
            .get(&trigger_node)
            .map(|s| s.generation + 1)
            .unwrap_or(1);
        self.tasks.insert(trigger_node, TaskState { generation: gen });

        let transport = Arc::clone(&self.transport);
        let tagged_tx = TaggedSender {
            gen,
            trigger: trigger_node,
            tx: self.progress_tx.clone(),
        };

        let repaint = self.repaint.clone();
        std::thread::spawn(move || {
            let result = transport.execute(request, tagged_tx.into_sender());
            // No need to send extra error — TaggedSender thread handles it
            if let Err(e) = result {
                // Send error via the original tx since TaggedSender thread may have ended
                // This is best-effort; the trigger may already be cancelled
                eprintln!("[execution] Task error for trigger {}: {}", trigger_node, e);
            }
            if let Some(ctx) = repaint {
                ctx.request_repaint();
            }
        });
    }

    /// Non-blocking poll: drain all completed results from all tasks.
    pub fn poll(&mut self) -> Vec<(NodeId, ExecuteProgress)> {
        let mut results = Vec::new();
        while let Ok((trigger, gen, progress)) = self.progress_rx.try_recv() {
            let current_gen = self.tasks.get(&trigger).map(|s| s.generation);
            if current_gen != Some(gen) {
                continue; // stale result
            }
            let is_terminal = matches!(
                progress,
                ExecuteProgress::Finished | ExecuteProgress::Error { .. }
            );
            results.push((trigger, progress));
            if is_terminal {
                self.tasks.remove(&trigger);
            }
        }
        results
    }

    /// Cancel a specific trigger node's task.
    pub fn cancel(&mut self, trigger_node: NodeId) {
        if let Some(state) = self.tasks.get_mut(&trigger_node) {
            state.generation += 1;
        }
        self.tasks.remove(&trigger_node);
    }

    /// Cancel all running tasks.
    pub fn cancel_all(&mut self) {
        self.tasks.clear();
    }

    /// Check if a task is running for the given trigger node.
    pub fn is_running(&self, trigger_node: NodeId) -> bool {
        self.tasks.contains_key(&trigger_node)
    }

    pub fn transport(&self) -> &Arc<dyn ProcessingTransport> {
        &self.transport
    }
}
```

- [ ] **Step 2: 运行 cargo build 确认编译通过**

Run: `cargo build -p nodeimg-app`
Expected: 编译通过（ExecutionManager 的公共 API 有变化但尚未被调用方引用新签名）

注意：`submit()` 签名从 `submit(request)` 变为 `submit(trigger_node, request)`，`poll()` 返回类型从 `Vec<ExecuteProgress>` 变为 `Vec<(NodeId, ExecuteProgress)>`。这些变化会在 Task 3 中一起��新调用方。

- [ ] **Step 3: 提交**

```bash
git add crates/nodeimg-app/src/execution.rs
git commit -m "feat: ExecutionManager 支持多并发任务（per-trigger-node tracking）#65"
```

---

### Task 3: 迁移 show_body

**Files:**
- Modify: `crates/nodeimg-app/src/node/viewer.rs`

这是最核心的改动。show_body 中的所有 engine 直接调用替换为 transport 方法。

- [ ] **Step 1: 在 NodeViewer 中添加 execution_manager 字段**

在 `NodeViewer` struct 中添加字段（暂时和旧字段共存）：

```rust
pub execution_manager: ExecutionManager,
```

在 `NodeViewer::new()` 中初始化：

```rust
use crate::execution::ExecutionManager;

// 在 Self { ... } 中添加：
execution_manager: ExecutionManager::new(Arc::clone(&transport) as Arc<dyn ProcessingTransport>),
```

需要在文件顶部添加 import：
```rust
use crate::execution::ExecutionManager;
use nodeimg_engine::transport::{ConnectionRequest, GraphRequest, NodeRequest, ParamValue};
```

- [ ] **Step 2: 添加 build_graph_request helper**

在 `impl NodeViewer` 块中添加一个辅助方法，将 snarl 图转换为 `GraphRequest`（transport 的请求格式）：

```rust
/// Build a GraphRequest from the current Snarl graph for a given target node.
fn build_graph_request(&self, target: NodeId, snarl: &Snarl<NodeInstance>) -> GraphRequest {
    let mut nodes = HashMap::new();
    for (nid, instance) in snarl.node_ids() {
        let params: HashMap<String, ParamValue> = instance
            .params
            .iter()
            .filter_map(|(k, v)| ParamValue::from_value(v).map(|pv| (k.clone(), pv)))
            .collect();
        nodes.insert(nid.0, NodeRequest {
            type_id: instance.type_id.clone(),
            params,
        });
    }

    let connections = self.build_connection_requests(snarl);

    GraphRequest {
        nodes,
        connections,
        target_node: target.0,
    }
}

/// Build ConnectionRequest list from the Snarl graph (transport-compatible format).
fn build_connection_requests(&self, snarl: &Snarl<NodeInstance>) -> Vec<ConnectionRequest> {
    let mut connections = Vec::new();
    for (node_id, instance) in snarl.node_ids() {
        let def = match self.find_type_def(&instance.type_id) {
            Some(d) => d,
            None => continue,
        };
        for (i, input_pin) in def.inputs.iter().enumerate() {
            let in_pin = snarl.in_pin(InPinId { node: node_id, input: i });
            for remote in &in_pin.remotes {
                let upstream_instance = &snarl[remote.node];
                if let Some(upstream_def) = self.find_type_def(&upstream_instance.type_id) {
                    if let Some(out_pin) = upstream_def.outputs.get(remote.output) {
                        connections.push(ConnectionRequest {
                            from_node: remote.node.0,
                            from_pin: out_pin.name.clone(),
                            to_node: node_id.0,
                            to_pin: input_pin.name.clone(),
                        });
                    }
                }
            }
        }
    }
    connections
}
```

- [ ] **Step 3: 重写 show_body 使用 transport 和 execution_manager**

替换 `show_body` 方法的整个预览区域（从 `if has_preview {` 到最后的闭合 `}`），保留参数渲染部分不变（它只用 widget_registry）：

```rust
fn show_body(
    &mut self,
    node_id: NodeId,
    _inputs: &[InPin],
    _outputs: &[OutPin],
    ui: &mut Ui,
    snarl: &mut Snarl<NodeInstance>,
) {
    let instance = snarl[node_id].clone();
    let type_def = match self.find_type_def(&instance.type_id) {
        Some(d) => d,
        None => {
            ui.label("Unknown node type");
            return;
        }
    };
    let params_def: Vec<_> = type_def.params.clone();
    let has_preview = type_def.has_preview;

    ui.vertical(|ui: &mut Ui| {
        ui.set_max_width(crate::theme::tokens::NODE_MAX_WIDTH);
        let mut any_changed = false;

        // Render parameter widgets (unchanged — uses widget_registry)
        for param_def in &params_def {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(crate::theme::tokens::PARAM_LABEL_WIDTH, ui.available_height()),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.colored_label(self.theme.text_secondary(), &param_def.name);
                    },
                );

                let disabled = false;

                let old_value = snarl[node_id].params.get(&param_def.name).cloned();
                if let Some(mut value) = old_value {
                    // Convert ConstraintInfo to engine Constraint for widget rendering
                    let engine_constraint = Self::constraint_info_to_engine(&param_def.constraint);
                    let constraint_type = engine_constraint.constraint_type();
                    let widget_override_id = param_def.widget_override.as_ref()
                        .map(|w| nodeimg_types::widget::WidgetId(w.clone()));
                    let data_type_id = nodeimg_types::data_type::DataTypeId::new(&param_def.data_type);

                    let changed = self.widget_registry.render(
                        widget_override_id.as_ref(),
                        &data_type_id,
                        &constraint_type,
                        ui,
                        &mut value,
                        &engine_constraint,
                        &param_def.name,
                        disabled,
                    );
                    if changed {
                        snarl[node_id].params.insert(param_def.name.clone(), value);
                        any_changed = true;
                    }
                }
            });
        }

        if any_changed {
            self.transport.invalidate(node_id.0);
            self.textures.remove(&node_id);
            self.execution_manager.cancel_all();
            self.ai_errors.remove(&node_id);
            self.graph_dirty = true;
        }

        // Render preview area
        if has_preview {
            // Step 1: Poll background AI results
            for (trigger, progress) in self.execution_manager.poll() {
                match progress {
                    nodeimg_engine::transport::ExecuteProgress::NodeCompleted { node_id: completed_id, .. } => {
                        self.textures.remove(&NodeId(completed_id));
                    }
                    nodeimg_engine::transport::ExecuteProgress::Error { message, .. } => {
                        self.ai_errors.insert(NodeId(trigger), message);
                    }
                    nodeimg_engine::transport::ExecuteProgress::Finished => {
                        self.textures.remove(&NodeId(trigger));
                    }
                }
            }

            // Step 2: Evaluate local nodes synchronously
            let request = self.build_graph_request(node_id, snarl);
            if let Err(e) = self.transport.evaluate_local_sync(request.clone()) {
                self.backend_status = Some(e);
            } else {
                self.backend_status = None;
            }

            // Step 3: Check for pending AI work
            if !self.execution_manager.is_running(node_id.0) {
                if let Some((ai_node_id, graph_json)) =
                    self.transport.pending_ai_execution(request.clone())
                {
                    eprintln!(
                        "[backend] Spawning async AI execution: trigger={} ai_node={}",
                        node_id.0, ai_node_id
                    );
                    self.ai_errors.remove(&node_id);
                    self.execution_manager.set_repaint_ctx(ui.ctx().clone());
                    // Build a request specifically for the AI subgraph
                    // For now, submit the full graph — transport.execute() will handle AI nodes
                    let ai_request = self.build_graph_request(node_id, snarl);
                    self.execution_manager.submit(node_id.0, ai_request);
                }
            }

            // Step 4: UI status display
            if self.execution_manager.is_running(node_id.0) {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Generating...");
                });
            }

            if let Some(ref status) = self.backend_status {
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), status);
            }

            if let Some(err) = self.ai_errors.get(&node_id) {
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
            }

            // Step 5: Render preview from cache
            if let Some(cached) = self.transport.get_cached(node_id.0) {
                if let Some(Value::Image(img)) = cached.get("image") {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                    self.preview_texture = Some(ui.ctx().load_texture(
                        "preview-panel",
                        color_image.clone(),
                        egui::TextureOptions::LINEAR,
                    ));

                    self.textures.entry(node_id).or_insert_with(|| {
                        ui.ctx().load_texture(
                            format!("node-{:?}", node_id),
                            color_image,
                            egui::TextureOptions::LINEAR,
                        )
                    });
                    if let Some(tex) = self.textures.get(&node_id) {
                        let tex_size = tex.size_vec2();
                        let scale = (200.0_f32 / tex_size.x.max(tex_size.y)).min(1.0);
                        ui.image(egui::load::SizedTexture::new(tex.id(), tex_size * scale));
                    }
                }
            } else if !self.execution_manager.is_running(node_id.0)
                && !self.ai_errors.contains_key(&node_id)
            {
                ui.label("No input connected");
            }
        }
    });
}
```

注意：参数渲染部分需要将 `ConstraintInfo` 转回 engine 的 `Constraint` 类型，因为 `WidgetRegistry::render()` 接受 `&Constraint`。添加辅助方法：

```rust
/// Convert transport ConstraintInfo back to engine Constraint (for widget rendering).
fn constraint_info_to_engine(info: &Option<ConstraintInfo>) -> nodeimg_types::constraint::Constraint {
    match info {
        None => nodeimg_types::constraint::Constraint::None,
        Some(ConstraintInfo::Range { min, max }) => {
            nodeimg_types::constraint::Constraint::Range { min: *min, max: *max }
        }
        Some(ConstraintInfo::Options(opts)) => {
            nodeimg_types::constraint::Constraint::Enum { options: opts.clone() }
        }
        Some(ConstraintInfo::FilePath { filters }) => {
            nodeimg_types::constraint::Constraint::FilePath { filters: filters.clone() }
        }
    }
}
```

- [ ] **Step 4: 更新 invalidate/invalidate_all 方法**

迁移后只需通过 transport，不再需要双重 invalidate：

```rust
pub fn invalidate(&mut self, node_id: NodeId) {
    self.transport.invalidate(node_id.0);
    self.textures.remove(&node_id);
}

pub fn invalidate_all(&mut self) {
    self.transport.invalidate_all();
    self.textures.clear();
}
```

- [ ] **Step 5: 运行 cargo build**

Run: `cargo build -p nodeimg-app`
Expected: 编译通过。此时新旧字段共存，旧字段有 dead_code 警告。

- [ ] **Step 6: 提交**

```bash
git add crates/nodeimg-app/src/node/viewer.rs
git commit -m "feat: show_body 迁移到 transport 接口 #65"
```

---

### Task 4: 迁移 connect() 环检测

**Files:**
- Modify: `crates/nodeimg-app/src/node/viewer.rs`

- [ ] **Step 1: 将 connect() 中的 EvalEngine::topo_sort 替换为 transport.would_create_cycle**

将 `connect()` 方法中的环检测部分替换为：

```rust
// Cycle detection (via transport)
let would_create_cycle = {
    let mut connections = self.build_connection_requests(snarl);
    let from_pin_name = self
        .find_type_def(&from_type_id)
        .and_then(|def| def.outputs.get(from.id.output).map(|p| p.name.clone()))
        .unwrap_or_default();
    let to_pin_name = self
        .find_type_def(&to_type_id)
        .and_then(|def| def.inputs.get(to.id.input).map(|p| p.name.clone()))
        .unwrap_or_default();
    connections.push(ConnectionRequest {
        from_node: from.id.node.0,
        from_pin: from_pin_name,
        to_node: to.id.node.0,
        to_pin: to_pin_name,
    });
    self.transport.would_create_cycle(to.id.node.0, &connections)
};
```

- [ ] **Step 2: 删除旧的 build_connections 方法**

删除返回 `Vec<Connection>` 的 `build_connections` 方法（已被 `build_connection_requests` 替代）。

- [ ] **Step 3: 运行 cargo build**

Run: `cargo build -p nodeimg-app`
Expected: 编译通过

- [ ] **Step 4: 提交**

```bash
git add crates/nodeimg-app/src/node/viewer.rs
git commit -m "refactor: connect() 环检测迁移到 transport.would_create_cycle #65"
```

---

### Task 5: 清理 NodeViewer 重复字段 + app.rs 双重注册

**Files:**
- Modify: `crates/nodeimg-app/src/node/viewer.rs`
- Modify: `crates/nodeimg-app/src/app.rs`

- [ ] **Step 1: 从 NodeViewer struct 中删除重复字段**

删除以下字段及其 `new()` 中的初始化代码：

```rust
// 删除这些字段
pub type_registry: DataTypeRegistry,
pub category_registry: CategoryRegistry,
pub node_registry: NodeRegistry,
pub cache: Cache,
pub backend: Option<BackendClient>,
pub ai_executor: AiExecutor,
```

同时删除不再需要的 import：
```rust
// 删除这些 use
use nodeimg_engine::ai_task::AiExecutor;
use nodeimg_engine::backend::BackendClient;
use nodeimg_engine::cache::Cache;
use nodeimg_engine::eval::{Connection, EvalEngine};
use nodeimg_engine::registry::{NodeInstance, NodeRegistry};
use nodeimg_types::category::CategoryRegistry;
use nodeimg_types::data_type::DataTypeRegistry;
```

保留必要的 import：
```rust
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::{
    ConnectionRequest, ConstraintInfo, GraphRequest, NodeRequest,
    NodeTypeDef, ParamValue, ProcessingTransport,
};
use nodeimg_engine::registry::NodeInstance;  // 仍需要，Snarl<NodeInstance> 用
use nodeimg_engine::NodeId as EngineNodeId;  // 可能需要区分 egui NodeId 和 engine NodeId
use nodeimg_types::value::Value;
```

- [ ] **Step 2: 更新 NodeViewer::new()**

简化构造函数，删除旧字段的初始化：

```rust
pub fn new(theme: Arc<dyn Theme>, gpu_ctx: Option<Arc<GpuContext>>, transport: Arc<LocalTransport>) -> Self {
    let node_type_defs = transport.node_types().unwrap_or_default();
    let execution_manager = ExecutionManager::new(
        Arc::clone(&transport) as Arc<dyn ProcessingTransport>,
    );

    Self {
        transport,
        node_type_defs,
        widget_registry: WidgetRegistry::with_builtins(),
        execution_manager,
        textures: HashMap::new(),
        preview_texture: None,
        theme,
        gpu_ctx,
        backend_status: None,
        ai_errors: HashMap::new(),
        graph_dirty: false,
    }
}
```

- [ ] **Step 3: 清理 app.rs — 删除双重 AI 注册**

将 `app.rs` 中 lines 108-131 的双重注册替换为：

```rust
// Register AI nodes if connected
if connected {
    match transport.register_remote_nodes(&backend) {
        Ok(count) => eprintln!("[backend] Registered {} AI node types", count),
        Err(e) => eprintln!("[backend] Failed to register AI nodes: {}", e),
    }
    // Refresh node_type_defs after AI node registration
    viewer.node_type_defs = transport.node_types().unwrap_or_default();
}
```

同时删除 `viewer.backend = Some(backend);`（line 132）— backend 已在 transport 内部。

更新 app.rs 的 import，删除不再需要的：
```rust
// 删除
use nodeimg_engine::backend::BackendClient;
// 改为从 transport::local 模块获取 BackendClient（如果 app.rs 仍需要创建 backend）
```

注意：app.rs 仍需要 `BackendClient` 来做 health_check 和传给 transport.register_remote_nodes。需要保留 `BackendClient` 的 import，但改为从 transport 模块或保持现有路径（在 Task 6 收紧导出前仍可用）。

- [ ] **Step 4: 运行 cargo build**

Run: `cargo build -p nodeimg-app`
Expected: 编译通过，无 dead_code 警告（旧字段已删除）

- [ ] **Step 5: 运行 cargo test**

Run: `cargo test --workspace`
Expected: 全部通过

- [ ] **Step 6: 提交**

```bash
git add crates/nodeimg-app/src/node/viewer.rs crates/nodeimg-app/src/app.rs
git commit -m "refactor: 删除 NodeViewer 重复字段，消除双重 AI 注册 #65"
```

---

### Task 6: 删除 ai_task.rs + 收紧 engine 导出

**Files:**
- Delete: `crates/nodeimg-engine/src/internal/ai_task.rs`
- Modify: `crates/nodeimg-engine/src/internal/mod.rs`
- Modify: `crates/nodeimg-engine/src/lib.rs`

- [ ] **Step 1: 删除 ai_task.rs**

```bash
rm -f crates/nodeimg-engine/src/internal/ai_task.rs
```

- [ ] **Step 2: 从 internal/mod.rs 中删除 ai_task 模块声明**

在 `crates/nodeimg-engine/src/internal/mod.rs` 中删除 `pub mod ai_task;` 行。

- [ ] **Step 3: 收紧 lib.rs 导出**

将 `crates/nodeimg-engine/src/lib.rs` 替换为：

```rust
pub(crate) mod builtins;
pub(crate) mod internal;
pub mod transport;

// Only export types that app layer still needs directly
pub use internal::cache::NodeId;
pub use internal::registry::NodeInstance;
```

- [ ] **Step 4: 修复 app.rs 中 BackendClient 的 import 路径**

`BackendClient` 不再通过 `nodeimg_engine::backend` 导出。需要：

选项 A：在 transport 模块中 re-export BackendClient：
```rust
// crates/nodeimg-engine/src/transport/mod.rs 顶部添加
pub use crate::internal::backend::BackendClient;
```

选项 B：让 LocalTransport 提供一个方法来处理后端连接检查。

**推荐选项 A**——BackendClient 是 app 层创建 transport 时需要的，属于公共 API。

- [ ] **Step 5: 修复所有编译错误**

收紧导出后，app 层可能还有其他地方引用了旧路径。逐个修复：

- `nodeimg_engine::registry::NodeInstance` → `nodeimg_engine::NodeInstance`
- `nodeimg_engine::backend::BackendClient` → `nodeimg_engine::transport::BackendClient`
- 删除所有对 `nodeimg_engine::ai_task`、`nodeimg_engine::cache`、`nodeimg_engine::eval`、`nodeimg_engine::registry`（除 NodeInstance）、`nodeimg_engine::menu` 的引用

检查方式：`cargo build --workspace 2>&1 | head -50`，逐个修复。

- [ ] **Step 6: 运行 cargo build + cargo test**

Run: `cargo build --workspace && cargo test --workspace`
Expected: 全部通过

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "refactor: 删除 ai_task.rs，收紧 engine 导出为 transport + NodeId + NodeInstance #65"
```

---

### Task 7: 最终验证

**Files:** 无新改动

- [ ] **Step 1: cargo test --workspace**

Run: `cargo test --workspace`
Expected: 全部通过

- [ ] **Step 2: cargo build --release**

Run: `cargo build -p nodeimg-app --release`
Expected: 编译通过

- [ ] **Step 3: 手动运行验证**

Run: `cargo run -p nodeimg-app --release`
验证项目：
1. 启动正常
2. 添加节点、连接、断开
3. 调节参数滑条——预览实时更新（无延迟）
4. 加载/保存项目文件
5. 环检测正常工作（尝试创建环形连接，应被拒绝）

- [ ] **Step 4: 确认 engine 导出干净**

检查 `nodeimg-engine` 的公共 API：
```bash
grep -n "^pub " crates/nodeimg-engine/src/lib.rs
```
Expected: 只有 `pub mod transport`、`pub use ... NodeId`、`pub use ... NodeInstance`

- [ ] **Step 5: 提交并推送**

如果一切正常，推送分支准备 PR。
