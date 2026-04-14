use crate::capability::{Capability, CapabilityId, SideEffect};
use crate::execution::{ExecutionOutputs, ExecutorError, NodeExecutionRequest};
use crate::executors::{Executor, ExecutorFuture, LocalityProfile};
use crate::node_manager::NodeDef;

pub struct InventoryBuiltinExecutor {
    capabilities: Vec<Capability>,
}

impl InventoryBuiltinExecutor {
    pub fn new(capabilities: Vec<Capability>) -> Self {
        Self { capabilities }
    }
}

impl Executor for InventoryBuiltinExecutor {
    fn provides(&self) -> Vec<Capability> {
        self.capabilities.clone()
    }

    fn execute<'a>(
        &'a self,
        cap_id: &'a CapabilityId,
        req: NodeExecutionRequest<'a>,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            if !req.node_def.requires.iter().any(|required| required == cap_id) {
                return Err(ExecutorError::Unavailable {
                    message: format!(
                        "node '{}' does not require capability '{cap_id}'",
                        req.node_def.type_id
                    ),
                });
            }

            let outputs = (req.node_def.execute)(req.exec_context, req.inputs)
                .await
                .map_err(|error| ExecutorError::RuntimeFailed {
                    message: error.to_string(),
                })?;
            Ok(ExecutionOutputs::full(outputs))
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Any
    }
}

pub fn capability_from_node_def(node_def: &NodeDef) -> Option<Capability> {
    let capability_id = node_def.requires.first()?.clone();
    let side_effects = match node_def.type_id.as_str() {
        "load_image" => vec![SideEffect::FileRead("*".into())],
        "save_image" => vec![SideEffect::FileWrite("*".into())],
        _ => vec![SideEffect::None],
    };

    Some(Capability::new(
        capability_id,
        node_def
            .inputs
            .iter()
            .map(|pin| pin.data_type.clone())
            .collect(),
        node_def
            .outputs
            .iter()
            .map(|pin| pin.data_type.clone())
            .collect(),
        side_effects,
    ))
}
