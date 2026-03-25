use crate::builtins;
use crate::internal::backend::BackendClient;
use crate::internal::cache::{Cache, NodeId};
use crate::internal::eval::{Connection, EvalEngine};
use crate::internal::menu::{Menu, MenuCategory};
use crate::internal::registry::{NodeDef, NodeInstance, NodeRegistry};
use crate::transport::{
    ConstraintInfo, ExecuteProgress, GraphRequest, HealthResponse, NodeTypeDef, ParamDefInfo,
    ParamValue, PinDefInfo, ProcessingTransport,
};
use nodeimg_gpu::GpuContext;
use nodeimg_types::category::CategoryRegistry;
use nodeimg_types::constraint::Constraint;
use nodeimg_types::data_type::{DataTypeId, DataTypeRegistry};
use nodeimg_types::value::Value;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

/// Local transport: executes node graphs in-process using the built-in engine.
///
/// Wraps `NodeRegistry`, `DataTypeRegistry`, `Cache`, and optional GPU/backend
/// behind `Mutex` guards so the struct can be shared across threads.
pub struct LocalTransport {
    registry: Mutex<NodeRegistry>,
    type_registry: Mutex<DataTypeRegistry>,
    cache: Mutex<Cache>,
    category_registry: CategoryRegistry,
    gpu_ctx: Option<Arc<GpuContext>>,
    #[allow(dead_code)] // Will be used when AI backend integration is added
    backend: Option<BackendClient>,
}

impl LocalTransport {
    /// Create a new `LocalTransport`, registering all built-in nodes.
    pub fn new(gpu_ctx: Option<Arc<GpuContext>>, backend: Option<BackendClient>) -> Self {
        let mut registry = NodeRegistry::new();
        builtins::register_all(&mut registry);

        let type_registry = DataTypeRegistry::with_builtins();
        let category_registry = CategoryRegistry::with_builtins();
        let cache = Cache::new();

        Self {
            registry: Mutex::new(registry),
            type_registry: Mutex::new(type_registry),
            cache: Mutex::new(cache),
            category_registry,
            gpu_ctx,
            backend,
        }
    }

    /// Access the node registry (locked). Useful for registering additional
    /// nodes (e.g. AI nodes from backend) or for app-layer queries.
    pub fn with_registry<R>(&self, f: impl FnOnce(&mut NodeRegistry) -> R) -> R {
        let mut reg = self.registry.lock().unwrap();
        f(&mut reg)
    }

    /// Access the data type registry (locked).
    pub fn with_type_registry<R>(&self, f: impl FnOnce(&mut DataTypeRegistry) -> R) -> R {
        let mut reg = self.type_registry.lock().unwrap();
        f(&mut reg)
    }

    /// Generate a categorised menu for the node picker UI.
    pub fn generate_menu(&self) -> Vec<MenuCategory> {
        let reg = self.registry.lock().unwrap();
        Menu::generate(&reg, &self.category_registry)
    }

    /// Create a new `NodeInstance` with default parameter values.
    pub fn instantiate(&self, type_id: &str) -> Option<NodeInstance> {
        let reg = self.registry.lock().unwrap();
        reg.instantiate(type_id)
    }

    // -- internal helpers --

    /// Convert a `NodeDef` to the serialisable `NodeTypeDef`.
    fn node_def_to_type_def(def: &NodeDef) -> NodeTypeDef {
        NodeTypeDef {
            type_id: def.type_id.clone(),
            title: def.title.clone(),
            category: def.category.0.clone(),
            has_preview: def.has_preview,
            inputs: def
                .inputs
                .iter()
                .map(|pin| PinDefInfo {
                    name: pin.name.clone(),
                    data_type: pin.data_type.0.clone(),
                    required: pin.required,
                })
                .collect(),
            outputs: def
                .outputs
                .iter()
                .map(|pin| PinDefInfo {
                    name: pin.name.clone(),
                    data_type: pin.data_type.0.clone(),
                    required: pin.required,
                })
                .collect(),
            params: def
                .params
                .iter()
                .map(|p| ParamDefInfo {
                    name: p.name.clone(),
                    data_type: p.data_type.0.clone(),
                    default: ParamValue::from_value(&p.default)
                        .unwrap_or(ParamValue::String(String::new())),
                    constraint: Self::constraint_to_info(&p.constraint),
                    widget_override: p.widget_override.as_ref().map(|w| w.0.clone()),
                })
                .collect(),
        }
    }

    /// Map internal `Constraint` to the serialisable `ConstraintInfo`.
    fn constraint_to_info(constraint: &Constraint) -> Option<ConstraintInfo> {
        match constraint {
            Constraint::None => None,
            Constraint::Range { min, max } => Some(ConstraintInfo::Range {
                min: *min,
                max: *max,
            }),
            Constraint::Enum { options } => Some(ConstraintInfo::Options(options.clone())),
            Constraint::FilePath { filters } => Some(ConstraintInfo::FilePath {
                filters: filters.clone(),
            }),
        }
    }
}

impl ProcessingTransport for LocalTransport {
    fn execute(
        &self,
        request: GraphRequest,
        progress: Sender<ExecuteProgress>,
    ) -> Result<(), String> {
        // 1. Convert GraphRequest to internal types
        let registry = self.registry.lock().unwrap();
        let type_registry = self.type_registry.lock().unwrap();
        let mut cache = self.cache.lock().unwrap();

        let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
        for (&node_id, req) in &request.nodes {
            let mut params: HashMap<String, Value> = HashMap::new();

            // Start with defaults from the NodeDef
            if let Some(def) = registry.get(&req.type_id) {
                for p in &def.params {
                    params.insert(p.name.clone(), p.default.clone());
                }
            }

            // Override with request params
            for (key, pv) in &req.params {
                params.insert(key.clone(), pv.to_value());
            }

            nodes.insert(
                node_id,
                NodeInstance {
                    type_id: req.type_id.clone(),
                    params,
                },
            );
        }

        let connections: Vec<Connection> = request
            .connections
            .iter()
            .map(|c| Connection {
                from_node: c.from_node,
                from_pin: c.from_pin.clone(),
                to_node: c.to_node,
                to_pin: c.to_pin.clone(),
            })
            .collect();

        // 2. Topological sort
        let order = EvalEngine::topo_sort(request.target_node, &connections)?;

        // 3. Set up downstream relationships in cache
        cache.clear_downstream();
        for conn in &connections {
            cache.set_downstream(conn.from_node, conn.to_node);
        }

        // 4. Evaluate each node in order
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

            // Gather inputs from upstream cached outputs
            let mut inputs: HashMap<String, Value> = HashMap::new();
            for conn in &connections {
                if conn.to_node == node_id {
                    if let Some(upstream_outputs) = cache.get(conn.from_node) {
                        if let Some(val) = upstream_outputs.get(&conn.from_pin) {
                            // Type conversion if needed
                            let expected_type = def
                                .inputs
                                .iter()
                                .find(|pin| pin.name == conn.to_pin)
                                .map(|pin| &pin.data_type);

                            let upstream_type = nodes
                                .get(&conn.from_node)
                                .and_then(|inst| registry.get(&inst.type_id))
                                .and_then(|upstream_def| {
                                    upstream_def
                                        .outputs
                                        .iter()
                                        .find(|pin| pin.name == conn.from_pin)
                                        .map(|pin| &pin.data_type)
                                });

                            let converted = match (upstream_type, expected_type) {
                                (Some(from_type), Some(to_type)) if from_type != to_type => {
                                    type_registry
                                        .convert(val.clone(), from_type, to_type)
                                        .unwrap_or_else(|| val.clone())
                                }
                                _ => val.clone(),
                            };

                            inputs.insert(conn.to_pin.clone(), converted);
                        }
                    }
                }
            }

            // GPU path (highest priority)
            if let Some(ref gpu_process) = def.gpu_process {
                if let Some(ctx) = &self.gpu_ctx {
                    eprintln!("[local] node {} ({}) -> GPU", node_id, def.title);
                    let outputs = gpu_process(ctx, &inputs, &instance.params);
                    let _ = progress.send(ExecuteProgress::NodeCompleted {
                        node_id,
                        outputs: outputs.clone(),
                    });
                    cache.insert(node_id, outputs);
                    continue;
                }
            }

            // GPU-only nodes without GPU context: error
            if def.gpu_process.is_some() && self.gpu_ctx.is_none() && def.process.is_none() {
                let msg = format!(
                    "Node '{}' requires GPU but no GPU context available",
                    def.title
                );
                let _ = progress.send(ExecuteProgress::Error {
                    node_id: Some(node_id),
                    message: msg.clone(),
                });
                return Err(msg);
            }

            // CPU path
            if let Some(ref process) = def.process {
                // Auto-convert GpuImage inputs to CPU Image via readback
                if let Some(ctx) = &self.gpu_ctx {
                    for val in inputs.values_mut() {
                        if let Value::GpuImage(tex) = val {
                            *val = Value::Image(std::sync::Arc::new(
                                tex.to_dynamic_image(&ctx.device, &ctx.queue),
                            ));
                        }
                    }
                }
                eprintln!("[local] node {} ({}) -> CPU", node_id, def.title);
                let outputs = process(&inputs, &instance.params);
                let _ = progress.send(ExecuteProgress::NodeCompleted {
                    node_id,
                    outputs: outputs.clone(),
                });
                cache.insert(node_id, outputs);
            } else if def.gpu_process.is_none() {
                // AI node: skip with log (full AI integration is future work)
                eprintln!(
                    "[local] node {} ({}) is an AI node — skipped",
                    node_id, def.title
                );
            }
        }

        let _ = progress.send(ExecuteProgress::Finished);
        Ok(())
    }

    fn invalidate(&self, node_id: NodeId) {
        let mut cache = self.cache.lock().unwrap();
        cache.invalidate(node_id);
    }

    fn invalidate_all(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.invalidate_all();
    }

    fn is_compatible(&self, from_type: &str, to_type: &str) -> bool {
        let type_reg = self.type_registry.lock().unwrap();
        type_reg.is_compatible(&DataTypeId::new(from_type), &DataTypeId::new(to_type))
    }

    fn health_check(&self) -> Result<HealthResponse, String> {
        Ok(HealthResponse {
            status: "ok".into(),
            gpu: self.gpu_ctx.as_ref().map(|_| "local".into()),
            vram_free_gb: None,
        })
    }

    fn node_types(&self) -> Result<Vec<NodeTypeDef>, String> {
        let reg = self.registry.lock().unwrap();
        let defs: Vec<NodeTypeDef> = reg
            .list(None)
            .iter()
            .map(|def| Self::node_def_to_type_def(def))
            .collect();
        Ok(defs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::registry::{NodeDef, ParamDef, PinDef};
    use crate::transport::{ConnectionRequest, GraphRequest, NodeRequest, ParamValue};
    use nodeimg_types::category::CategoryId;
    use nodeimg_types::constraint::Constraint;
    use nodeimg_types::data_type::DataTypeId;
    use std::sync::mpsc;

    fn create_test_transport() -> LocalTransport {
        LocalTransport::new(None, None)
    }

    /// Register a simple CPU-only "source" node that outputs a float from params.
    fn register_test_source(transport: &LocalTransport) {
        transport.with_registry(|reg| {
            reg.register(NodeDef {
                type_id: "test_source".into(),
                title: "Test Source".into(),
                category: CategoryId::new("tool"),
                inputs: vec![],
                outputs: vec![PinDef {
                    name: "value".into(),
                    data_type: DataTypeId::new("float"),
                    required: false,
                }],
                params: vec![ParamDef {
                    name: "value".into(),
                    data_type: DataTypeId::new("float"),
                    constraint: Constraint::None,
                    default: Value::Float(0.0),
                    widget_override: None,
                }],
                has_preview: false,
                process: Some(Box::new(|_inputs, params| {
                    let mut out = HashMap::new();
                    if let Some(v) = params.get("value") {
                        out.insert("value".into(), v.clone());
                    }
                    out
                })),
                gpu_process: None,
            });
        });
    }

    /// Register a CPU-only "add_one" node: takes float input, outputs float + 1.
    fn register_test_add_one(transport: &LocalTransport) {
        transport.with_registry(|reg| {
            reg.register(NodeDef {
                type_id: "test_add_one".into(),
                title: "Test Add One".into(),
                category: CategoryId::new("tool"),
                inputs: vec![PinDef {
                    name: "value".into(),
                    data_type: DataTypeId::new("float"),
                    required: true,
                }],
                outputs: vec![PinDef {
                    name: "result".into(),
                    data_type: DataTypeId::new("float"),
                    required: false,
                }],
                params: vec![],
                has_preview: false,
                process: Some(Box::new(|inputs, _params| {
                    let mut out = HashMap::new();
                    if let Some(Value::Float(v)) = inputs.get("value") {
                        out.insert("result".into(), Value::Float(v + 1.0));
                    }
                    out
                })),
                gpu_process: None,
            });
        });
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
        assert!(types.iter().any(|t| t.type_id == "load_image"));
    }

    #[test]
    fn test_is_compatible() {
        let transport = create_test_transport();
        assert!(transport.is_compatible("float", "float"));
        assert!(transport.is_compatible("int", "float"));
        assert!(!transport.is_compatible("image", "float"));
    }

    #[test]
    fn test_execute_simple_graph() {
        let transport = create_test_transport();
        register_test_source(&transport);

        let (tx, rx) = mpsc::channel();

        let request = GraphRequest {
            nodes: HashMap::from([(
                0,
                NodeRequest {
                    type_id: "test_source".into(),
                    params: HashMap::from([("value".into(), ParamValue::Float(42.0))]),
                },
            )]),
            connections: vec![],
            target_node: 0,
        };

        transport.execute(request, tx).unwrap();

        let mut got_completed = false;
        let mut got_finished = false;
        while let Ok(progress) = rx.try_recv() {
            match progress {
                ExecuteProgress::NodeCompleted { node_id, outputs } => {
                    assert_eq!(node_id, 0);
                    match outputs.get("value") {
                        Some(Value::Float(v)) => assert_eq!(*v, 42.0),
                        other => panic!("expected Float(42.0), got {:?}", other),
                    }
                    got_completed = true;
                }
                ExecuteProgress::Finished => got_finished = true,
                ExecuteProgress::Error { message, .. } => panic!("unexpected error: {}", message),
            }
        }
        assert!(got_completed);
        assert!(got_finished);
    }

    #[test]
    fn test_execute_connected_graph() {
        let transport = create_test_transport();
        register_test_source(&transport);
        register_test_add_one(&transport);

        let (tx, rx) = mpsc::channel();

        let request = GraphRequest {
            nodes: HashMap::from([
                (
                    0,
                    NodeRequest {
                        type_id: "test_source".into(),
                        params: HashMap::from([("value".into(), ParamValue::Float(5.0))]),
                    },
                ),
                (
                    1,
                    NodeRequest {
                        type_id: "test_add_one".into(),
                        params: HashMap::new(),
                    },
                ),
            ]),
            connections: vec![ConnectionRequest {
                from_node: 0,
                from_pin: "value".into(),
                to_node: 1,
                to_pin: "value".into(),
            }],
            target_node: 1,
        };

        transport.execute(request, tx).unwrap();

        let mut completed_nodes = Vec::new();
        let mut got_finished = false;
        while let Ok(progress) = rx.try_recv() {
            match progress {
                ExecuteProgress::NodeCompleted { node_id, outputs } => {
                    if node_id == 1 {
                        match outputs.get("result") {
                            Some(Value::Float(v)) => assert_eq!(*v, 6.0),
                            other => panic!("expected Float(6.0), got {:?}", other),
                        }
                    }
                    completed_nodes.push(node_id);
                }
                ExecuteProgress::Finished => got_finished = true,
                ExecuteProgress::Error { message, .. } => panic!("unexpected error: {}", message),
            }
        }
        assert!(completed_nodes.contains(&0));
        assert!(completed_nodes.contains(&1));
        assert!(got_finished);
    }

    #[test]
    fn test_invalidate() {
        let transport = create_test_transport();
        register_test_source(&transport);

        let request = GraphRequest {
            nodes: HashMap::from([(
                0,
                NodeRequest {
                    type_id: "test_source".into(),
                    params: HashMap::from([("value".into(), ParamValue::Float(1.0))]),
                },
            )]),
            connections: vec![],
            target_node: 0,
        };

        let (tx, _) = mpsc::channel();
        transport.execute(request.clone(), tx).unwrap();

        // Invalidate and re-execute — should succeed without error
        transport.invalidate(0);

        let (tx2, rx2) = mpsc::channel();
        transport.execute(request, tx2).unwrap();

        let mut got_completed = false;
        while let Ok(progress) = rx2.try_recv() {
            match progress {
                ExecuteProgress::NodeCompleted { node_id, .. } => {
                    assert_eq!(node_id, 0);
                    got_completed = true;
                }
                ExecuteProgress::Finished => {}
                ExecuteProgress::Error { message, .. } => panic!("unexpected error: {}", message),
            }
        }
        assert!(got_completed, "node should re-execute after invalidation");
    }

    #[test]
    fn test_invalidate_all() {
        let transport = create_test_transport();
        register_test_source(&transport);

        let request = GraphRequest {
            nodes: HashMap::from([(
                0,
                NodeRequest {
                    type_id: "test_source".into(),
                    params: HashMap::from([("value".into(), ParamValue::Float(1.0))]),
                },
            )]),
            connections: vec![],
            target_node: 0,
        };

        let (tx, _) = mpsc::channel();
        transport.execute(request.clone(), tx).unwrap();

        transport.invalidate_all();

        let (tx2, rx2) = mpsc::channel();
        transport.execute(request, tx2).unwrap();

        let mut got_completed = false;
        while let Ok(progress) = rx2.try_recv() {
            if let ExecuteProgress::NodeCompleted { .. } = progress {
                got_completed = true;
            }
        }
        assert!(got_completed, "all nodes should re-execute after invalidate_all");
    }

    #[test]
    fn test_generate_menu() {
        let transport = create_test_transport();
        let menu = transport.generate_menu();
        // Builtins register nodes in several categories
        assert!(!menu.is_empty());
    }

    #[test]
    fn test_instantiate() {
        let transport = create_test_transport();
        let inst = transport.instantiate("load_image");
        assert!(inst.is_some());

        let inst = transport.instantiate("nonexistent_node");
        assert!(inst.is_none());
    }

    #[test]
    fn test_node_def_to_type_def_conversion() {
        let transport = create_test_transport();
        let types = transport.node_types().unwrap();

        // Find histogram (has Enum constraint, CPU process, etc.)
        let histogram = types.iter().find(|t| t.type_id == "histogram");
        assert!(histogram.is_some());
        let histogram = histogram.unwrap();
        assert_eq!(histogram.title, "Histogram");
        assert_eq!(histogram.category, "tool");
        assert!(histogram.has_preview);

        // Should have "channel" param with Options constraint
        let channel_param = histogram.params.iter().find(|p| p.name == "channel");
        assert!(channel_param.is_some());
        let channel_param = channel_param.unwrap();
        assert_eq!(channel_param.data_type, "string");
        match &channel_param.constraint {
            Some(ConstraintInfo::Options(opts)) => {
                assert!(!opts.is_empty());
                assert!(opts.iter().any(|(label, _)| label == "rgb"));
            }
            other => panic!("expected Options constraint, got {:?}", other),
        }
    }

    #[test]
    fn test_gpu_only_node_without_gpu_returns_error() {
        let transport = create_test_transport();
        let (tx, rx) = mpsc::channel();

        // solid_color is GPU-only — should fail without GPU context
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

        let result = transport.execute(request, tx);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("requires GPU"),
            "error should mention GPU, got: {}",
            err_msg
        );

        // Should also have sent an Error progress
        let mut got_error = false;
        while let Ok(progress) = rx.try_recv() {
            if let ExecuteProgress::Error { message, .. } = progress {
                assert!(message.contains("requires GPU"));
                got_error = true;
            }
        }
        assert!(got_error);
    }
}
