use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use engine::capability::{Capability, CapabilityId, SideEffect};
use engine::events::PollResult;
use engine::execution::{EvaluationFidelity, NodeExecutionRequest};
use engine::executors::{ExecutionOutputs, Executor, ExecutorFuture, LocalityProfile};
use engine::facade::EngineFacade;
use engine::graph::model::subgraph::ExecuteTarget;
use engine::node_manager::{
    ExecutionPolicy, NodeDef, NodeSourceKind, ParamDef, ParamExpose, PinDef, Purity,
};
use engine::node_registry::{NodeRegistration, NodeRegistry, NodeSource};
use engine::session::{InteractionEvent, PreviewRequestStatus, Session, SessionEvent};
use engine::Engine;
use types::{DataType, Value};

struct SessionEventExecutor;

impl Executor for SessionEventExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "test.session_events",
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
            let value = req
                .inputs
                .get("value")
                .cloned()
                .unwrap_or(Value::Float(0.0));
            let outputs = HashMap::from([("value".to_string(), value)]);
            Ok(match &req.fidelity {
                EvaluationFidelity::Preview { .. } => {
                    ExecutionOutputs::preview(outputs, req.fidelity.clone())
                }
                EvaluationFidelity::Full => ExecutionOutputs::full(outputs),
            })
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

struct SessionEventSource;

impl NodeSource for SessionEventSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        vec![NodeRegistration {
            static_def: NodeDef {
                type_id: "session_event_value".into(),
                version: 1,
                source: NodeSourceKind::Builtin,
                name: "Session Event Value".into(),
                category: "test".into(),
                requires: vec!["test.session_events".into()],
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
            capability_bindings: vec![(
                "test.session_events".into(),
                Arc::new(SessionEventExecutor),
            )],
        }]
    }
}

fn make_engine() -> Arc<Mutex<Engine>> {
    let mut registry = NodeRegistry::new();
    registry.register(SessionEventSource);
    Arc::new(Mutex::new(Engine::from_registry_bundle(
        registry.build(),
        None,
    )))
}

#[tokio::test]
async fn session_subscription_receives_interaction_and_preview_events() {
    let engine = make_engine();
    let mut session = Session::new(Arc::clone(&engine));
    let node_id = engine
        .lock()
        .unwrap()
        .add_node("session_event_value")
        .unwrap();
    let mut sub = session.subscribe_session_events();

    session
        .submit_interaction(InteractionEvent::ParamChanged {
            node_id,
            param: "value".into(),
            value: Value::Float(5.0),
            preview: true,
        })
        .unwrap();

    let PollResult::Items(items) = sub.poll_pending() else {
        panic!("expected interaction ack event");
    };
    assert!(items
        .iter()
        .any(|record| matches!(record.payload, SessionEvent::InteractionAck { .. })));

    let status = session
        .request_preview(ExecuteTarget::Node(node_id))
        .await
        .unwrap();
    let ticket = match status {
        PreviewRequestStatus::Executed(ticket) => ticket,
        PreviewRequestStatus::CacheHit => panic!("expected preview execution on cold cache"),
    };
    session.await_execution(ticket.execution_id).unwrap();

    let PollResult::Items(items) = sub.poll_pending() else {
        panic!("expected preview update event");
    };
    assert!(items.iter().any(|record| matches!(
        &record.payload,
        SessionEvent::PreviewUpdated { node_id: updated, output_pin }
            if *updated == node_id && output_pin == "value"
    )));

    session
        .submit_interaction(InteractionEvent::ParamChanged {
            node_id,
            param: "value".into(),
            value: Value::Float(6.0),
            preview: false,
        })
        .unwrap();

    let PollResult::Items(items) = sub.poll_pending() else {
        panic!("expected post-commit session events");
    };
    assert!(items.iter().any(|record| matches!(
        record.payload,
        SessionEvent::UndoStackChanged { can_undo: true, .. }
    )));
}
