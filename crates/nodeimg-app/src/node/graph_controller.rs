use crate::execution::ExecutionManager;
use crate::gpu::GpuContext;
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::{
    ConnectionRequest, ConstraintInfo, GraphRequest, NodeRequest, NodeTypeDef,
    ParamValue, ProcessingTransport,
};
use nodeimg_types::constraint::Constraint;
use nodeimg_types::node_instance::NodeInstance;
use egui_snarl::{InPinId, NodeId, OutPin, InPin, Snarl};
use std::collections::HashMap;
use std::sync::Arc;

pub struct GraphController {
    pub transport: Arc<LocalTransport>,
    pub node_type_defs: HashMap<String, NodeTypeDef>,
    pub execution_manager: ExecutionManager,
    pub gpu_ctx: Option<Arc<GpuContext>>,
    pub backend_status: Option<String>,
    pub ai_errors: HashMap<NodeId, String>,
    pub graph_dirty: bool,
}

impl GraphController {
    pub fn new(gpu_ctx: Option<Arc<GpuContext>>, transport: Arc<LocalTransport>) -> Self {
        let node_type_defs: HashMap<String, NodeTypeDef> = transport
            .node_types()
            .unwrap_or_default()
            .into_iter()
            .map(|d| (d.type_id.clone(), d))
            .collect();
        let execution_manager =
            ExecutionManager::new(Arc::clone(&transport) as Arc<dyn ProcessingTransport>);

        Self {
            transport,
            node_type_defs,
            execution_manager,
            gpu_ctx,
            backend_status: None,
            ai_errors: HashMap::new(),
            graph_dirty: false,
        }
    }

    pub fn find_type_def(&self, type_id: &str) -> Option<&NodeTypeDef> {
        self.node_type_defs.get(type_id)
    }

    pub fn invalidate(&mut self, node_id: NodeId) {
        self.transport.invalidate(node_id.0);
    }

    pub fn invalidate_all(&mut self) {
        self.transport.invalidate_all();
    }

    pub fn build_graph_request(
        &self,
        target: NodeId,
        snarl: &Snarl<NodeInstance>,
    ) -> GraphRequest {
        let mut nodes = HashMap::new();
        for (nid, instance) in snarl.node_ids() {
            let params: HashMap<String, ParamValue> = instance
                .params
                .iter()
                .filter_map(|(k, v)| ParamValue::from_value(v).map(|pv| (k.clone(), pv)))
                .collect();
            nodes.insert(
                nid.0,
                NodeRequest {
                    type_id: instance.type_id.clone(),
                    params,
                },
            );
        }
        let connections = self.build_connection_requests(snarl);
        GraphRequest {
            nodes,
            connections,
            target_node: target.0,
        }
    }

    pub fn build_connection_requests(
        &self,
        snarl: &Snarl<NodeInstance>,
    ) -> Vec<ConnectionRequest> {
        let mut connections = Vec::new();
        for (node_id, instance) in snarl.node_ids() {
            let def = match self.find_type_def(&instance.type_id) {
                Some(d) => d,
                None => continue,
            };
            for (i, input_pin) in def.inputs.iter().enumerate() {
                let in_pin = snarl.in_pin(InPinId {
                    node: node_id,
                    input: i,
                });
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

    pub fn constraint_info_to_engine(info: &Option<ConstraintInfo>) -> Constraint {
        match info {
            None => Constraint::None,
            Some(ConstraintInfo::Range { min, max }) => Constraint::Range {
                min: *min,
                max: *max,
            },
            Some(ConstraintInfo::Options(opts)) => Constraint::Enum {
                options: opts.clone(),
            },
            Some(ConstraintInfo::FilePath { filters }) => Constraint::FilePath {
                filters: filters.clone(),
            },
        }
    }

    /// Validate whether a connection is allowed (type compatibility + cycle detection).
    pub fn validate_connection(
        &self,
        from: &OutPin,
        to: &InPin,
        snarl: &Snarl<NodeInstance>,
    ) -> bool {
        // Type compatibility check
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
            return false;
        }

        // Cycle detection
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

        !self
            .transport
            .would_create_cycle(to.id.node.0, &connections)
    }
}
