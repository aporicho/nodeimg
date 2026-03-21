use crate::node::registry::{NodeInstance, NodeRegistry};
use crate::node::types::Value;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct SerializedGraph {
    pub version: u32,
    pub nodes: Vec<SerializedNode>,
    pub connections: Vec<SerializedConnection>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedNode {
    pub id: usize,
    pub type_id: String,
    pub position: [f32; 2],
    pub params: HashMap<String, SerializedValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "value")]
pub enum SerializedValue {
    Float(f32),
    Int(i32),
    Boolean(bool),
    String(String),
    Color([f32; 4]),
}

#[derive(Serialize, Deserialize)]
pub struct SerializedConnection {
    pub from_node: usize,
    pub from_pin: String,
    pub to_node: usize,
    pub to_pin: String,
}

impl SerializedValue {
    pub fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::Float(f) => Some(Self::Float(*f)),
            Value::Int(i) => Some(Self::Int(*i)),
            Value::Boolean(b) => Some(Self::Boolean(*b)),
            Value::String(s) => Some(Self::String(s.clone())),
            Value::Color(c) => Some(Self::Color(*c)),
            _ => None,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Float(f) => Value::Float(*f),
            Self::Int(i) => Value::Int(*i),
            Self::Boolean(b) => Value::Boolean(*b),
            Self::String(s) => Value::String(s.clone()),
            Self::Color(c) => Value::Color(*c),
        }
    }
}

pub struct Serializer;

impl Serializer {
    pub fn save(graph: &SerializedGraph) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(graph)
    }

    pub fn load(json: &str, registry: &NodeRegistry) -> Result<SerializedGraph, String> {
        let mut graph: SerializedGraph =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))?;

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
            // Unknown node types are intentionally kept in the graph
            // so they survive save/load. EvalEngine skips them.
        }

        Ok(graph)
    }

    /// Take a snapshot of the current Snarl graph into a SerializedGraph.
    pub fn snapshot(
        snarl: &Snarl<NodeInstance>,
        registry: &NodeRegistry,
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
                .and_then(|inst| registry.get(&inst.type_id))
                .and_then(|def| def.outputs.get(out_pin.output))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            let to_name = snarl
                .get_node(in_pin.node)
                .and_then(|inst| registry.get(&inst.type_id))
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

    /// Restore a SerializedGraph into a fresh Snarl, returning it.
    pub fn restore(
        graph: &SerializedGraph,
        registry: &NodeRegistry,
    ) -> Snarl<NodeInstance> {
        let mut snarl = Snarl::new();
        // Map old serialized IDs -> new Snarl NodeIds
        let mut id_map: HashMap<usize, NodeId> = HashMap::new();

        for sn in &graph.nodes {
            let instance = match registry.instantiate(&sn.type_id) {
                Some(mut inst) => {
                    // Apply saved param values
                    for (k, sv) in &sn.params {
                        inst.params.insert(k.clone(), sv.to_value());
                    }
                    inst
                }
                None => {
                    // Unknown node type -- create a placeholder instance so the
                    // graph structure is preserved across save/load even when
                    // the backend is not connected.
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

            // Resolve pin names -> indices
            let out_idx = snarl
                .get_node(from_id)
                .and_then(|inst| registry.get(&inst.type_id))
                .and_then(|def| def.outputs.iter().position(|p| p.name == sc.from_pin));
            let in_idx = snarl
                .get_node(to_id)
                .and_then(|inst| registry.get(&inst.type_id))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_roundtrip() {
        let graph = SerializedGraph {
            version: FORMAT_VERSION,
            nodes: vec![SerializedNode {
                id: 0,
                type_id: "color_adjust".into(),
                position: [100.0, 200.0],
                params: HashMap::from([("brightness".into(), SerializedValue::Float(0.5))]),
            }],
            connections: vec![],
        };
        let json = Serializer::save(&graph).unwrap();
        assert!(json.contains("color_adjust"));
        assert!(json.contains("0.5"));
    }

    #[test]
    fn test_serialized_value_conversion() {
        let v = Value::Float(3.14);
        let sv = SerializedValue::from_value(&v).unwrap();
        let v2 = sv.to_value();
        match v2 {
            Value::Float(f) => assert!((f - 3.14).abs() < 0.001),
            _ => panic!("wrong type"),
        }
    }
}
