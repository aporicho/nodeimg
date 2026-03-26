pub mod local;

pub use crate::internal::backend::BackendClient;

use crate::cache::NodeId;
use nodeimg_types::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::Sender;

// === Transport Trait ===

/// Abstraction over execution backends (local, remote, etc.).
/// Decouples the app from the engine internals.
pub trait ProcessingTransport: Send + Sync + 'static {
    /// Execute a node graph, pushing per-node results via `progress`.
    fn execute(
        &self,
        request: &GraphRequest,
        progress: Sender<ExecuteProgress>,
    ) -> Result<(), String>;

    /// Invalidate cache for a node and its downstream dependents.
    fn invalidate(&self, node_id: NodeId);

    /// Clear all cached results.
    fn invalidate_all(&self);

    /// Check whether two data types can be connected.
    fn is_compatible(&self, from_type: &str, to_type: &str) -> bool;

    /// Health check (always OK for local, ping for remote).
    fn health_check(&self) -> Result<HealthResponse, String>;

    /// List all available node type definitions.
    fn node_types(&self) -> Result<Vec<NodeTypeDef>, String>;

    /// Synchronous local evaluation (skips AI nodes). Results written to internal cache.
    fn evaluate_local_sync(&self, request: &GraphRequest) -> Result<(), String>;

    /// Query cached output for a node (does not trigger execution).
    fn get_cached(&self, node_id: NodeId) -> Option<HashMap<String, Value>>;

    /// Check for pending AI execution in the graph.
    /// Returns (ai_node_id, serialized_subgraph_json) if found.
    fn pending_ai_execution(
        &self,
        request: &GraphRequest,
    ) -> Option<(NodeId, serde_json::Value)>;

    /// Check if adding connections would create a cycle.
    fn would_create_cycle(
        &self,
        target: NodeId,
        connections: &[ConnectionRequest],
    ) -> bool;
}

// === Execution Progress ===

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

// === Request / Response Types ===

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
    /// Convert to the engine-internal Value type.
    pub fn to_value(&self) -> Value {
        match self {
            ParamValue::Float(f) => Value::Float(*f),
            ParamValue::Int(i) => Value::Int(*i),
            ParamValue::String(s) => Value::String(s.clone()),
            ParamValue::Boolean(b) => Value::Boolean(*b),
            ParamValue::Color(c) => Value::Color(*c),
        }
    }

    /// Convert from a Value (only scalar types; Image/GpuImage/Mask return None).
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Float(f) => Some(ParamValue::Float(*f)),
            Value::Int(i) => Some(ParamValue::Int(*i)),
            Value::String(s) => Some(ParamValue::String(s.clone())),
            Value::Boolean(b) => Some(ParamValue::Boolean(*b)),
            Value::Color(c) => Some(ParamValue::Color(*c)),
            _ => None, // Image/GpuImage/Mask are not serializable
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
    fn test_param_value_roundtrip_float() {
        let pv = ParamValue::Float(3.14);
        let v = pv.to_value();
        let pv2 = ParamValue::from_value(&v).unwrap();
        match pv2 {
            ParamValue::Float(f) => assert!((f - 3.14).abs() < 0.001),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn test_param_value_roundtrip_int() {
        let pv = ParamValue::Int(42);
        let v = pv.to_value();
        let pv2 = ParamValue::from_value(&v).unwrap();
        match pv2 {
            ParamValue::Int(i) => assert_eq!(i, 42),
            _ => panic!("expected Int"),
        }
    }

    #[test]
    fn test_param_value_roundtrip_string() {
        let pv = ParamValue::String("hello".into());
        let v = pv.to_value();
        let pv2 = ParamValue::from_value(&v).unwrap();
        match pv2 {
            ParamValue::String(s) => assert_eq!(s, "hello"),
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn test_param_value_roundtrip_boolean() {
        let pv = ParamValue::Boolean(true);
        let v = pv.to_value();
        let pv2 = ParamValue::from_value(&v).unwrap();
        match pv2 {
            ParamValue::Boolean(b) => assert!(b),
            _ => panic!("expected Boolean"),
        }
    }

    #[test]
    fn test_param_value_roundtrip_color() {
        let pv = ParamValue::Color([1.0, 0.5, 0.0, 1.0]);
        let v = pv.to_value();
        let pv2 = ParamValue::from_value(&v).unwrap();
        match pv2 {
            ParamValue::Color(c) => {
                assert_eq!(c[0], 1.0);
                assert_eq!(c[1], 0.5);
                assert_eq!(c[2], 0.0);
                assert_eq!(c[3], 1.0);
            }
            _ => panic!("expected Color"),
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
            nodes: HashMap::from([(
                0,
                NodeRequest {
                    type_id: "invert".into(),
                    params: HashMap::from([("brightness".into(), ParamValue::Float(0.5))]),
                },
            )]),
            connections: vec![ConnectionRequest {
                from_node: 0,
                from_pin: "image".into(),
                to_node: 1,
                to_pin: "image".into(),
            }],
            target_node: 0,
        };
        let json = serde_json::to_string(&req).unwrap();
        let req2: GraphRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req2.target_node, 0);
        assert_eq!(req2.nodes.len(), 1);
        assert_eq!(req2.connections.len(), 1);
        assert_eq!(req2.connections[0].from_pin, "image");
    }

    #[test]
    fn test_connection_request_serialize() {
        let conn = ConnectionRequest {
            from_node: 0,
            from_pin: "output".into(),
            to_node: 1,
            to_pin: "input".into(),
        };
        let json = serde_json::to_string(&conn).unwrap();
        let conn2: ConnectionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(conn2.from_node, 0);
        assert_eq!(conn2.to_node, 1);
        assert_eq!(conn2.from_pin, "output");
        assert_eq!(conn2.to_pin, "input");
    }

    #[test]
    fn test_health_response_serialize() {
        let resp = HealthResponse {
            status: "ok".into(),
            gpu: Some("NVIDIA RTX 4090".into()),
            vram_free_gb: Some(20.5),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let resp2: HealthResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp2.status, "ok");
        assert_eq!(resp2.gpu.as_deref(), Some("NVIDIA RTX 4090"));
        assert!((resp2.vram_free_gb.unwrap() - 20.5).abs() < 0.001);
    }

    #[test]
    fn test_node_type_def_serialize() {
        let def = NodeTypeDef {
            type_id: "blur".into(),
            title: "Gaussian Blur".into(),
            category: "filter".into(),
            inputs: vec![PinDefInfo {
                name: "image".into(),
                data_type: "image".into(),
                required: true,
            }],
            outputs: vec![PinDefInfo {
                name: "image".into(),
                data_type: "image".into(),
                required: false,
            }],
            params: vec![ParamDefInfo {
                name: "radius".into(),
                data_type: "float".into(),
                default: ParamValue::Float(5.0),
                constraint: Some(ConstraintInfo::Range {
                    min: 0.0,
                    max: 100.0,
                }),
                widget_override: None,
            }],
            has_preview: true,
        };
        let json = serde_json::to_string(&def).unwrap();
        let def2: NodeTypeDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def2.type_id, "blur");
        assert_eq!(def2.inputs.len(), 1);
        assert_eq!(def2.params.len(), 1);
    }

    #[test]
    fn test_constraint_info_serialize() {
        let range = ConstraintInfo::Range {
            min: -1.0,
            max: 1.0,
        };
        let json = serde_json::to_string(&range).unwrap();
        let range2: ConstraintInfo = serde_json::from_str(&json).unwrap();
        match range2 {
            ConstraintInfo::Range { min, max } => {
                assert_eq!(min, -1.0);
                assert_eq!(max, 1.0);
            }
            _ => panic!("expected Range"),
        }

        let opts = ConstraintInfo::Options(vec![("A".into(), "a".into())]);
        let json = serde_json::to_string(&opts).unwrap();
        let opts2: ConstraintInfo = serde_json::from_str(&json).unwrap();
        match opts2 {
            ConstraintInfo::Options(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].0, "A");
            }
            _ => panic!("expected Options"),
        }
    }
}
