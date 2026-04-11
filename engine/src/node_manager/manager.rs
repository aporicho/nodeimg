use crate::registry::NodeDef;
use std::collections::HashMap;
use types::{DataType, Value};

pub struct NodeManager {
    nodes: HashMap<String, NodeDef>,
}

impl NodeManager {
    pub fn from_inventory() -> Self {
        let rust_defs = crate::node_manager::collect::collect_rust_defs::collect_rust_defs();
        let python_defs = crate::node_manager::collect::collect_python_defs::collect_python_defs();
        let defs = crate::node_manager::collect::merge_node_defs::merge_node_defs(rust_defs, python_defs);
        let nodes = crate::node_manager::store::defs_by_type::defs_by_type(defs);
        Self { nodes }
    }

    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, def: NodeDef) {
        self.nodes.insert(def.type_id.clone(), def);
    }

    pub(crate) fn nodes(&self) -> &HashMap<String, NodeDef> {
        &self.nodes
    }

    pub fn default_params(&self, type_id: &str) -> Option<HashMap<String, Value>> {
        self.get(type_id).map(|def| {
            def.params
                .iter()
                .map(|p| (p.name.clone(), p.default_value.clone()))
                .collect()
        })
    }

    pub fn pin_type(&self, type_id: &str, pin_name: &str) -> Option<&DataType> {
        let def = self.get(type_id)?;
        def.inputs
            .iter()
            .chain(def.outputs.iter())
            .find(|p| p.name == pin_name)
            .map(|p| &p.data_type)
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ParamDef, PinDef};

    fn make_test_def(type_id: &str, category: &str) -> NodeDef {
        NodeDef {
            type_id: type_id.into(),
            name: type_id.into(),
            category: category.into(),
            inputs: vec![PinDef {
                name: "in".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "amount".into(),
                data_type: DataType::float(),
                constraint: None,
                default_value: Value::Float(0.0),
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        assert!(nm.get("brightness").is_some());
        assert!(nm.get("nonexistent").is_none());
    }

    #[test]
    fn test_pin_type() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        assert_eq!(nm.pin_type("brightness", "in"), Some(&DataType::image()));
        assert_eq!(nm.pin_type("brightness", "out"), Some(&DataType::image()));
        assert_eq!(nm.pin_type("brightness", "nope"), None);
    }

    #[test]
    fn test_default_params() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        let params = nm.default_params("brightness").unwrap();
        assert!(matches!(params.get("amount"), Some(Value::Float(f)) if *f == 0.0));
    }
}
