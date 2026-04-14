use std::collections::HashMap;
use std::sync::Arc;

use engine::capability::{Capability, CapabilityId, SideEffect};
use engine::events::{EngineEvent, ExecutionStatus, NodeExecutionStatus, PollResult};
use engine::execution::{ExecutionOutputs, NodeExecutionRequest};
use engine::executors::{Executor, ExecutorFuture, LocalityProfile};
use engine::facade::{EngineFacade, ExecutionRequest};
use engine::graph::model::subgraph::ExecuteTarget;
use engine::node_manager::{ExecutionPolicy, NodeDef, NodeSourceKind, ParamDef, ParamExpose, PinDef, Purity};
use engine::node_registry::{NodeRegistration, NodeRegistry, NodeSource};
use engine::Engine;
use types::{DataType, Value};

struct EventTestExecutor;

struct SlowEventExecutor;

impl Executor for EventTestExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "test.engine_events",
            Vec::new(),
            vec![DataType::float()],
            vec![SideEffect::None],
        )]
    }

    fn execute<'a>(
        &'a self,
        _cap_id: &'a CapabilityId,
        req: NodeExecutionRequest<'a>,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            req.progress_sink
                .report(engine::execution::ExecutorProgressEvent::Message {
                    text: "working".into(),
                });
            req.progress_sink
                .report(engine::execution::ExecutorProgressEvent::Fraction {
                    current: 1,
                    total: 1,
                });

            Ok(ExecutionOutputs::full(HashMap::from([(
                "value".to_string(),
                Value::Float(1.0),
            )])))
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

impl Executor for SlowEventExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "test.engine_events.slow",
            Vec::new(),
            vec![DataType::float()],
            vec![SideEffect::None],
        )]
    }

    fn execute<'a>(
        &'a self,
        _cap_id: &'a CapabilityId,
        _req: NodeExecutionRequest<'a>,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            std::thread::sleep(std::time::Duration::from_millis(75));
            Ok(ExecutionOutputs::full(HashMap::from([(
                "value".to_string(),
                Value::Float(2.0),
            )])))
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

struct EventTestSource;

impl NodeSource for EventTestSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        vec![NodeRegistration {
            static_def: NodeDef {
                type_id: "engine_event_value".into(),
                version: 1,
                source: NodeSourceKind::Builtin,
                name: "Engine Event Value".into(),
                category: "test".into(),
                requires: vec!["test.engine_events".into()],
                purity: Purity::Pure,
                cooking_sensitivity: vec![],
                realtime_capable: true,
                execution: ExecutionPolicy::default(),
                api: None,
                inputs: vec![],
                outputs: vec![PinDef {
                    name: "value".into(),
                    data_type: DataType::float(),
                    optional: false,
                }],
                params: vec![ParamDef {
                    name: "value".into(),
                    data_type: DataType::float(),
                    constraint: None,
                    default_value: Value::Float(1.0),
                    expose: vec![ParamExpose::Control],
                }],
                execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
            },
            schema_provider: None,
            presentation_provider: None,
            capability_bindings: vec![("test.engine_events".into(), Arc::new(EventTestExecutor))],
        }]
    }
}

fn make_engine() -> Engine {
    let mut registry = NodeRegistry::new();
    registry.register(EventTestSource);
    Engine::from_registry_bundle(registry.build(), None)
}

fn make_slow_engine() -> Engine {
    struct SlowSource;

    impl NodeSource for SlowSource {
        fn collect(&self) -> Vec<NodeRegistration> {
            vec![NodeRegistration {
                static_def: NodeDef {
                    type_id: "engine_event_value_slow".into(),
                    version: 1,
                    source: NodeSourceKind::Builtin,
                    name: "Slow Engine Event Value".into(),
                    category: "test".into(),
                    requires: vec!["test.engine_events.slow".into()],
                    purity: Purity::Pure,
                    cooking_sensitivity: vec![],
                    realtime_capable: true,
                    execution: ExecutionPolicy::default(),
                    api: None,
                    inputs: vec![],
                    outputs: vec![PinDef {
                        name: "value".into(),
                        data_type: DataType::float(),
                        optional: false,
                    }],
                    params: vec![],
                    execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
                },
                schema_provider: None,
                presentation_provider: None,
                capability_bindings: vec![(
                    "test.engine_events.slow".into(),
                    Arc::new(SlowEventExecutor),
                )],
            }]
        }
    }

    let mut registry = NodeRegistry::new();
    registry.register(SlowSource);
    Engine::from_registry_bundle(registry.build(), None)
}

#[tokio::test]
async fn engine_subscription_receives_lifecycle_events() {
    let mut engine = make_engine();
    let node_id = engine.add_node("engine_event_value").unwrap();
    let mut sub = engine.subscribe_engine_events();

    let ticket = engine
        .execute_request(ExecutionRequest {
            target: ExecuteTarget::Node(node_id),
            mode: None,
        })
        .await
        .unwrap();
    engine.await_execution(ticket.execution_id).unwrap();

    let PollResult::Items(items) = sub.poll_pending() else {
        panic!("expected engine events after execution");
    };

    assert!(items.iter().any(|record| matches!(
        &record.payload,
        EngineEvent::ExecutionStarted { execution_id, target, .. }
            if *execution_id == ticket.execution_id && *target == ExecuteTarget::Node(node_id)
    )));
    assert!(items.iter().any(|record| matches!(
        &record.payload,
        EngineEvent::NodeStarted { execution_id, node_id: started }
            if *execution_id == ticket.execution_id && *started == node_id
    )));
    assert!(items.iter().any(|record| matches!(
        &record.payload,
        EngineEvent::NodeProgressMessage { execution_id, node_id: progressed, text }
            if *execution_id == ticket.execution_id && *progressed == node_id && text == "working"
    )));
    assert!(items.iter().any(|record| matches!(
        &record.payload,
        EngineEvent::NodeProgressFraction { execution_id, node_id: progressed, current: 1, total: 1 }
            if *execution_id == ticket.execution_id && *progressed == node_id
    )));
    assert!(items.iter().any(|record| matches!(
        &record.payload,
        EngineEvent::NodeFinished { execution_id, node_id: finished, status: NodeExecutionStatus::Finished }
            if *execution_id == ticket.execution_id && *finished == node_id
    )));
    assert!(items.iter().any(|record| matches!(
        &record.payload,
        EngineEvent::ExecutionFinished { execution_id, .. }
            if *execution_id == ticket.execution_id
    )));
    assert_eq!(engine.query_state().status, ExecutionStatus::Idle);
}

#[tokio::test]
async fn execute_request_returns_before_background_execution_finishes() {
    let mut engine = make_slow_engine();
    let node_id = engine.add_node("engine_event_value_slow").unwrap();

    let ticket = engine
        .execute_request(ExecutionRequest {
            target: ExecuteTarget::Node(node_id),
            mode: None,
        })
        .await
        .unwrap();

    assert_eq!(engine.query_state().status, ExecutionStatus::Running);
    assert!(engine.query_execution_outputs(ticket.execution_id, node_id).is_err());

    engine.await_execution(ticket.execution_id).unwrap();
    assert_eq!(engine.query_state().status, ExecutionStatus::Idle);
    let outputs = engine.query_execution_outputs(ticket.execution_id, node_id).unwrap();
    assert!(matches!(outputs.get("value"), Some(Value::Float(actual)) if (*actual - 2.0).abs() < f32::EPSILON));
}
