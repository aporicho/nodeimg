use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::cache;
use crate::execution::CookingContext;
use crate::node_manager::{NodeDef, NodeManager};
use types::Value;

pub(crate) struct SignatureInput<'a> {
    pub node_manager: &'a NodeManager,
    pub node_def: &'a NodeDef,
    pub effective_params: &'a HashMap<String, Value>,
    pub upstream_signatures: &'a [cache::model::ExecSignature],
    pub cooking_context: &'a CookingContext,
}

pub(crate) fn compute_exec_signature(input: SignatureInput<'_>) -> cache::model::ExecSignature {
    let node_manager = input.node_manager;
    let def = input.node_def;
    let effective_params = input.effective_params;
    let upstream_signatures = input.upstream_signatures;
    let cooking_context = input.cooking_context;

    let mut params_entries: Vec<(&String, &Value)> = effective_params.iter().collect();
    params_entries.sort_by(|a, b| a.0.cmp(b.0));

    let params_hash = hash_entries(&params_entries);
    let upstream_hash = hash_signatures(upstream_signatures);
    let node_version = def.version.max(1) as u16;
    let mut capability_versions = def
        .requires
        .iter()
        .filter_map(|capability| node_manager.capability_version_of(&def.type_id, capability))
        .collect::<Vec<_>>();
    capability_versions.sort();
    let capability_version = capability_versions.into_iter().fold(0_u32, |acc, version| {
        acc.wrapping_mul(31).wrapping_add(version)
    });
    let cooking_context_hash = cooking_context.hash_filtered(&def.cooking_sensitivity);

    cache::model::ExecSignature::with_context(
        2,
        node_version,
        params_hash,
        upstream_hash,
        cooking_context_hash,
        capability_version,
    )
}

fn hash_entries(entries: &[(&String, &Value)]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for (key, value) in entries {
        key.hash(&mut hasher);
        format!("{value:?}").hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::model::ExecSignature;
    use crate::execution::{CookingContext, DimensionValue};
    use crate::node_manager::{
        ExecutionPolicy, NodeSourceKind, ParamDef, ParamExpose, PinDef, Purity,
    };
    use types::DataType;

    fn make_def() -> NodeDef {
        NodeDef {
            type_id: "test.node".into(),
            version: 1,
            source: NodeSourceKind::Builtin,
            name: "Test Node".into(),
            category: "test".into(),
            requires: vec![],
            purity: Purity::Pure,
            cooking_sensitivity: vec![],
            realtime_capable: true,
            execution: ExecutionPolicy::default(),
            api: None,
            inputs: vec![],
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

    fn signature(
        node_manager: &NodeManager,
        node_def: &NodeDef,
        effective_params: &HashMap<String, Value>,
        upstream_signatures: &[ExecSignature],
        cooking_context: &CookingContext,
    ) -> ExecSignature {
        compute_exec_signature(SignatureInput {
            node_manager,
            node_def,
            effective_params,
            upstream_signatures,
            cooking_context,
        })
    }

    #[test]
    fn params_change_signature() {
        let node_manager = NodeManager::new();
        let node_def = make_def();
        let first_params = HashMap::from([("amount".into(), Value::Float(1.0))]);
        let second_params = HashMap::from([("amount".into(), Value::Float(2.0))]);

        let first = signature(
            &node_manager,
            &node_def,
            &first_params,
            &[],
            &CookingContext::default(),
        );
        let second = signature(
            &node_manager,
            &node_def,
            &second_params,
            &[],
            &CookingContext::default(),
        );

        assert_ne!(first, second);
    }

    #[test]
    fn upstream_signature_changes_signature() {
        let node_manager = NodeManager::new();
        let node_def = make_def();
        let params = HashMap::from([("amount".into(), Value::Float(1.0))]);
        let first_upstream = [ExecSignature::with_context(2, 1, 10, 20, 30, 40)];
        let second_upstream = [ExecSignature::with_context(2, 1, 11, 20, 30, 40)];

        let first = signature(
            &node_manager,
            &node_def,
            &params,
            &first_upstream,
            &CookingContext::default(),
        );
        let second = signature(
            &node_manager,
            &node_def,
            &params,
            &second_upstream,
            &CookingContext::default(),
        );

        assert_ne!(first, second);
    }

    #[test]
    fn frame_context_only_affects_frame_sensitive_nodes() {
        let node_manager = NodeManager::new();
        let mut frame_sensitive = make_def();
        frame_sensitive.cooking_sensitivity = vec!["frame".into()];
        let frame_insensitive = make_def();
        let params = HashMap::from([("amount".into(), Value::Float(1.0))]);
        let frame_zero = CookingContext {
            dimensions: HashMap::from([("frame".into(), DimensionValue::Int(0))]),
        };
        let frame_one = CookingContext {
            dimensions: HashMap::from([("frame".into(), DimensionValue::Int(1))]),
        };

        assert_ne!(
            signature(&node_manager, &frame_sensitive, &params, &[], &frame_zero),
            signature(&node_manager, &frame_sensitive, &params, &[], &frame_one)
        );
        assert_eq!(
            signature(&node_manager, &frame_insensitive, &params, &[], &frame_zero),
            signature(&node_manager, &frame_insensitive, &params, &[], &frame_one)
        );
    }
}

fn hash_signatures(signatures: &[cache::model::ExecSignature]) -> u64 {
    let mut ordered = signatures.to_vec();
    ordered.sort_by_key(|signature| {
        (
            signature.sig_schema_version,
            signature.node_version,
            signature.params_hash,
            signature.upstream_hash,
            signature.cooking_context_hash,
            signature.capability_version,
        )
    });

    let mut hasher = DefaultHasher::new();
    for signature in ordered {
        signature.hash(&mut hasher);
    }
    hasher.finish()
}
