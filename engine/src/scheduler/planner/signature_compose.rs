use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::cache::model::ExecSignature;
use crate::graph::model::graph::NodeInstance;
use crate::node_manager::NodeDef;
use types::Value;

pub fn compose_exec_signature(
    def: &NodeDef,
    node: &NodeInstance,
    inputs: &HashMap<String, Value>,
) -> ExecSignature {
    let param_names: std::collections::HashSet<&str> =
        def.params.iter().map(|p| p.name.as_str()).collect();

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

    ExecSignature::new(1, node_version, params_hash, upstream_hash)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_manager::{ExecutorType, ParamDef, ParamExpose, PinDef};
    use std::collections::HashMap;
    use types::{DataType, NodeId};

    fn make_def() -> NodeDef {
        NodeDef {
            type_id: "test.node".into(),
            version: 1,
            source: crate::node_manager::NodeSourceKind::Builtin,
            name: "test.node".into(),
            category: "test".into(),
            executor_type: ExecutorType::Image,
            requires: vec![],
            purity: crate::node_manager::Purity::Pure,
            cooking_sensitivity: vec![],
            realtime_capable: true,
            execution: crate::node_manager::ExecutionPolicy::default(),
            api: None,
            inputs: vec![PinDef {
                name: "in".into(),
                data_type: DataType::float(),
                optional: false,
            }],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataType::float(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "amount".into(),
                data_type: DataType::float(),
                constraint: None,
                default_value: Value::Float(1.0),
                expose: vec![ParamExpose::Control],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        }
    }

    fn make_node() -> NodeInstance {
        NodeInstance {
            id: NodeId(1),
            type_id: "test.node".into(),
            params: HashMap::new(),
        }
    }

    #[test]
    fn same_inputs_produce_same_signature() {
        let def = make_def();
        let node = make_node();
        let inputs = HashMap::from([
            (String::from("amount"), Value::Float(1.0)),
            (String::from("in"), Value::Float(2.0)),
        ]);

        let a = compose_exec_signature(&def, &node, &inputs);
        let b = compose_exec_signature(&def, &node, &inputs);

        assert_eq!(a, b);
    }

    #[test]
    fn param_change_affects_params_hash() {
        let def = make_def();
        let node = make_node();
        let a = compose_exec_signature(
            &def,
            &node,
            &HashMap::from([
                (String::from("amount"), Value::Float(1.0)),
                (String::from("in"), Value::Float(2.0)),
            ]),
        );
        let b = compose_exec_signature(
            &def,
            &node,
            &HashMap::from([
                (String::from("amount"), Value::Float(3.0)),
                (String::from("in"), Value::Float(2.0)),
            ]),
        );

        assert_ne!(a.params_hash, b.params_hash);
        assert_eq!(a.upstream_hash, b.upstream_hash);
    }
}
