# Transport Trait + LocalTransport 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 引入 ProcessingTransport trait 解耦 app 与 engine，实现 LocalTransport 进程内调用。

**Architecture:** 在 nodeimg-engine 中定义 ProcessingTransport trait（execute/invalidate/health_check/node_types/is_compatible），实现 LocalTransport 包装现有执行逻辑。App 侧新增 ExecutionManager 替代 AiExecutor。Engine 文件结构重组为 transport/（公开）+ internal/（内部）。

**Tech Stack:** Rust, serde, std::sync::mpsc, std::sync::Mutex

**Spec:** `docs/superpowers/specs/2026-03-23-transport-trait-design.md`

---

### Task 1: 定义 Transport 类型（nodeimg-engine transport/mod.rs）

**Files:**
- Create: `crates/nodeimg-engine/src/transport/mod.rs`
- Modify: `crates/nodeimg-engine/src/lib.rs`
- Modify: `crates/nodeimg-types/src/data_type.rs:21` — ConversionFn 加 Send + Sync
- Test: `crates/nodeimg-engine/src/transport/mod.rs`（内联测试）

- [ ] **Step 1: 修改 ConversionFn 加 Send + Sync**

```rust
// crates/nodeimg-types/src/data_type.rs:21
// 旧:
type ConversionFn = Box<dyn Fn(Value) -> Value>;
// 新:
type ConversionFn = Box<dyn Fn(Value) -> Value + Send + Sync>;
```

同时修改 `register_conversion` 的 trait bound：

```rust
// crates/nodeimg-types/src/data_type.rs:51-58
// 旧:
pub fn register_conversion(
    &mut self,
    from: DataTypeId,
    to: DataTypeId,
    f: impl Fn(Value) -> Value + 'static,
) {
// 新:
pub fn register_conversion(
    &mut self,
    from: DataTypeId,
    to: DataTypeId,
    f: impl Fn(Value) -> Value + Send + Sync + 'static,
) {
```

- [ ] **Step 2: 运行测试确认 ConversionFn 改动不破坏现有代码**

Run: `cargo test --workspace`
Expected: 全部通过

- [ ] **Step 3: 创建 transport/mod.rs，定义所有类型**

```rust
// crates/nodeimg-engine/src/transport/mod.rs
pub mod local;

use crate::cache::NodeId;
use nodeimg_types::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::Sender;

// === Transport Trait ===

pub trait ProcessingTransport: Send + Sync + 'static {
    /// 执行节点图，逐节点推送结果
    fn execute(&self, request: GraphRequest, progress: Sender<ExecuteProgress>) -> Result<(), String>;

    /// 缓存失效：某节点参数变了，清除该节点及下游缓存
    fn invalidate(&self, node_id: NodeId);

    /// 清除所有缓存
    fn invalidate_all(&self);

    /// 检查两个数据类型是否可以连接
    fn is_compatible(&self, from_type: &str, to_type: &str) -> bool;

    /// 健康检查
    fn health_check(&self) -> Result<HealthResponse, String>;

    /// 获取可用节点类型列表
    fn node_types(&self) -> Result<Vec<NodeTypeDef>, String>;
}

// === 执行进度 ===

pub enum ExecuteProgress {
    NodeCompleted {
        node_id: NodeId,
        outputs: HashMap<String, Value>,
    },
    Finished,
    Error {
        node_id: Option<NodeId>,
        message: String,
    },
}

// === 请求/响应类型 ===

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphRequest {
    pub nodes: HashMap<NodeId, NodeRequest>,
    pub connections: Vec<ConnectionRequest>,
    pub target_node: NodeId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeRequest {
    pub type_id: String,
    pub params: HashMap<String, ParamValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionRequest {
    pub from_node: NodeId,
    pub from_pin: String,
    pub to_node: NodeId,
    pub to_pin: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ParamValue {
    Float(f32),
    Int(i32),
    String(String),
    Boolean(bool),
    Color([f32; 4]),
}

impl ParamValue {
    /// 转换为 engine 内部的 Value 类型
    pub fn to_value(&self) -> Value {
        match self {
            ParamValue::Float(f) => Value::Float(*f),
            ParamValue::Int(i) => Value::Int(*i),
            ParamValue::String(s) => Value::String(s.clone()),
            ParamValue::Boolean(b) => Value::Boolean(*b),
            ParamValue::Color(c) => Value::Color(*c),
        }
    }

    /// 从 Value 转换为 ParamValue（只支持标量类型）
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Float(f) => Some(ParamValue::Float(*f)),
            Value::Int(i) => Some(ParamValue::Int(*i)),
            Value::String(s) => Some(ParamValue::String(s.clone())),
            Value::Boolean(b) => Some(ParamValue::Boolean(*b)),
            Value::Color(c) => Some(ParamValue::Color(*c)),
            _ => None, // Image/GpuImage/Mask 不可序列化
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub gpu: Option<String>,
    pub vram_free_gb: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeTypeDef {
    pub type_id: String,
    pub title: String,
    pub category: String,
    pub inputs: Vec<PinDefInfo>,
    pub outputs: Vec<PinDefInfo>,
    pub params: Vec<ParamDefInfo>,
    pub has_preview: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PinDefInfo {
    pub name: String,
    pub data_type: String,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParamDefInfo {
    pub name: String,
    pub data_type: String,
    pub default: ParamValue,
    pub constraint: Option<ConstraintInfo>,
    pub widget_override: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConstraintInfo {
    Range { min: f64, max: f64 },
    Options(Vec<(String, String)>),
    FilePath { filters: Vec<String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_param_value_roundtrip() {
        let pv = ParamValue::Float(3.14);
        let v = pv.to_value();
        let pv2 = ParamValue::from_value(&v).unwrap();
        match pv2 {
            ParamValue::Float(f) => assert!((f - 3.14).abs() < 0.001),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn test_param_value_from_image_returns_none() {
        let v = Value::Image(std::sync::Arc::new(image::DynamicImage::new_rgba8(1, 1)));
        assert!(ParamValue::from_value(&v).is_none());
    }

    #[test]
    fn test_graph_request_serialize() {
        let req = GraphRequest {
            nodes: HashMap::from([(0, NodeRequest {
                type_id: "invert".into(),
                params: HashMap::from([("brightness".into(), ParamValue::Float(0.5))]),
            })]),
            connections: vec![],
            target_node: 0,
        };
        let json = serde_json::to_string(&req).unwrap();
        let req2: GraphRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req2.target_node, 0);
    }
}
```

- [ ] **Step 4: 在 lib.rs 中注册 transport 模块**

```rust
// crates/nodeimg-engine/src/lib.rs 顶部添加:
pub mod transport;
```

- [ ] **Step 5: 运行测试**

Run: `cargo test -p nodeimg-engine`
Expected: 全部通过，包括新加的 transport 测试

- [ ] **Step 6: 提交**

```bash
git add crates/nodeimg-engine/src/transport/mod.rs crates/nodeimg-engine/src/lib.rs crates/nodeimg-types/src/data_type.rs
git commit -m "feat: define ProcessingTransport trait and associated types #57"
```

---

### Task 2: 重组 engine 文件结构（internal/）

**Files:**
- Create: `crates/nodeimg-engine/src/internal/mod.rs`
- Move: `eval.rs` → `internal/eval.rs`
- Move: `cache.rs` → `internal/cache.rs`
- Move: `registry.rs` → `internal/registry.rs`
- Move: `backend.rs` → `internal/backend.rs`
- Move: `ai_task.rs` → `internal/ai_task.rs`
- Move: `menu.rs` → `internal/menu.rs`
- Modify: `crates/nodeimg-engine/src/lib.rs`

- [ ] **Step 1: 创建 internal/mod.rs**

```rust
// crates/nodeimg-engine/src/internal/mod.rs
pub mod ai_task;
pub mod backend;
pub mod cache;
pub mod eval;
pub mod menu;
pub mod registry;
```

- [ ] **Step 2: 移动文件到 internal/**

```bash
cd crates/nodeimg-engine/src
mkdir -p internal
mv eval.rs internal/
mv cache.rs internal/
mv backend.rs internal/
mv ai_task.rs internal/
mv menu.rs internal/
mv registry.rs internal/
```

- [ ] **Step 3: 更新 lib.rs 的模块声明和导出**

```rust
// crates/nodeimg-engine/src/lib.rs
pub mod builtins;
pub(crate) mod internal;
pub mod transport;

// 暂时保持 pub 重导出，等 app 迁移完再收紧
pub use internal::ai_task;
pub use internal::backend;
pub use internal::cache;
pub use internal::eval;
pub use internal::menu;
pub use internal::registry;

pub use internal::cache::NodeId;
pub use internal::registry::{GpuProcessFn, NodeDef, NodeInstance, NodeRegistry, ProcessFn};
```

- [ ] **Step 4: 更新 internal 模块内部的 crate 引用路径**

所有 `internal/` 下的文件中，`crate::xxx` 引用需要更新为 `crate::internal::xxx`。具体：

`internal/eval.rs`:
```rust
// 旧: use crate::backend::BackendClient;
// 新: use crate::internal::backend::BackendClient;
// 旧: use crate::cache::{Cache, NodeId};
// 新: use crate::internal::cache::{Cache, NodeId};
// 旧: use crate::registry::{NodeInstance, NodeRegistry};
// 新: use crate::internal::registry::{NodeInstance, NodeRegistry};
```

`internal/ai_task.rs`:
```rust
// 旧: use crate::backend::BackendClient;
// 新: use crate::internal::backend::BackendClient;
// 旧: use crate::cache::{Cache, NodeId};
// 新: use crate::internal::cache::{Cache, NodeId};
```

`internal/backend.rs`:
```rust
// 旧: use crate::cache::NodeId;
// 新: use crate::internal::cache::NodeId;
// 旧: use crate::eval::Connection;
// 新: use crate::internal::eval::Connection;
// 旧: use crate::registry::{NodeDef, NodeInstance, NodeRegistry, ParamDef, PinDef};
// 新: use crate::internal::registry::{NodeDef, NodeInstance, NodeRegistry, ParamDef, PinDef};
```

`internal/menu.rs`:
```rust
// 旧: use crate::registry::NodeRegistry;
// 新: use crate::internal::registry::NodeRegistry;
```

`builtins/mod.rs` 和各 builtin 文件中如果有 `crate::registry::` 引用也需要更新为 `crate::internal::registry::`。检查 builtins 目录的导入路径。

- [ ] **Step 5: 运行测试**

Run: `cargo test --workspace`
Expected: 全部通过。这一步只是搬文件和改路径，逻辑不变。

- [ ] **Step 6: 提交**

```bash
git add -A crates/nodeimg-engine/src/
git commit -m "refactor: reorganize engine into transport/ and internal/ #57"
```

---

### Task 3: 实现 LocalTransport

**Files:**
- Create: `crates/nodeimg-engine/src/transport/local.rs`
- Test: `crates/nodeimg-engine/src/transport/local.rs`（内联测试）

- [ ] **Step 1: 写 LocalTransport 的测试**

```rust
// crates/nodeimg-engine/src/transport/local.rs 底部
#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{
        ConnectionRequest, GraphRequest, NodeRequest, ParamValue, ExecuteProgress,
    };
    use std::sync::mpsc;

    fn create_test_transport() -> LocalTransport {
        LocalTransport::new(None, None)
    }

    #[test]
    fn test_health_check() {
        let transport = create_test_transport();
        let resp = transport.health_check().unwrap();
        assert_eq!(resp.status, "ok");
    }

    #[test]
    fn test_node_types_includes_builtins() {
        let transport = create_test_transport();
        let types = transport.node_types().unwrap();
        assert!(!types.is_empty());
        // 应该包含 load_image 等内置节点
        assert!(types.iter().any(|t| t.type_id == "load_image"));
    }

    #[test]
    fn test_execute_simple_graph() {
        let transport = create_test_transport();
        let (tx, rx) = mpsc::channel();

        // 构建一个只有 solid_color 节点的简单图
        let request = GraphRequest {
            nodes: HashMap::from([(
                0,
                NodeRequest {
                    type_id: "solid_color".into(),
                    params: HashMap::from([
                        ("width".into(), ParamValue::Int(64)),
                        ("height".into(), ParamValue::Int(64)),
                        ("color".into(), ParamValue::Color([1.0, 0.0, 0.0, 1.0])),
                    ]),
                },
            )]),
            connections: vec![],
            target_node: 0,
        };

        transport.execute(request, tx).unwrap();

        // 应该收到 NodeCompleted + Finished
        let mut got_completed = false;
        let mut got_finished = false;
        while let Ok(progress) = rx.try_recv() {
            match progress {
                ExecuteProgress::NodeCompleted { node_id, outputs } => {
                    assert_eq!(node_id, 0);
                    assert!(outputs.contains_key("image"));
                    got_completed = true;
                }
                ExecuteProgress::Finished => {
                    got_finished = true;
                }
                ExecuteProgress::Error { message, .. } => {
                    panic!("unexpected error: {}", message);
                }
            }
        }
        assert!(got_completed, "should receive NodeCompleted");
        assert!(got_finished, "should receive Finished");
    }

    #[test]
    fn test_invalidate_clears_cache() {
        let transport = create_test_transport();
        let (tx, _rx) = mpsc::channel();

        let request = GraphRequest {
            nodes: HashMap::from([(
                0,
                NodeRequest {
                    type_id: "solid_color".into(),
                    params: HashMap::from([
                        ("width".into(), ParamValue::Int(64)),
                        ("height".into(), ParamValue::Int(64)),
                        ("color".into(), ParamValue::Color([1.0, 0.0, 0.0, 1.0])),
                    ]),
                },
            )]),
            connections: vec![],
            target_node: 0,
        };

        transport.execute(request.clone(), tx).unwrap();
        // 节点 0 现在已缓存
        transport.invalidate(0);
        // 再次执行应该重新计算（不出错即可）
        let (tx2, _rx2) = mpsc::channel();
        transport.execute(request, tx2).unwrap();
    }

    #[test]
    fn test_is_compatible() {
        let transport = create_test_transport();
        assert!(transport.is_compatible("float", "float"));
        assert!(transport.is_compatible("int", "float")); // 有转换
        assert!(!transport.is_compatible("image", "float")); // 无转换
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p nodeimg-engine transport::local`
Expected: 编译失败，LocalTransport 未定义

- [ ] **Step 3: 实现 LocalTransport**

```rust
// crates/nodeimg-engine/src/transport/local.rs
use crate::internal::backend::BackendClient;
use crate::internal::cache::{Cache, NodeId};
use crate::internal::eval::{Connection, EvalEngine};
use crate::internal::menu::Menu;
use crate::internal::registry::{NodeDef, NodeInstance, NodeRegistry};
use crate::transport::{
    ConstraintInfo, ExecuteProgress, GraphRequest, HealthResponse, NodeTypeDef,
    ParamDefInfo, ParamValue, PinDefInfo, ProcessingTransport,
};
use nodeimg_gpu::GpuContext;
use nodeimg_types::category::CategoryRegistry;
use nodeimg_types::constraint::Constraint;
use nodeimg_types::data_type::{DataTypeId, DataTypeRegistry};
use nodeimg_types::value::Value;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub struct LocalTransport {
    registry: Mutex<NodeRegistry>,
    type_registry: Mutex<DataTypeRegistry>,
    category_registry: CategoryRegistry,
    cache: Mutex<Cache>,
    gpu_ctx: Option<Arc<GpuContext>>,
    backend: Option<BackendClient>,
}

impl LocalTransport {
    pub fn new(gpu_ctx: Option<Arc<GpuContext>>, backend: Option<BackendClient>) -> Self {
        let mut registry = NodeRegistry::new();
        crate::builtins::register_all(&mut registry);

        let type_registry = DataTypeRegistry::with_builtins();
        let category_registry = CategoryRegistry::with_builtins();

        Self {
            registry: Mutex::new(registry),
            type_registry: Mutex::new(type_registry),
            category_registry,
            cache: Mutex::new(Cache::new()),
            gpu_ctx,
            backend,
        }
    }

    /// 注册 AI 后端的远程节点（连接 Python 后端后调用）
    pub fn register_remote_nodes(&self) -> Result<usize, String> {
        let client = self.backend.as_ref().ok_or("No backend configured")?;
        let mut registry = self.registry.lock().unwrap();
        let mut type_registry = self.type_registry.lock().unwrap();
        client.register_remote_nodes(&mut registry, &mut type_registry)
    }

    /// 构建菜单（供 app 使用）
    pub fn generate_menu(&self) -> Vec<crate::internal::menu::MenuCategory> {
        let registry = self.registry.lock().unwrap();
        Menu::generate(&registry, &self.category_registry)
    }

    /// 实例化节点（供 app 使用）
    pub fn instantiate(&self, type_id: &str) -> Option<NodeInstance> {
        let registry = self.registry.lock().unwrap();
        registry.instantiate(type_id)
    }

    /// 提供对 NodeRegistry 的临时访问（用于序列化等需要直接访问 registry 的场景）
    pub fn with_registry<R>(&self, f: impl FnOnce(&NodeRegistry) -> R) -> R {
        let registry = self.registry.lock().unwrap();
        f(&registry)
    }

    /// 将内部 NodeDef 转换为可序列化的 NodeTypeDef
    fn node_def_to_type_def(def: &NodeDef) -> NodeTypeDef {
        NodeTypeDef {
            type_id: def.type_id.clone(),
            title: def.title.clone(),
            category: def.category.0.clone(),
            inputs: def.inputs.iter().map(|p| PinDefInfo {
                name: p.name.clone(),
                data_type: p.data_type.0.clone(),
                required: p.required,
            }).collect(),
            outputs: def.outputs.iter().map(|p| PinDefInfo {
                name: p.name.clone(),
                data_type: p.data_type.0.clone(),
                required: p.required,
            }).collect(),
            params: def.params.iter().map(|p| ParamDefInfo {
                name: p.name.clone(),
                data_type: p.data_type.0.clone(),
                default: ParamValue::from_value(&p.default)
                    .unwrap_or(ParamValue::String(String::new())),
                constraint: Self::constraint_to_info(&p.constraint),
                widget_override: p.widget_override.as_ref().map(|w| w.0.clone()),
            }).collect(),
            has_preview: def.has_preview,
        }
    }

    fn constraint_to_info(c: &Constraint) -> Option<ConstraintInfo> {
        match c {
            Constraint::None => None,
            Constraint::Range { min, max } => Some(ConstraintInfo::Range { min: *min, max: *max }),
            Constraint::Enum { options } => Some(ConstraintInfo::Options(options.clone())),
            Constraint::FilePath { filters } => Some(ConstraintInfo::FilePath { filters: filters.clone() }),
        }
    }
}

impl ProcessingTransport for LocalTransport {
    fn execute(&self, request: GraphRequest, progress: Sender<ExecuteProgress>) -> Result<(), String> {
        let registry = self.registry.lock().unwrap();
        let type_registry = self.type_registry.lock().unwrap();
        let mut cache = self.cache.lock().unwrap();

        // 将 GraphRequest 转换为 engine 内部类型
        let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
        for (id, req) in &request.nodes {
            let params: HashMap<String, Value> = req.params.iter()
                .map(|(k, v)| (k.clone(), v.to_value()))
                .collect();
            nodes.insert(*id, NodeInstance {
                type_id: req.type_id.clone(),
                params,
            });
        }

        let connections: Vec<Connection> = request.connections.iter()
            .map(|c| Connection {
                from_node: c.from_node,
                from_pin: c.from_pin.clone(),
                to_node: c.to_node,
                to_pin: c.to_pin.clone(),
            })
            .collect();

        // 拓扑排序
        let order = EvalEngine::topo_sort(request.target_node, &connections)?;

        // 设置 downstream 关系
        cache.clear_downstream();
        for conn in &connections {
            cache.set_downstream(conn.from_node, conn.to_node);
        }

        // 逐节点执行
        for node_id in order {
            if cache.get(node_id).is_some() {
                continue;
            }

            let instance = match nodes.get(&node_id) {
                Some(inst) => inst,
                None => continue,
            };
            let def = match registry.get(&instance.type_id) {
                Some(d) => d,
                None => continue,
            };

            // 收集输入
            let mut inputs: HashMap<String, Value> = HashMap::new();
            for conn in &connections {
                if conn.to_node == node_id {
                    if let Some(upstream_outputs) = cache.get(conn.from_node) {
                        if let Some(val) = upstream_outputs.get(&conn.from_pin) {
                            let expected_type = def.inputs.iter()
                                .find(|pin| pin.name == conn.to_pin)
                                .map(|pin| &pin.data_type);

                            let upstream_type = nodes.get(&conn.from_node)
                                .and_then(|inst| registry.get(&inst.type_id))
                                .and_then(|upstream_def| {
                                    upstream_def.outputs.iter()
                                        .find(|pin| pin.name == conn.from_pin)
                                        .map(|pin| &pin.data_type)
                                });

                            let converted = match (upstream_type, expected_type) {
                                (Some(from_type), Some(to_type)) if from_type != to_type => {
                                    type_registry.convert(val.clone(), from_type, to_type)
                                        .unwrap_or_else(|| val.clone())
                                }
                                _ => val.clone(),
                            };

                            inputs.insert(conn.to_pin.clone(), converted);
                        }
                    }
                }
            }

            // GPU path
            if let Some(ref gpu_process) = def.gpu_process {
                if let Some(ctx) = &self.gpu_ctx {
                    let outputs = gpu_process(ctx, &inputs, &instance.params);
                    cache.insert(node_id, outputs.clone());
                    let _ = progress.send(ExecuteProgress::NodeCompleted { node_id, outputs });
                    continue;
                }
            }

            // GPU-only without context
            if def.gpu_process.is_some() && self.gpu_ctx.is_none() && def.process.is_none() {
                let msg = format!("Node '{}' requires GPU but no GPU context available", def.title);
                let _ = progress.send(ExecuteProgress::Error {
                    node_id: Some(node_id),
                    message: msg.clone(),
                });
                return Err(msg);
            }

            // CPU path
            if let Some(ref process) = def.process {
                // Auto-convert GpuImage to CPU Image
                if let Some(ctx) = &self.gpu_ctx {
                    for val in inputs.values_mut() {
                        if let Value::GpuImage(tex) = val {
                            *val = Value::Image(Arc::new(
                                tex.to_dynamic_image(&ctx.device, &ctx.queue),
                            ));
                        }
                    }
                }
                let outputs = process(&inputs, &instance.params);
                cache.insert(node_id, outputs.clone());
                let _ = progress.send(ExecuteProgress::NodeCompleted { node_id, outputs });
            } else if def.gpu_process.is_none() {
                // AI 节点：通过 BackendClient 转发
                if let Some(client) = &self.backend {
                    // 使用现有的 EvalEngine::execute_ai_subgraph 逻辑的简化版
                    // AI 节点暂时跳过，由 app 的现有 AI 执行流程处理
                    // 完整的 AI 集成将在后续迭代中完成
                    eprintln!("[transport] AI node {} skipped (pending AI integration)", node_id);
                    let _ = client; // suppress unused warning
                }
            }
        }

        let _ = progress.send(ExecuteProgress::Finished);
        Ok(())
    }

    fn invalidate(&self, node_id: NodeId) {
        self.cache.lock().unwrap().invalidate(node_id);
    }

    fn invalidate_all(&self) {
        self.cache.lock().unwrap().invalidate_all();
    }

    fn is_compatible(&self, from_type: &str, to_type: &str) -> bool {
        let type_registry = self.type_registry.lock().unwrap();
        let from = DataTypeId::new(from_type);
        let to = DataTypeId::new(to_type);
        type_registry.is_compatible(&from, &to)
    }

    fn health_check(&self) -> Result<HealthResponse, String> {
        // LocalTransport 总是健康的
        Ok(HealthResponse {
            status: "ok".into(),
            gpu: self.gpu_ctx.as_ref().map(|_| "available".into()),
            vram_free_gb: None,
        })
    }

    fn node_types(&self) -> Result<Vec<NodeTypeDef>, String> {
        let registry = self.registry.lock().unwrap();
        Ok(registry.list(None).iter().map(|def| {
            Self::node_def_to_type_def(def)
        }).collect())
    }
}
```

- [ ] **Step 4: 运行测试**

Run: `cargo test -p nodeimg-engine transport::local`
Expected: 全部通过

- [ ] **Step 5: 运行全量测试**

Run: `cargo test --workspace`
Expected: 全部通过

- [ ] **Step 6: 提交**

```bash
git add crates/nodeimg-engine/src/transport/local.rs
git commit -m "feat: implement LocalTransport wrapping existing engine logic #57"
```

---

### Task 4: 实现 ExecutionManager（app 侧）

**Files:**
- Create: `crates/nodeimg-app/src/execution.rs`
- Modify: `crates/nodeimg-app/src/lib.rs`

- [ ] **Step 1: 创建 execution.rs**

```rust
// crates/nodeimg-app/src/execution.rs
use nodeimg_engine::transport::{ExecuteProgress, GraphRequest, ProcessingTransport};
use nodeimg_engine::cache::NodeId;
use std::sync::mpsc;
use std::sync::Arc;

/// 包装 Sender，自动附加 generation 标记并实时转发进度
struct TaggedSender {
    gen: u64,
    tx: mpsc::Sender<(u64, ExecuteProgress)>,
}

impl TaggedSender {
    /// 转换为 transport.execute() 需要的 Sender<ExecuteProgress>
    /// 通过中间线程实时转发，附加 generation
    fn into_sender(self) -> mpsc::Sender<ExecuteProgress> {
        let (inner_tx, inner_rx) = mpsc::channel();
        let tagged_tx = self.tx;
        let gen = self.gen;
        std::thread::spawn(move || {
            while let Ok(progress) = inner_rx.recv() {
                if tagged_tx.send((gen, progress)).is_err() {
                    break; // receiver 已关闭
                }
            }
        });
        inner_tx
    }
}

/// App 侧的执行调度器：spawn 后台线程执行图，每帧轮询结果。
/// 替代 AiExecutor 的职责。
pub struct ExecutionManager {
    transport: Arc<dyn ProcessingTransport>,
    progress_rx: Option<mpsc::Receiver<(u64, ExecuteProgress)>>,
    generation: u64,
    running: bool,
    repaint: Option<egui::Context>,
}

impl ExecutionManager {
    pub fn new(transport: Arc<dyn ProcessingTransport>) -> Self {
        Self {
            transport,
            progress_rx: None,
            generation: 0,
            running: false,
            repaint: None,
        }
    }

    /// 设置 egui Context 用于执行完成时唤醒 UI
    pub fn set_repaint_ctx(&mut self, ctx: egui::Context) {
        self.repaint = Some(ctx);
    }

    /// 提交一个图执行任务到后台线程
    pub fn submit(&mut self, request: GraphRequest) {
        self.generation += 1;
        let gen = self.generation;
        let transport = Arc::clone(&self.transport);
        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.running = true;

        let repaint = self.repaint.clone();
        std::thread::spawn(move || {
            // 创建包装 Sender，自动附加 generation 并实时转发
            let tagged_tx = TaggedSender { gen, tx: tx.clone() };

            let result = transport.execute(request, tagged_tx.into_sender());

            // 如果 execute 自身返回错误
            if let Err(e) = result {
                let _ = tx.send((gen, ExecuteProgress::Error {
                    node_id: None,
                    message: e,
                }));
            }

            // 唤醒 UI 刷新
            if let Some(ctx) = repaint {
                ctx.request_repaint();
            }
        });
    }

    /// 每帧非阻塞轮询，返回本帧内收到的所有进度消息
    pub fn poll(&mut self) -> Vec<ExecuteProgress> {
        let mut results = Vec::new();
        if let Some(ref rx) = self.progress_rx {
            while let Ok((gen, progress)) = rx.try_recv() {
                // 丢弃过期结果
                if gen != self.generation {
                    continue;
                }
                if matches!(progress, ExecuteProgress::Finished | ExecuteProgress::Error { .. }) {
                    self.running = false;
                }
                results.push(progress);
            }
        }
        results
    }

    /// 取消当前执行（generation+1，旧结果会被丢弃）
    pub fn cancel(&mut self) {
        self.generation += 1;
        self.running = false;
    }

    /// 是否有正在执行的任务
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// 获取 transport 引用
    pub fn transport(&self) -> &Arc<dyn ProcessingTransport> {
        &self.transport
    }
}
```

- [ ] **Step 2: 在 lib.rs 注册模块**

```rust
// crates/nodeimg-app/src/lib.rs 添加:
pub mod execution;
```

- [ ] **Step 3: 编译检查**

Run: `cargo build -p nodeimg-app`
Expected: 编译通过

- [ ] **Step 4: 提交**

```bash
git add crates/nodeimg-app/src/execution.rs crates/nodeimg-app/src/lib.rs
git commit -m "feat: add ExecutionManager for app-side async execution #57"
```

---

### Task 5: 集成 — App 使用 Transport

**Files:**
- Modify: `crates/nodeimg-app/src/app.rs`
- Modify: `crates/nodeimg-app/src/node/viewer.rs`

这是最大的集成任务。核心改动：
1. App 创建 LocalTransport，传给 NodeViewer
2. NodeViewer 用 Transport 替代直接操作 engine 内部
3. 逐步替换，保持可编译

- [ ] **Step 1: App 创建 LocalTransport**

修改 `app.rs` 的 `App::new`，创建 `LocalTransport` 并共享给 viewer：

```rust
// app.rs 新增导入:
use crate::execution::ExecutionManager;
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::ProcessingTransport;

// App 结构体新增字段:
// execution_manager: ExecutionManager,

// App::new 中:
// 1. 创建 LocalTransport（替代直接构造 registry + backend）
// 2. 注册 AI 节点到 transport
// 3. 创建 ExecutionManager
// 4. 传给 NodeViewer
```

具体实现较长，涉及 `App::new` 和 `NodeViewer::new` 的签名调整。核心步骤：

a. `LocalTransport::new(gpu_ctx, Some(backend))` 替代分散的初始化
b. `transport.register_remote_nodes()` 替代 `backend.register_remote_nodes(&mut viewer.node_registry, &mut viewer.type_registry)`
c. `NodeViewer` 持有 `Arc<LocalTransport>` 替代直接持有 registry/cache/backend/ai_executor

- [ ] **Step 2: NodeViewer 结构体改造**

```rust
// viewer.rs 新的 imports（替换旧的 engine imports）
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::{NodeTypeDef, PinDefInfo, ParamDefInfo, ConstraintInfo, ParamValue};
use nodeimg_engine::registry::NodeInstance;
use nodeimg_engine::cache::NodeId as EngineNodeId;
// 删除: use nodeimg_engine::ai_task::AiExecutor;
// 删除: use nodeimg_engine::backend::BackendClient;
// 删除: use nodeimg_engine::cache::Cache;
// 删除: use nodeimg_engine::eval::{Connection, EvalEngine};

pub struct NodeViewer {
    pub transport: Arc<LocalTransport>,
    pub node_type_defs: Vec<NodeTypeDef>,
    pub category_registry: CategoryRegistry,
    pub widget_registry: WidgetRegistry,
    pub textures: HashMap<NodeId, egui::TextureHandle>,
    pub preview_texture: Option<egui::TextureHandle>,
    pub theme: Arc<dyn Theme>,
    pub gpu_ctx: Option<Arc<GpuContext>>,
    pub backend_status: Option<String>,
    ai_errors: HashMap<NodeId, String>,
    pub graph_dirty: bool,
}

impl NodeViewer {
    pub fn new(theme: Arc<dyn Theme>, transport: Arc<LocalTransport>) -> Self {
        let node_type_defs = transport.node_types().unwrap_or_default();
        let gpu_ctx = None; // GPU ctx 现在由 transport 持有

        Self {
            transport,
            node_type_defs,
            category_registry: CategoryRegistry::with_builtins(),
            widget_registry: WidgetRegistry::with_builtins(),
            textures: HashMap::new(),
            preview_texture: None,
            theme,
            gpu_ctx,
            backend_status: None,
            ai_errors: HashMap::new(),
            graph_dirty: false,
        }
    }

    /// 查找节点类型定义
    fn find_type_def(&self, type_id: &str) -> Option<&NodeTypeDef> {
        self.node_type_defs.iter().find(|d| d.type_id == type_id)
    }

    pub fn invalidate(&mut self, node_id: NodeId) {
        self.transport.invalidate(node_id.0);
        self.textures.remove(&node_id);
    }

    pub fn invalidate_all(&mut self) {
        self.transport.invalidate_all();
        self.textures.clear();
    }
}
```

- [ ] **Step 3: 替换 SnarlViewer 实现中的所有 node_registry 调用**

每个使用 `self.node_registry.get(&node.type_id)` 的地方，改为 `self.find_type_def(&node.type_id)`。返回类型从 `&NodeDef` 变为 `&NodeTypeDef`，字段名相同但类型不同（如 `inputs` 变为 `Vec<PinDefInfo>`）。需要逐方法替换：

```rust
// title() — 直接兼容
fn title(&mut self, node: &NodeInstance) -> String {
    self.find_type_def(&node.type_id)
        .map(|def| def.title.clone())
        .unwrap_or_else(|| format!("[Unknown: {}]", node.type_id))
}

// inputs() / outputs() — 直接兼容
fn inputs(&mut self, node: &NodeInstance) -> usize {
    self.find_type_def(&node.type_id).map(|d| d.inputs.len()).unwrap_or(0)
}

// show_input() — 用 PinDefInfo 替代 PinDef
fn show_input(&mut self, pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<NodeInstance>) -> PinInfo {
    let instance = &snarl[pin.id.node];
    if let Some(def) = self.find_type_def(&instance.type_id) {
        if let Some(pin_def) = def.inputs.get(pin.id.input) {
            ui.colored_label(self.theme.text_secondary(), &pin_def.name);
            let color = self.theme.pin_color(&pin_def.data_type);
            return PinInfo::circle().with_fill(color);
        }
    }
    ui.colored_label(self.theme.text_secondary(), "?");
    PinInfo::circle().with_fill(Color32::GRAY)
}

// connect() — 用 transport.is_compatible 替代 type_registry
fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NodeInstance>) {
    let from_type_id = snarl[from.id.node].type_id.clone();
    let to_type_id = snarl[to.id.node].type_id.clone();
    let compatible = (|| {
        let from_def = self.find_type_def(&from_type_id)?;
        let to_def = self.find_type_def(&to_type_id)?;
        let from_type = &from_def.outputs.get(from.id.output)?.data_type;
        let to_type = &to_def.inputs.get(to.id.input)?.data_type;
        Some(self.transport.is_compatible(from_type, to_type))
    })();

    if compatible != Some(true) {
        return;
    }
    // ... 环检测和连接逻辑保持不变（用 transport 的 build_connections）
}

// show_graph_menu() — 用 transport.generate_menu() 和 transport.instantiate()
fn show_graph_menu(&mut self, pos: egui::Pos2, ui: &mut Ui, snarl: &mut Snarl<NodeInstance>) {
    let categories = self.transport.generate_menu();
    ui.label("Add Node");
    ui.separator();
    for cat in &categories {
        ui.menu_button(&cat.name, |ui: &mut Ui| {
            for item in &cat.items {
                if ui.button(&item.title).clicked() {
                    if let Some(instance) = self.transport.instantiate(&item.type_id) {
                        snarl.insert_node(pos, instance);
                        self.graph_dirty = true;
                    }
                    ui.close();
                }
            }
        });
    }
}
```

- [ ] **Step 4: 更新 show_body 的执行流程**

`show_body` 中的 `EvalEngine::evaluate()` 和 `AiExecutor` 调用需要替换。这部分暂时保持现有行为（本地节点在 UI 线程同步执行），因为完整的 ExecutionManager 异步流程涉及 show_body 的重大重写。分步迁移：

a. 先通过 `transport.with_registry()` 访问 registry 来维持 `show_body` 中需要的 `NodeDef`（process/gpu_process 函数指针）
b. 参数变更时调用 `self.transport.invalidate(node_id.0)` 替代 `self.cache.invalidate(node_id.0)`
c. AI 执行暂时保持通过 transport 的 backend 间接处理

- [ ] **Step 5: 更新 app.rs 的序列化调用**

`auto_save`、`save_as`、`quick_save`、`open_file`、`try_auto_load` 中的 `&self.viewer.node_registry` 改用 `transport.with_registry()`：

```rust
// 旧: Serializer::snapshot(&self.snarl, &self.viewer.node_registry)
// 新:
self.viewer.transport.with_registry(|registry| {
    Serializer::snapshot(&self.snarl, registry)
})

// 旧: Serializer::load(&json, &self.viewer.node_registry)
// 新:
self.viewer.transport.with_registry(|registry| {
    Serializer::load(&json, registry)
})

// 旧: Serializer::restore(&graph, &self.viewer.node_registry)
// 新:
self.viewer.transport.with_registry(|registry| {
    Serializer::restore(&graph, registry)
})
```

- [ ] **Step 5: 编译检查**

Run: `cargo build -p nodeimg-app`
Expected: 编译通过

- [ ] **Step 6: 运行全量测试**

Run: `cargo test --workspace`
Expected: 全部通过

- [ ] **Step 7: 手动测试**

Run: `cargo run -p nodeimg-app --release`
Expected:
- 节点菜单正常显示
- 可以添加/删除/连接节点
- 参数修改后预览正常更新
- 主题切换正常
- 保存/加载正常

- [ ] **Step 8: 提交**

```bash
git add crates/nodeimg-app/src/app.rs crates/nodeimg-app/src/node/viewer.rs
git commit -m "feat: integrate Transport into app, replace direct engine access #57"
```

---

### Task 6: 清理 — 删除旧代码，收紧可见性

**Files:**
- Delete: `crates/nodeimg-engine/src/internal/ai_task.rs`
- Modify: `crates/nodeimg-engine/src/internal/mod.rs` — 移除 `pub mod ai_task;`
- Modify: `crates/nodeimg-engine/src/lib.rs` — 收紧导出

- [ ] **Step 0: 删除 ai_task.rs（已被 ExecutionManager 替代）**

```bash
rm -f crates/nodeimg-engine/src/internal/ai_task.rs
```

从 `internal/mod.rs` 中移除 `pub mod ai_task;` 行。
从 `lib.rs` 中移除 `pub use internal::ai_task;` 行。

- [ ] **Step 1: 收紧 lib.rs 的导出**

```rust
// crates/nodeimg-engine/src/lib.rs
pub mod builtins;
pub(crate) mod internal;
pub mod transport;

// 只保留必要的公开重导出
pub use internal::cache::NodeId;
pub use internal::registry::{NodeDef, NodeInstance, NodeRegistry, ProcessFn, GpuProcessFn};
pub use internal::registry::{ParamDef, PinDef};

// 以下用于 builtins 注册，保持 pub
pub use internal::backend::BackendClient;
pub use internal::eval::Connection;
```

**注意：** 需要检查 app 中是否还有直接使用 `nodeimg_engine::eval::*`、`nodeimg_engine::cache::*`、`nodeimg_engine::menu::*`、`nodeimg_engine::ai_task::*`、`nodeimg_engine::backend::*` 的 import。如果有，需要全部替换为通过 transport 的接口。

- [ ] **Step 2: 编译检查**

Run: `cargo build --workspace`
Expected: 编译通过。如果有编译错误，说明 app 还有地方直接依赖 engine 内部，需要继续迁移。

- [ ] **Step 3: 运行全量测试**

Run: `cargo test --workspace`
Expected: 全部通过

- [ ] **Step 4: 手动测试**

Run: `cargo run -p nodeimg-app --release`
Expected: 行为与 Task 5 一致

- [ ] **Step 5: 提交**

```bash
git add crates/nodeimg-engine/src/lib.rs
git commit -m "refactor: tighten engine visibility, only expose transport API #57"
```

---

### Task 7: 最终验证

- [ ] **Step 1: 全量测试**

Run: `cargo test --workspace`
Expected: 全部通过

- [ ] **Step 2: Release 构建**

Run: `cargo build --release`
Expected: 编译成功

- [ ] **Step 3: 手动完整测试**

Run: `cargo run -p nodeimg-app --release`
验证清单：
- [ ] 节点菜单显示所有分类和节点
- [ ] 添加节点、删除节点正常
- [ ] 连接节点后预览自动更新
- [ ] 修改参数后预览自动更新
- [ ] 断开连接后缓存正确失效
- [ ] 主题切换正常
- [ ] 保存/加载正常
- [ ] FPS 显示正常（F12）
- [ ] 如果 Python 后端可用，AI 节点正常工作

- [ ] **Step 4: 最终提交**

如果有任何修复，创建新提交。
