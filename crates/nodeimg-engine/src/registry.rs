use nodeimg_gpu::GpuContext;
use nodeimg_types::category::CategoryId;
use nodeimg_types::value::Value;
use std::collections::HashMap;

pub use nodeimg_types::node_def::{ParamDef, PinDef};

/// Processing function signature: inputs + params -> outputs.
pub type ProcessFn = Box<
    dyn Fn(&HashMap<String, Value>, &HashMap<String, Value>) -> HashMap<String, Value>
        + Send
        + Sync,
>;

/// GPU processing function: gpu_ctx + inputs + params -> outputs.
pub type GpuProcessFn = Box<
    dyn Fn(&GpuContext, &HashMap<String, Value>, &HashMap<String, Value>) -> HashMap<String, Value>
        + Send
        + Sync,
>;

/// Complete definition of a node type.
pub struct NodeDef {
    pub type_id: String,
    pub title: String,
    pub category: CategoryId,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
    pub params: Vec<ParamDef>,
    pub has_preview: bool,
    /// Processing function. None for remote/backend-executed nodes.
    pub process: Option<ProcessFn>,
    /// GPU processing function. None when no GPU implementation exists.
    pub gpu_process: Option<GpuProcessFn>,
}

/// Runtime node instance stored in the graph.
#[derive(Clone, Debug)]
pub struct NodeInstance {
    pub type_id: String,
    pub params: HashMap<String, Value>,
}

pub struct NodeRegistry {
    nodes: HashMap<String, NodeDef>,
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, def: NodeDef) {
        self.nodes.insert(def.type_id.clone(), def);
    }

    pub fn get(&self, type_id: &str) -> Option<&NodeDef> {
        self.nodes.get(type_id)
    }

    /// Returns all registered nodes, optionally filtered by category.
    pub fn list(&self, category: Option<&CategoryId>) -> Vec<&NodeDef> {
        self.nodes
            .values()
            .filter(|n| category.is_none_or(|c| n.category == *c))
            .collect()
    }

    /// Creates a new NodeInstance with default parameter values.
    pub fn instantiate(&self, type_id: &str) -> Option<NodeInstance> {
        let def = self.nodes.get(type_id)?;
        let params = def
            .params
            .iter()
            .map(|p| (p.name.clone(), p.default.clone()))
            .collect();
        Some(NodeInstance {
            type_id: type_id.to_string(),
            params,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodeimg_types::constraint::Constraint;
    use nodeimg_types::data_type::DataTypeId;

    #[test]
    fn test_node_instance_creation() {
        let inst = NodeInstance {
            type_id: "color_adjust".into(),
            params: HashMap::from([
                ("brightness".into(), Value::Float(0.0)),
                ("contrast".into(), Value::Float(0.0)),
            ]),
        };
        assert_eq!(inst.type_id, "color_adjust");
        assert_eq!(inst.params.len(), 2);
    }

    #[test]
    fn test_register_and_query_node() {
        let mut reg = NodeRegistry::new();
        reg.register(NodeDef {
            type_id: "invert".into(),
            title: "Invert".into(),
            category: CategoryId::new("color"),
            inputs: vec![PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: true,
            }],
            outputs: vec![PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: Some(Box::new(|_inputs, _params| HashMap::new())),
            gpu_process: None,
        });
        assert!(reg.get("invert").is_some());
        assert_eq!(reg.get("invert").unwrap().title, "Invert");
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_instantiate_node() {
        let mut reg = NodeRegistry::new();
        reg.register(NodeDef {
            type_id: "color_adjust".into(),
            title: "Color Adjustment".into(),
            category: CategoryId::new("color"),
            inputs: vec![PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: true,
            }],
            outputs: vec![PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: false,
            }],
            params: vec![ParamDef {
                name: "brightness".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: -1.0,
                    max: 1.0,
                },
                default: Value::Float(0.0),
                widget_override: None,
            }],
            has_preview: false,
            process: Some(Box::new(|_inputs, _params| HashMap::new())),
            gpu_process: None,
        });
        let inst = reg.instantiate("color_adjust");
        assert!(inst.is_some());
        let inst = inst.unwrap();
        assert_eq!(inst.type_id, "color_adjust");
        match inst.params.get("brightness") {
            Some(Value::Float(v)) => assert_eq!(*v, 0.0),
            _ => panic!("expected float param"),
        }
    }
}
