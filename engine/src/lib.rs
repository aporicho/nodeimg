pub mod graph;
pub mod graph_controller;
pub mod node_manager;
pub mod artifact;
pub mod cache;
pub mod executors;
pub mod builtins;

use cache::manager::CacheManager;
use graph_controller::GraphController;
use graph::topology::topo_sort;
use node_manager::NodeManager;
use executors::image::{ImageExecutor, GpuExecutor};
use types::{NodeId, Value};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Engine：顶层协调者。
///
/// 当前持有：
/// - graph_controller：节点图实例与编辑
/// - node_manager：节点定义收集、注册、索引与查询
/// - executor：图像执行入口
pub struct Engine {
    pub graph: GraphController,
    pub node_manager: Arc<NodeManager>,
    pub executor: ImageExecutor,
    pub cache: CacheManager,
}

impl Engine {
    pub fn new(gpu: Option<GpuExecutor>) -> Self {
        let node_manager = Arc::new(NodeManager::from_inventory());
        Self {
            graph: GraphController::new(Arc::clone(&node_manager), 50),
            node_manager,
            executor: ImageExecutor::new(gpu),
            cache: CacheManager::new(),
        }
    }

    pub async fn evaluate(
        &self,
        target: NodeId,
    ) -> Result<HashMap<NodeId, HashMap<String, Value>>, Box<dyn std::error::Error + Send + Sync>> {
        let graph = self.graph.snapshot();
        let order = topo_sort(&graph, target)?;
        let ctx = self.executor.context();
        let generation = self.cache.current_generation();
        let mut results: HashMap<NodeId, HashMap<String, Value>> = HashMap::new();

        for node_id in order {
            let node = graph.nodes.get(&node_id)
                .ok_or_else(|| format!("Node {:?} not found in graph", node_id))?;
            let def = self.node_manager.get_node_def(&node.type_id)
                .ok_or_else(|| format!("Node type '{}' not registered", node.type_id))?;

            // Collect upstream inputs from results
            let mut inputs: HashMap<String, Value> = HashMap::new();
            for conn in &graph.connections {
                if conn.to_node == node_id {
                    if let Some(upstream_outputs) = results.get(&conn.from_node) {
                        if let Some(val) = upstream_outputs.get(&conn.from_pin) {
                            inputs.insert(conn.to_pin.clone(), val.clone());
                        }
                    }
                }
            }

            // Merge param defaults + user values
            for param_def in &def.params {
                if !inputs.contains_key(&param_def.name) {
                    let val = node.params.get(&param_def.name)
                        .cloned()
                        .unwrap_or_else(|| param_def.default_value.clone());
                    inputs.insert(param_def.name.clone(), val);
                }
            }

            let exec_signature = compute_exec_signature(def, node, &inputs);

            if !def.outputs.is_empty() {
                let mut cached_outputs: HashMap<String, Value> = HashMap::new();
                let mut all_cached = true;

                for output in &def.outputs {
                    match self.cache.get_result(node_id, &output.name, exec_signature, generation) {
                        Some(value) => {
                            cached_outputs.insert(output.name.clone(), value.as_ref().clone());
                        }
                        None => {
                            all_cached = false;
                            break;
                        }
                    }
                }

                if all_cached {
                    results.insert(node_id, cached_outputs);
                    continue;
                }
            }

            let outputs = (def.execute)(ctx, inputs).await?;

            for (output_pin, value) in &outputs {
                let _ = self.cache.put_result(
                    node_id,
                    output_pin,
                    exec_signature,
                    generation,
                    value.clone(),
                );
            }

            results.insert(node_id, outputs);
        }

        Ok(results)
    }
}

fn compute_exec_signature(
    def: &node_manager::model::NodeDef,
    node: &graph::Node,
    inputs: &HashMap<String, Value>,
) -> cache::model::ExecSignature {
    let param_names: std::collections::HashSet<&str> = def.params.iter().map(|p| p.name.as_str()).collect();

    let mut params_entries: Vec<(&String, &Value)> = inputs
        .iter()
        .filter(|(key, _)| param_names.contains(key.as_str()))
        .collect();
    params_entries.sort_by(|a, b| a.0.cmp(b.0));

    let mut upstream_entries: Vec<(&String, &Value)> = inputs
        .iter()
        .filter(|(key, _)| !param_names.contains(key.as_str()))
        .collect();
    upstream_entries.sort_by(|a, b| a.0.cmp(b.0));

    let params_hash = hash_entries(&params_entries);
    let upstream_hash = hash_entries(&upstream_entries);
    let node_version = hash_str(&node.type_id) as u16;

    cache::model::ExecSignature::new(1, node_version, params_hash, upstream_hash)
}

fn hash_entries(entries: &[(&String, &Value)]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for (key, value) in entries {
        key.hash(&mut hasher);
        format!("{value:?}").hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_str(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
