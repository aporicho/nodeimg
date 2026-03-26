//! HTTP client for communicating with the Python backend.

use crate::internal::cache::NodeId;
use crate::internal::eval::Connection;
use crate::internal::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use nodeimg_types::node_instance::NodeInstance;
use nodeimg_types::category::CategoryId;
use nodeimg_types::constraint::Constraint;
use nodeimg_types::data_type::{DataTypeId, DataTypeInfo, DataTypeRegistry};
use nodeimg_types::value::Value;
use nodeimg_types::widget_id::WidgetId;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

/// Client for the Python FastAPI backend that serves AI node types
/// and executes AI graph operations.
#[derive(Clone)]
pub struct BackendClient {
    client: reqwest::blocking::Client,
    base_url: String,
}

/// AI-specific data types that get registered when connecting to the backend.
const AI_DATA_TYPES: &[(&str, &str)] = &[
    ("model", "Model"),
    ("clip", "CLIP"),
    ("vae", "VAE"),
    ("conditioning", "Conditioning"),
    ("latent", "Latent"),
];

impl BackendClient {
    /// Create a new client targeting the given base URL (e.g. "http://127.0.0.1:8000").
    /// Uses a 5-minute timeout for long-running AI operations.
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Check backend health. Returns the JSON response or an error message.
    pub fn health_check(&self) -> Result<serde_json::Value, String> {
        self.get("/health")
    }

    /// Fetch all registered node type definitions from the backend.
    pub fn fetch_node_types(&self) -> Result<serde_json::Value, String> {
        self.get("/node_types")
    }

    /// Execute a graph on the backend and return the results.
    pub fn execute_graph(&self, graph: &serde_json::Value) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({ "graph": graph });
        self.post("/execute", &body)
    }

    /// Fetch node types from the backend and register them as AI nodes.
    /// Registers AI-specific data types and creates NodeDefs for each remote node.
    /// Returns the number of nodes registered, or an error message.
    pub fn register_remote_nodes(
        &self,
        node_registry: &mut NodeRegistry,
        type_registry: &mut DataTypeRegistry,
    ) -> Result<usize, String> {
        let node_types = self.fetch_node_types()?;

        // Register AI data types (idempotent — skip if already registered)
        for &(id, name) in AI_DATA_TYPES {
            if type_registry.get(&DataTypeId::new(id)).is_none() {
                type_registry.register(DataTypeInfo {
                    id: DataTypeId::new(id),
                    name: name.to_string(),
                });
            }
        }

        let map = node_types
            .as_object()
            .ok_or_else(|| "Expected JSON object from /node_types".to_string())?;

        let mut count = 0;
        for (type_id, def_json) in map {
            let inputs = Self::parse_pins(def_json.get("inputs"));
            let outputs = Self::parse_pins(def_json.get("outputs"));
            let params = Self::parse_params(def_json.get("params"));

            // Build a human-readable title from the type_id (e.g. "LoadCheckpoint" -> "Load Checkpoint")
            let title = Self::type_id_to_title(type_id);

            node_registry.register(NodeDef {
                type_id: type_id.clone(),
                title,
                category: CategoryId::new("ai"),
                inputs,
                outputs,
                params,
                has_preview: false,
                process: None, // Executed on the backend
                gpu_process: None,
            });
            count += 1;
        }

        Ok(count)
    }

    // -- AI graph serialization --

    /// Serialize an AI subgraph rooted at `output_node_id` into the protocol JSON format.
    ///
    /// Walks upstream from the output node, collecting all AI nodes (those with
    /// `process: None` in their NodeDef). Returns the JSON payload expected by
    /// `POST /execute`, or None if the output node is not an AI node.
    pub fn serialize_ai_subgraph(
        output_node_id: NodeId,
        nodes: &HashMap<NodeId, NodeInstance>,
        connections: &[Connection],
        node_registry: &NodeRegistry,
    ) -> Option<serde_json::Value> {
        // Check that the output node is an AI node
        let output_instance = nodes.get(&output_node_id)?;
        let output_def = node_registry.get(&output_instance.type_id)?;
        if !output_def.is_ai_node() {
            return None; // Not an AI node
        }

        // BFS upstream, collecting all connected AI nodes
        let mut ai_nodes: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        queue.push_back(output_node_id);

        while let Some(node_id) = queue.pop_front() {
            if !ai_nodes.insert(node_id) {
                continue;
            }
            // Find all upstream nodes connected to this one
            for conn in connections {
                if conn.to_node == node_id && !ai_nodes.contains(&conn.from_node) {
                    // Only include upstream nodes that are also AI nodes
                    if let Some(inst) = nodes.get(&conn.from_node) {
                        if let Some(def) = node_registry.get(&inst.type_id) {
                            if def.is_ai_node() {
                                queue.push_back(conn.from_node);
                            }
                        }
                    }
                }
            }
        }

        // Build protocol JSON
        let mut json_nodes = serde_json::Map::new();
        for &nid in &ai_nodes {
            if let Some(instance) = nodes.get(&nid) {
                let mut params_json = serde_json::Map::new();
                for (key, value) in &instance.params {
                    params_json.insert(key.clone(), Self::value_to_json(value));
                }
                json_nodes.insert(
                    nid.to_string(),
                    serde_json::json!({
                        "type": instance.type_id,
                        "params": params_json,
                    }),
                );
            }
        }

        // Collect connections between AI nodes
        let json_connections: Vec<serde_json::Value> = connections
            .iter()
            .filter(|c| ai_nodes.contains(&c.from_node) && ai_nodes.contains(&c.to_node))
            .map(|c| {
                serde_json::json!({
                    "from_node": c.from_node.to_string(),
                    "from_output": c.from_pin,
                    "to_node": c.to_node.to_string(),
                    "to_input": c.to_pin,
                })
            })
            .collect();

        Some(serde_json::json!({
            "nodes": json_nodes,
            "connections": json_connections,
            "output_node": output_node_id.to_string(),
        }))
    }

    /// Parse the backend response for a given output node.
    ///
    /// Expected format: `{ "outputs": { "<node_id>": { "image": "<base64 PNG>" } } }`
    /// Returns a map of output pin name -> Value, suitable for caching.
    pub fn parse_backend_response(
        response: &serde_json::Value,
        output_node_id: NodeId,
    ) -> Result<HashMap<String, Value>, String> {
        let outputs = response
            .get("outputs")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "Backend response missing 'outputs' object".to_string())?;

        let node_key = output_node_id.to_string();
        let node_outputs = outputs
            .get(&node_key)
            .and_then(|v| v.as_object())
            .ok_or_else(|| format!("Backend response missing outputs for node '{}'", node_key))?;

        let mut result: HashMap<String, Value> = HashMap::new();

        for (pin_name, pin_value) in node_outputs {
            if let Some(b64_str) = pin_value.as_str() {
                // Try to decode as base64-encoded PNG image
                match Self::decode_base64_image(b64_str) {
                    Ok(img) => {
                        result.insert(pin_name.clone(), Value::Image(std::sync::Arc::new(img)));
                    }
                    Err(e) => {
                        eprintln!(
                            "[backend] Failed to decode base64 image for pin '{}': {}",
                            pin_name, e
                        );
                        // Store as string fallback
                        result.insert(pin_name.clone(), Value::String(b64_str.to_string()));
                    }
                }
            }
        }

        Ok(result)
    }

    /// Decode a base64-encoded PNG string into a DynamicImage.
    fn decode_base64_image(b64: &str) -> Result<image::DynamicImage, String> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| format!("base64 decode error: {}", e))?;
        let img =
            image::load_from_memory(&bytes).map_err(|e| format!("image decode error: {}", e))?;
        Ok(img)
    }

    /// Convert a Value to a serde_json::Value for protocol serialization.
    fn value_to_json(value: &Value) -> serde_json::Value {
        match value {
            Value::Float(f) => serde_json::json!(*f),
            Value::Int(i) => serde_json::json!(*i),
            Value::String(s) => serde_json::json!(s),
            Value::Boolean(b) => serde_json::json!(*b),
            Value::Color(c) => serde_json::json!(c),
            // Image/Mask/GpuImage cannot be serialized to params
            Value::Image(_) | Value::Mask(_) | Value::GpuImage(_) => serde_json::Value::Null,
        }
    }

    /// Collect all AI node IDs from a subgraph rooted at the given output node.
    /// Used by EvalEngine to know which nodes were sent to the backend.
    pub fn collect_ai_node_ids(
        output_node_id: NodeId,
        nodes: &HashMap<NodeId, NodeInstance>,
        connections: &[Connection],
        node_registry: &NodeRegistry,
    ) -> HashSet<NodeId> {
        let mut ai_nodes: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();

        // Check that the output node itself is AI
        if let Some(inst) = nodes.get(&output_node_id) {
            if let Some(def) = node_registry.get(&inst.type_id) {
                if def.is_ai_node() {
                    queue.push_back(output_node_id);
                }
            }
        }

        while let Some(node_id) = queue.pop_front() {
            if !ai_nodes.insert(node_id) {
                continue;
            }
            for conn in connections {
                if conn.to_node == node_id && !ai_nodes.contains(&conn.from_node) {
                    if let Some(inst) = nodes.get(&conn.from_node) {
                        if let Some(def) = node_registry.get(&inst.type_id) {
                            if def.is_ai_node() {
                                queue.push_back(conn.from_node);
                            }
                        }
                    }
                }
            }
        }

        ai_nodes
    }

    // -- JSON parsing helpers --

    /// Parse pin definitions from the JSON array.
    fn parse_pins(json: Option<&serde_json::Value>) -> Vec<PinDef> {
        let arr = match json.and_then(|v| v.as_array()) {
            Some(a) => a,
            None => return Vec::new(),
        };
        arr.iter()
            .filter_map(|pin| {
                let name = pin.get("name")?.as_str()?.to_string();
                let type_str = pin.get("type")?.as_str()?;
                let data_type = Self::map_backend_type(type_str);
                Some(PinDef {
                    name,
                    data_type,
                    required: true,
                })
            })
            .collect()
    }

    /// Parse parameter definitions from the JSON array.
    fn parse_params(json: Option<&serde_json::Value>) -> Vec<ParamDef> {
        let arr = match json.and_then(|v| v.as_array()) {
            Some(a) => a,
            None => return Vec::new(),
        };
        arr.iter()
            .filter_map(|param| {
                let name = param.get("name")?.as_str()?.to_string();
                let type_str = param.get("type")?.as_str()?;

                let (data_type, constraint, default) = Self::parse_param_type(type_str, param);

                Some(ParamDef {
                    name,
                    data_type,
                    constraint,
                    default,
                    widget_override: Self::parse_widget_override(param),
                })
            })
            .collect()
    }

    /// Map a backend type string (e.g. "MODEL", "FLOAT") to a Rust DataTypeId.
    fn map_backend_type(backend_type: &str) -> DataTypeId {
        match backend_type {
            "MODEL" => DataTypeId::new("model"),
            "CLIP" => DataTypeId::new("clip"),
            "VAE" => DataTypeId::new("vae"),
            "CONDITIONING" => DataTypeId::new("conditioning"),
            "LATENT" => DataTypeId::new("latent"),
            "IMAGE" => DataTypeId::new("image"),
            "INT" => DataTypeId::new("int"),
            "FLOAT" => DataTypeId::new("float"),
            "STRING" => DataTypeId::new("string"),
            "BOOL" => DataTypeId::new("boolean"),
            other => DataTypeId::new(&other.to_lowercase()),
        }
    }

    /// Parse parameter type, constraint, and default value from JSON.
    fn parse_param_type(
        type_str: &str,
        param: &serde_json::Value,
    ) -> (DataTypeId, Constraint, Value) {
        match type_str {
            "INT" => {
                let min = param
                    .get("min")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(i32::MIN as f64);
                let max = param
                    .get("max")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(i32::MAX as f64);
                let default = param.get("default").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

                let has_range = param.get("min").is_some() || param.get("max").is_some();
                let constraint = if has_range {
                    Constraint::Range { min, max }
                } else {
                    Constraint::None
                };

                (DataTypeId::new("int"), constraint, Value::Int(default))
            }
            "FLOAT" => {
                let min = param
                    .get("min")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f32::MIN as f64);
                let max = param
                    .get("max")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f32::MAX as f64);
                let default = param.get("default").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                let has_range = param.get("min").is_some() || param.get("max").is_some();
                let constraint = if has_range {
                    Constraint::Range { min, max }
                } else {
                    Constraint::None
                };

                (DataTypeId::new("float"), constraint, Value::Float(default))
            }
            "STRING" => {
                let default = param
                    .get("default")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Check if there's a widget hint for file_picker
                let widget = param.get("widget").and_then(|v| v.as_str());
                let constraint = if widget == Some("file_picker") {
                    Constraint::FilePath { filters: vec![] }
                } else {
                    Constraint::None
                };

                (
                    DataTypeId::new("string"),
                    constraint,
                    Value::String(default),
                )
            }
            "BOOL" => {
                let default = param
                    .get("default")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                (
                    DataTypeId::new("boolean"),
                    Constraint::None,
                    Value::Boolean(default),
                )
            }
            "ENUM" => {
                let options: Vec<(String, String)> = param
                    .get("options")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| (s.to_string(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default();
                let default = param
                    .get("default")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                (
                    DataTypeId::new("string"),
                    Constraint::Enum { options },
                    Value::String(default),
                )
            }
            _ => (
                DataTypeId::new(&type_str.to_lowercase()),
                Constraint::None,
                Value::String(String::new()),
            ),
        }
    }

    /// Parse optional widget override from JSON param.
    fn parse_widget_override(param: &serde_json::Value) -> Option<WidgetId> {
        let widget = param.get("widget")?.as_str()?;
        Some(WidgetId::new(widget))
    }

    /// Convert a PascalCase type_id to a human-readable title.
    /// e.g. "LoadCheckpoint" -> "Load Checkpoint", "CLIPTextEncode" -> "CLIP Text Encode"
    fn type_id_to_title(type_id: &str) -> String {
        let mut result = String::with_capacity(type_id.len() + 4);
        let chars: Vec<char> = type_id.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            if i > 0 && c.is_uppercase() {
                // Don't insert space between consecutive uppercase letters
                // (e.g. "CLIP" stays together), unless the next char is lowercase
                let prev_upper = chars[i - 1].is_uppercase();
                let next_lower = chars.get(i + 1).is_some_and(|nc| nc.is_lowercase());
                if !prev_upper || next_lower {
                    result.push(' ');
                }
            }
            result.push(c);
        }
        result
    }

    // -- private HTTP helpers --

    fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("GET {} failed: {}", url, e))?;
        if !resp.status().is_success() {
            return Err(format!("GET {} returned status {}", url, resp.status()));
        }
        resp.json::<serde_json::Value>()
            .map_err(|e| format!("GET {} JSON parse error: {}", url, e))
    }

    fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .map_err(|e| format!("POST {} failed: {}", url, e))?;
        if !resp.status().is_success() {
            return Err(format!("POST {} returned status {}", url, resp.status()));
        }
        resp.json::<serde_json::Value>()
            .map_err(|e| format!("POST {} JSON parse error: {}", url, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_id_to_title() {
        assert_eq!(
            BackendClient::type_id_to_title("LoadCheckpoint"),
            "Load Checkpoint"
        );
        assert_eq!(
            BackendClient::type_id_to_title("CLIPTextEncode"),
            "CLIP Text Encode"
        );
        assert_eq!(BackendClient::type_id_to_title("KSampler"), "K Sampler");
        assert_eq!(BackendClient::type_id_to_title("VAEDecode"), "VAE Decode");
        assert_eq!(
            BackendClient::type_id_to_title("EmptyLatentImage"),
            "Empty Latent Image"
        );
    }

    #[test]
    fn test_map_backend_type() {
        assert_eq!(
            BackendClient::map_backend_type("MODEL"),
            DataTypeId::new("model")
        );
        assert_eq!(
            BackendClient::map_backend_type("CLIP"),
            DataTypeId::new("clip")
        );
        assert_eq!(
            BackendClient::map_backend_type("VAE"),
            DataTypeId::new("vae")
        );
        assert_eq!(
            BackendClient::map_backend_type("CONDITIONING"),
            DataTypeId::new("conditioning")
        );
        assert_eq!(
            BackendClient::map_backend_type("LATENT"),
            DataTypeId::new("latent")
        );
        assert_eq!(
            BackendClient::map_backend_type("IMAGE"),
            DataTypeId::new("image")
        );
        assert_eq!(
            BackendClient::map_backend_type("INT"),
            DataTypeId::new("int")
        );
        assert_eq!(
            BackendClient::map_backend_type("FLOAT"),
            DataTypeId::new("float")
        );
        assert_eq!(
            BackendClient::map_backend_type("STRING"),
            DataTypeId::new("string")
        );
        assert_eq!(
            BackendClient::map_backend_type("BOOL"),
            DataTypeId::new("boolean")
        );
    }

    #[test]
    fn test_parse_pins() {
        let json: serde_json::Value = serde_json::json!([
            {"name": "model", "type": "MODEL"},
            {"name": "clip", "type": "CLIP"},
        ]);
        let pins = BackendClient::parse_pins(Some(&json));
        assert_eq!(pins.len(), 2);
        assert_eq!(pins[0].name, "model");
        assert_eq!(pins[0].data_type, DataTypeId::new("model"));
        assert_eq!(pins[1].name, "clip");
        assert_eq!(pins[1].data_type, DataTypeId::new("clip"));
    }

    #[test]
    fn test_parse_params_int_with_range() {
        let json: serde_json::Value = serde_json::json!([
            {"name": "steps", "type": "INT", "default": 20, "min": 1, "max": 150, "options": null, "widget": null}
        ]);
        let params = BackendClient::parse_params(Some(&json));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "steps");
        assert_eq!(params[0].data_type, DataTypeId::new("int"));
        match &params[0].constraint {
            Constraint::Range { min, max } => {
                assert_eq!(*min, 1.0);
                assert_eq!(*max, 150.0);
            }
            _ => panic!("expected Range constraint"),
        }
        match &params[0].default {
            Value::Int(v) => assert_eq!(*v, 20),
            _ => panic!("expected Int default"),
        }
    }

    #[test]
    fn test_parse_params_enum() {
        let json: serde_json::Value = serde_json::json!([
            {"name": "sampler_name", "type": "ENUM", "default": "euler", "min": null, "max": null, "options": ["euler", "ddim"], "widget": null}
        ]);
        let params = BackendClient::parse_params(Some(&json));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].data_type, DataTypeId::new("string"));
        match &params[0].constraint {
            Constraint::Enum { options } => {
                assert_eq!(options.len(), 2);
                assert_eq!(options[0].0, "euler");
                assert_eq!(options[1].0, "ddim");
            }
            _ => panic!("expected Enum constraint"),
        }
        match &params[0].default {
            Value::String(s) => assert_eq!(s, "euler"),
            _ => panic!("expected String default"),
        }
    }

    #[test]
    fn test_parse_params_string_with_file_picker() {
        let json: serde_json::Value = serde_json::json!([
            {"name": "checkpoint_path", "type": "STRING", "default": "", "min": null, "max": null, "options": null, "widget": "file_picker"}
        ]);
        let params = BackendClient::parse_params(Some(&json));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].data_type, DataTypeId::new("string"));
        match &params[0].constraint {
            Constraint::FilePath { .. } => {}
            _ => panic!("expected FilePath constraint"),
        }
    }

    #[test]
    fn test_serialize_ai_subgraph() {
        use crate::internal::registry::{NodeDef, PinDef};
        use nodeimg_types::node_instance::NodeInstance;

        let mut node_reg = NodeRegistry::new();

        // Register two AI nodes (process: None)
        node_reg.register(NodeDef {
            type_id: "LoadCheckpoint".into(),
            title: "Load Checkpoint".into(),
            category: CategoryId::new("ai"),
            inputs: vec![],
            outputs: vec![
                PinDef {
                    name: "model".into(),
                    data_type: DataTypeId::new("model"),
                    required: false,
                },
                PinDef {
                    name: "clip".into(),
                    data_type: DataTypeId::new("clip"),
                    required: false,
                },
            ],
            params: vec![],
            has_preview: false,
            process: None,
            gpu_process: None,
        });

        node_reg.register(NodeDef {
            type_id: "KSampler".into(),
            title: "K Sampler".into(),
            category: CategoryId::new("ai"),
            inputs: vec![PinDef {
                name: "model".into(),
                data_type: DataTypeId::new("model"),
                required: true,
            }],
            outputs: vec![PinDef {
                name: "latent".into(),
                data_type: DataTypeId::new("latent"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: None,
            gpu_process: None,
        });

        let mut nodes = HashMap::new();
        nodes.insert(
            0,
            NodeInstance {
                type_id: "LoadCheckpoint".into(),
                params: HashMap::from([(
                    "checkpoint_path".into(),
                    Value::String("/models/test.safetensors".into()),
                )]),
            },
        );
        nodes.insert(
            1,
            NodeInstance {
                type_id: "KSampler".into(),
                params: HashMap::from([("steps".into(), Value::Int(20))]),
            },
        );

        let connections = vec![Connection {
            from_node: 0,
            from_pin: "model".into(),
            to_node: 1,
            to_pin: "model".into(),
        }];

        let result = BackendClient::serialize_ai_subgraph(1, &nodes, &connections, &node_reg);
        assert!(result.is_some());

        let graph = result.unwrap();
        let graph_nodes = graph.get("nodes").unwrap().as_object().unwrap();
        assert_eq!(graph_nodes.len(), 2);
        assert!(graph_nodes.contains_key("0"));
        assert!(graph_nodes.contains_key("1"));

        let conns = graph.get("connections").unwrap().as_array().unwrap();
        assert_eq!(conns.len(), 1);

        let output_node = graph.get("output_node").unwrap().as_str().unwrap();
        assert_eq!(output_node, "1");
    }

    #[test]
    fn test_serialize_ai_subgraph_skips_local_nodes() {
        use crate::internal::registry::{NodeDef, PinDef};
        use nodeimg_types::node_instance::NodeInstance;

        let mut node_reg = NodeRegistry::new();

        // AI node
        node_reg.register(NodeDef {
            type_id: "VAEDecode".into(),
            title: "VAE Decode".into(),
            category: CategoryId::new("ai"),
            inputs: vec![],
            outputs: vec![PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: None,
            gpu_process: None,
        });

        // Local node (has process)
        node_reg.register(NodeDef {
            type_id: "preview".into(),
            title: "Preview".into(),
            category: CategoryId::new("tool"),
            inputs: vec![PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: true,
            }],
            outputs: vec![],
            params: vec![],
            has_preview: true,
            process: Some(Box::new(|inputs, _params| inputs.clone())),
            gpu_process: None,
        });

        let mut nodes = HashMap::new();
        nodes.insert(
            0,
            NodeInstance {
                type_id: "VAEDecode".into(),
                params: HashMap::new(),
            },
        );
        nodes.insert(
            1,
            NodeInstance {
                type_id: "preview".into(),
                params: HashMap::new(),
            },
        );

        let connections = vec![Connection {
            from_node: 0,
            from_pin: "image".into(),
            to_node: 1,
            to_pin: "image".into(),
        }];

        // Trying to serialize from the preview node (local) should return None
        let result = BackendClient::serialize_ai_subgraph(1, &nodes, &connections, &node_reg);
        assert!(result.is_none());

        // Serializing from the VAEDecode node (AI) should work
        let result = BackendClient::serialize_ai_subgraph(0, &nodes, &connections, &node_reg);
        assert!(result.is_some());
        let graph = result.unwrap();
        let graph_nodes = graph.get("nodes").unwrap().as_object().unwrap();
        assert_eq!(graph_nodes.len(), 1); // Only the AI node, not the local preview
    }

    #[test]
    fn test_parse_backend_response_with_image() {
        // Create a tiny 1x1 PNG as base64
        use base64::Engine;
        let img = image::DynamicImage::new_rgba8(1, 1);
        let mut png_bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        let response = serde_json::json!({
            "outputs": {
                "6": {
                    "image": b64
                }
            }
        });

        let result = BackendClient::parse_backend_response(&response, 6);
        assert!(result.is_ok());
        let outputs = result.unwrap();
        assert!(outputs.contains_key("image"));
        match outputs.get("image") {
            Some(Value::Image(img)) => {
                assert_eq!(img.width(), 1);
                assert_eq!(img.height(), 1);
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_backend_response_error() {
        let response = serde_json::json!({
            "no_outputs": {}
        });
        let result = BackendClient::parse_backend_response(&response, 6);
        assert!(result.is_err());
    }

    #[test]
    fn test_value_to_json() {
        assert_eq!(
            BackendClient::value_to_json(&Value::Float(3.14)),
            serde_json::json!(3.14f32)
        );
        assert_eq!(
            BackendClient::value_to_json(&Value::Int(42)),
            serde_json::json!(42)
        );
        assert_eq!(
            BackendClient::value_to_json(&Value::String("hello".into())),
            serde_json::json!("hello")
        );
        assert_eq!(
            BackendClient::value_to_json(&Value::Boolean(true)),
            serde_json::json!(true)
        );
    }

    #[test]
    fn test_collect_ai_node_ids() {
        use crate::internal::registry::{NodeDef, PinDef};
        use nodeimg_types::node_instance::NodeInstance;

        let mut node_reg = NodeRegistry::new();

        node_reg.register(NodeDef {
            type_id: "ai_a".into(),
            title: "AI A".into(),
            category: CategoryId::new("ai"),
            inputs: vec![],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataTypeId::new("latent"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: None,
            gpu_process: None,
        });

        node_reg.register(NodeDef {
            type_id: "ai_b".into(),
            title: "AI B".into(),
            category: CategoryId::new("ai"),
            inputs: vec![PinDef {
                name: "in".into(),
                data_type: DataTypeId::new("latent"),
                required: true,
            }],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataTypeId::new("image"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: None,
            gpu_process: None,
        });

        node_reg.register(NodeDef {
            type_id: "local".into(),
            title: "Local".into(),
            category: CategoryId::new("tool"),
            inputs: vec![PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: true,
            }],
            outputs: vec![],
            params: vec![],
            has_preview: true,
            process: Some(Box::new(|i, _| i.clone())),
            gpu_process: None,
        });

        let mut nodes = HashMap::new();
        nodes.insert(
            0,
            NodeInstance {
                type_id: "ai_a".into(),
                params: HashMap::new(),
            },
        );
        nodes.insert(
            1,
            NodeInstance {
                type_id: "ai_b".into(),
                params: HashMap::new(),
            },
        );
        nodes.insert(
            2,
            NodeInstance {
                type_id: "local".into(),
                params: HashMap::new(),
            },
        );

        let connections = vec![
            Connection {
                from_node: 0,
                from_pin: "out".into(),
                to_node: 1,
                to_pin: "in".into(),
            },
            Connection {
                from_node: 1,
                from_pin: "out".into(),
                to_node: 2,
                to_pin: "image".into(),
            },
        ];

        // From AI node 1, should collect both AI nodes but not the local node
        let ai_ids = BackendClient::collect_ai_node_ids(1, &nodes, &connections, &node_reg);
        assert_eq!(ai_ids.len(), 2);
        assert!(ai_ids.contains(&0));
        assert!(ai_ids.contains(&1));
        assert!(!ai_ids.contains(&2));

        // From local node 2, should return empty (not an AI node)
        let ai_ids = BackendClient::collect_ai_node_ids(2, &nodes, &connections, &node_reg);
        assert!(ai_ids.is_empty());
    }
}
