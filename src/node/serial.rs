use crate::node::registry::NodeRegistry;
use crate::node::types::Value;
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
