use std::collections::{BTreeMap, HashMap};

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
    exec_signature: ExecSignature,
    effective_params: &HashMap<String, Value>,
) -> NodeArtifactIdentity {
    NodeArtifactIdentity {
        exec_signature,
        param_signature: format!(
            "schema:{}:node:{}:params:{}",
            exec_signature.sig_schema_version,
            exec_signature.node_version,
            exec_signature.params_hash,
        ),
        input_signature: format!(
            "schema:{}:upstream:{}",
            exec_signature.sig_schema_version, exec_signature.upstream_hash,
        ),
        params_snapshot: effective_params
            .iter()
            .map(|(key, value)| (key.clone(), format_param_snapshot(value)))
            .collect(),
    }
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
