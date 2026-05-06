use engine::capability::{Capability, CapabilityId, SideEffect};
use engine::execution::{ExecutionOutputs, NodeExecutionRequest};
use engine::executors::{Executor, ExecutorFuture, LocalityProfile};
use engine::facade::EngineFacade;
use engine::node_manager::{
    ArtifactPolicy, ExecutionPolicy, NodeDef, NodeSourceKind, ParamDef, ParamExpose, Purity,
    TriggerPolicy,
};
use engine::node_registry::{NodeRegistration, NodeSource};
use engine::Engine;
use std::collections::HashMap;
use std::sync::Arc;
use types::{Constraint, DataType, Handle, Value};

pub(crate) const UI_CONTROL_TEST_TYPE_ID: &str = "ui_control_test";
const UI_CONTROL_TEST_CAPABILITY: &str = "dev.ui_control_test";

pub(crate) fn clean_room_engine() -> Engine {
    let mut registry = engine::node_registry::NodeRegistry::new();
    registry.register(UiControlTestNodeSource);
    let mut engine = Engine::from_registry_bundle(registry.build(), None);
    engine
        .add_node(UI_CONTROL_TEST_TYPE_ID)
        .expect("dev UI control test node must be registered");
    engine
}

struct UiControlTestNodeSource;

impl NodeSource for UiControlTestNodeSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        let executor = Arc::new(UiControlTestExecutor);
        vec![NodeRegistration {
            static_def: ui_control_test_node_def(),
            schema_provider: None,
            presentation_provider: None,
            capability_bindings: vec![(
                UI_CONTROL_TEST_CAPABILITY.to_string(),
                executor as Arc<dyn Executor>,
            )],
        }]
    }
}

struct UiControlTestExecutor;

impl Executor for UiControlTestExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            UI_CONTROL_TEST_CAPABILITY,
            Vec::new(),
            Vec::new(),
            vec![SideEffect::None],
        )]
    }

    fn execute<'a>(
        &'a self,
        _cap_id: &'a CapabilityId,
        _req: NodeExecutionRequest<'a>,
    ) -> ExecutorFuture<'a> {
        Box::pin(async { Ok(ExecutionOutputs::full(HashMap::new())) })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

fn ui_control_test_node_def() -> NodeDef {
    NodeDef {
        type_id: UI_CONTROL_TEST_TYPE_ID.to_string(),
        version: 1,
        source: NodeSourceKind::Builtin,
        name: "UI Control Test".to_string(),
        category: "dev/ui".to_string(),
        requires: vec![UI_CONTROL_TEST_CAPABILITY.to_string()],
        purity: Purity::Pure,
        cooking_sensitivity: Vec::new(),
        realtime_capable: false,
        execution: ExecutionPolicy {
            artifact_policy: Some(ArtifactPolicy::None),
            trigger_policy: TriggerPolicy::ManualOnly,
            ..ExecutionPolicy::default()
        },
        api: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        params: vec![
            control_param(
                "prompt",
                DataType::string(),
                Value::String("A clean-room prompt".to_string()),
                None,
            ),
            control_param(
                "seed",
                DataType::int(),
                Value::Int(42),
                Some(Constraint::range(0.0, 9999.0)),
            ),
            control_param(
                "strength",
                DataType::float(),
                Value::Float(0.65),
                Some(Constraint::range(0.0, 1.0)),
            ),
            control_param("enabled", DataType::bool(), Value::Bool(true), None),
            control_param(
                "mode",
                DataType::string(),
                Value::String("Euler".to_string()),
                Some(Constraint::enum_options(vec![
                    "Euler".to_string(),
                    "DPM++ 2M".to_string(),
                    "UniPC".to_string(),
                ])),
            ),
            control_param(
                "tint",
                DataType::color(),
                Value::Color([1.0, 0.5, 0.25, 1.0]),
                None,
            ),
            control_param(
                "output_path",
                DataType::string(),
                Value::String("out.png".to_string()),
                Some(Constraint::file_path(vec![
                    "png".to_string(),
                    "jpg".to_string(),
                ])),
            ),
            control_param(
                "asset",
                DataType::handle(),
                Value::Handle(Handle::new("ui-control-test", DataType::image(), "dev", 0)),
                None,
            ),
        ],
        execute: Box::new(|_ctx, _inputs| Box::pin(async { Ok(HashMap::new()) })),
    }
}

fn control_param(
    name: &str,
    data_type: DataType,
    default_value: Value,
    constraint: Option<Constraint>,
) -> ParamDef {
    ParamDef {
        name: name.to_string(),
        data_type,
        constraint,
        default_value,
        expose: vec![ParamExpose::Control],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_room_engine_contains_only_the_ui_control_test_node() {
        let engine = clean_room_engine();
        let graph = engine.query_graph_snapshot();
        let defs = engine.list_node_defs();

        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].type_id, UI_CONTROL_TEST_TYPE_ID);
        assert!(graph
            .nodes
            .values()
            .any(|node| node.type_id == UI_CONTROL_TEST_TYPE_ID));
    }

    #[test]
    fn ui_control_test_node_covers_param_control_shapes() {
        let def = ui_control_test_node_def();
        let param_names = def
            .params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            param_names,
            vec![
                "prompt",
                "seed",
                "strength",
                "enabled",
                "mode",
                "tint",
                "output_path",
                "asset",
            ]
        );
        assert!(def
            .params
            .iter()
            .all(|param| param.expose.contains(&ParamExpose::Control)));
    }
}
