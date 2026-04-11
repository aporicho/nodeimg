pub mod graph;
pub mod graph_controller;
pub mod node_manager;
pub mod artifact;
pub mod executors;
pub mod builtins;

use graph_controller::GraphController;
use graph::topology::topo_sort;
use node_manager::NodeManager;
use executors::image::{ImageExecutor, GpuExecutor};
use types::{NodeId, Value};
use std::collections::HashMap;
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
}

impl Engine {
    pub fn new(gpu: Option<GpuExecutor>) -> Self {
        let node_manager = Arc::new(NodeManager::from_inventory());
        Self {
            graph: GraphController::new(Arc::clone(&node_manager), 50),
            node_manager,
            executor: ImageExecutor::new(gpu),
        }
    }

    pub async fn evaluate(
        &self,
        target: NodeId,
    ) -> Result<HashMap<NodeId, HashMap<String, Value>>, Box<dyn std::error::Error + Send + Sync>> {
        let graph = self.graph.snapshot();
        let order = topo_sort(&graph, target)?;
        let ctx = self.executor.context();
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

            let outputs = (def.execute)(ctx, inputs).await?;
            results.insert(node_id, outputs);
        }

        Ok(results)
    }
}
