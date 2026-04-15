use std::collections::{hash_map::DefaultHasher, BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use crate::cache::model::ExecSignature;
use types::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NodeArtifactIdentity {
    pub exec_signature: ExecSignature,
    pub param_signature: String,
    pub input_signature: String,
    pub params_snapshot: BTreeMap<String, String>,
}

pub(crate) fn build_artifact_identity(
    type_id: &str,
    exec_signature: ExecSignature,
    effective_params: &HashMap<String, Value>,
) -> NodeArtifactIdentity {
    let normalized_params = normalize_params(type_id, effective_params);
    NodeArtifactIdentity {
        exec_signature,
        param_signature: format!("params:{}", hash_params(&normalized_params)),
        input_signature: format!(
            "schema:{}:upstream:{}",
            exec_signature.sig_schema_version, exec_signature.upstream_hash,
        ),
        params_snapshot: normalized_params
            .iter()
            .map(|(key, value)| (key.clone(), format_param_snapshot(value)))
            .collect(),
    }
}

fn normalize_params(
    type_id: &str,
    effective_params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut normalized = effective_params.clone();
    if type_id == "ai_video_generate_api" {
        let inferred_fps = match effective_params.get("fps") {
            Some(Value::Int(value)) if *value > 0 => Some(*value),
            Some(Value::Float(value)) if *value > 0.0 => Some(*value as i64),
            _ => match effective_params.get("provider") {
                Some(Value::String(provider)) if provider == "libtv" || provider == "mock" => {
                    Some(8)
                }
                _ => None,
            },
        };
        if let Some(fps) = inferred_fps {
            normalized.insert(String::from("fps"), Value::Int(fps));
        }
    }
    normalized
}

fn hash_params(params: &HashMap<String, Value>) -> u64 {
    let mut entries = params.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut hasher = DefaultHasher::new();
    for (key, value) in entries {
        key.hash(&mut hasher);
        format!("{value:?}").hash(&mut hasher);
    }
    hasher.finish()
}

fn format_param_snapshot(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Int(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Color(value) => format!(
            "[{:.3}, {:.3}, {:.3}, {:.3}]",
            value[0], value[1], value[2], value[3]
        ),
        Value::Handle(handle) => format!("handle:{}@{}", handle.handle_id, handle.backend),
        Value::Image(image) => {
            if let Some(cpu) = image.cpu_data() {
                format!("image:{}x{}", cpu.width(), cpu.height())
            } else {
                String::from("image:<gpu>")
            }
        }
    }
}
