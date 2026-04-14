use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use engine::capability::{Capability, CapabilityId, SideEffect};
use engine::execution::{EvaluationFidelity, NodeExecutionRequest};
use engine::executors::{ExecutionOutputs, Executor, ExecutorFuture, LocalityProfile};
use engine::facade::EngineFacade;
use engine::graph::model::subgraph::ExecuteTarget;
use engine::node_manager::{ExecutionPolicy, NodeDef, NodeSourceKind, ParamDef, ParamExpose, PinDef, Purity};
use engine::node_registry::{NodeRegistration, NodeRegistry, NodeSource};
use engine::session::{InteractionEvent, PreviewRequestStatus, Session};
use engine::Engine;
use types::{DataType, Value};

struct SessionTestExecutor {
    call_count: Arc<AtomicUsize>,
}

impl Executor for SessionTestExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "test.preview_value",
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
        let call_count = Arc::clone(&self.call_count);
        Box::pin(async move {
            call_count.fetch_add(1, Ordering::SeqCst);
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

struct SessionTestSource {
    executor: Arc<dyn Executor>,
}

impl NodeSource for SessionTestSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        vec![NodeRegistration {
            static_def: NodeDef {
                type_id: "preview_value".into(),
                version: 1,
                source: NodeSourceKind::Builtin,
                name: "Preview Value".into(),
                category: "test".into(),
                requires: vec!["test.preview_value".into()],
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
                "test.preview_value".into(),
                Arc::clone(&self.executor),
            )],
        }]
    }
}

fn make_engine() -> (Arc<Mutex<Engine>>, Arc<AtomicUsize>) {
    let call_count = Arc::new(AtomicUsize::new(0));
    let executor: Arc<dyn Executor> = Arc::new(SessionTestExecutor {
        call_count: Arc::clone(&call_count),
    });
    let mut registry = NodeRegistry::new();
    registry.register(SessionTestSource { executor });
    (
        Arc::new(Mutex::new(Engine::from_registry_bundle(registry.build(), None))),
        call_count,
    )
}

fn assert_float(value: Option<Value>, expected: f32) {
    assert!(matches!(value, Some(Value::Float(actual)) if (actual - expected).abs() < f32::EPSILON));
}

#[tokio::test]
async fn preview_overlay_executes_without_mutating_committed_graph() {
    let (engine, call_count) = make_engine();
    let mut session = Session::new(Arc::clone(&engine));
    let node_id = engine.lock().unwrap().add_node("preview_value").unwrap();

    session
        .submit_interaction(InteractionEvent::ParamChanged {
            node_id,
            param: "value".into(),
            value: Value::Float(5.0),
            preview: true,
        })
        .unwrap();

    let graph = session.graph_snapshot();
    assert!(matches!(
        graph.nodes.get(&node_id).and_then(|node| node.params.get("value")),
        Some(Value::Float(actual)) if (*actual - 1.0).abs() < f32::EPSILON
    ));

    let status = session.request_preview(ExecuteTarget::Node(node_id)).await.unwrap();
    let ticket = match status {
        PreviewRequestStatus::Executed(ticket) => ticket,
        PreviewRequestStatus::CacheHit => panic!("expected preview execution on cold cache"),
    };
    session.await_execution(ticket.execution_id).unwrap();
    assert_float(session.get_preview(node_id, "value"), 5.0);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn preview_cache_hits_without_reexecuting() {
    let (engine, call_count) = make_engine();
    let mut session = Session::new(Arc::clone(&engine));
    let node_id = engine.lock().unwrap().add_node("preview_value").unwrap();

    session
        .submit_interaction(InteractionEvent::ParamChanged {
            node_id,
            param: "value".into(),
            value: Value::Float(7.0),
            preview: true,
        })
        .unwrap();

    let first = session.request_preview(ExecuteTarget::Node(node_id)).await.unwrap();
    let ticket = match first {
        PreviewRequestStatus::Executed(ticket) => ticket,
        PreviewRequestStatus::CacheHit => panic!("expected preview execution on cold cache"),
    };
    session.await_execution(ticket.execution_id).unwrap();
    assert!(matches!(
        session.request_preview(ExecuteTarget::Node(node_id)).await.unwrap(),
        PreviewRequestStatus::CacheHit
    ));
    assert_float(session.get_preview(node_id, "value"), 7.0);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn committed_param_change_clears_preview_cache() {
    let (engine, call_count) = make_engine();
    let mut session = Session::new(Arc::clone(&engine));
    let node_id = engine.lock().unwrap().add_node("preview_value").unwrap();

    session
        .submit_interaction(InteractionEvent::ParamChanged {
            node_id,
            param: "value".into(),
            value: Value::Float(5.0),
            preview: true,
        })
        .unwrap();
    let first = session.request_preview(ExecuteTarget::Node(node_id)).await.unwrap();
    let ticket = match first {
        PreviewRequestStatus::Executed(ticket) => ticket,
        PreviewRequestStatus::CacheHit => panic!("expected preview execution on cold cache"),
    };
    session.await_execution(ticket.execution_id).unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    session
        .submit_interaction(InteractionEvent::ParamChanged {
            node_id,
            param: "value".into(),
            value: Value::Float(6.0),
            preview: false,
        })
        .unwrap();
    session
        .submit_interaction(InteractionEvent::ParamChanged {
            node_id,
            param: "value".into(),
            value: Value::Float(5.0),
            preview: true,
        })
        .unwrap();

    let second = session.request_preview(ExecuteTarget::Node(node_id)).await.unwrap();
    let ticket = match second {
        PreviewRequestStatus::Executed(ticket) => ticket,
        PreviewRequestStatus::CacheHit => panic!("expected preview re-execution after commit"),
    };
    session.await_execution(ticket.execution_id).unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn preview_execution_does_not_populate_formal_cache() {
    let (engine, call_count) = make_engine();
    let mut session = Session::new(Arc::clone(&engine));
    let node_id = engine.lock().unwrap().add_node("preview_value").unwrap();

    session
        .submit_interaction(InteractionEvent::ParamChanged {
            node_id,
            param: "value".into(),
            value: Value::Float(9.0),
            preview: true,
        })
        .unwrap();
    let preview = session.request_preview(ExecuteTarget::Node(node_id)).await.unwrap();
    let ticket = match preview {
        PreviewRequestStatus::Executed(ticket) => ticket,
        PreviewRequestStatus::CacheHit => panic!("expected preview execution on cold cache"),
    };
    session.await_execution(ticket.execution_id).unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    session.discard_preview();
    let ticket = session.request_export(ExecuteTarget::Node(node_id)).await.unwrap();
    session.await_execution(ticket.execution_id).unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 2);

    let outputs = engine
        .lock()
        .unwrap()
        .query_execution_outputs(ticket.execution_id, node_id)
        .unwrap();
    assert!(matches!(outputs.get("value"), Some(Value::Float(actual)) if (*actual - 1.0).abs() < f32::EPSILON));
}
