use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use engine::capability::{Capability, CapabilityId, SideEffect};
use engine::events::{
    CancellationReason, CancellationSubject, EngineEvent, ExecutionStatus, QueueReason,
};
use engine::execution::{
    EvaluationFidelity, ExecutionMode, ExecutionOutputs, ExecutorError, NodeExecutionRequest,
};
use engine::executors::{Executor, ExecutorFuture, LocalityProfile};
use engine::facade::{EngineError, EngineFacade, ExecutionRequest, ExecutionRequestResult};
use engine::graph::model::subgraph::ExecuteTarget;
use engine::node_manager::{ExecutionPolicy, NodeDef, NodeSourceKind, PinDef, Purity};
use engine::node_registry::{NodeRegistration, NodeRegistry, NodeSource};
use engine::Engine;
use types::{DataType, Value};

struct SingleFlightExecutor;

impl Executor for SingleFlightExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "test.single_flight",
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
            let steps = match req.fidelity {
                EvaluationFidelity::Preview { .. } => 4,
                EvaluationFidelity::Full => 12,
            };
            for _ in 0..steps {
                if req.cancel_token.is_cancelled() {
                    return Err(ExecutorError::Cancelled);
                }
                std::thread::sleep(Duration::from_millis(10));
            }

            let value = match req.fidelity {
                EvaluationFidelity::Preview { .. } => 0.5,
                EvaluationFidelity::Full => 1.0,
            };
            Ok(match req.fidelity.clone() {
                EvaluationFidelity::Preview { .. } => ExecutionOutputs::preview(
                    HashMap::from([("value".to_string(), Value::Float(value))]),
                    req.fidelity,
                ),
                EvaluationFidelity::Full => ExecutionOutputs::full(HashMap::from([(
                    "value".to_string(),
                    Value::Float(value),
                )])),
            })
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

struct SingleFlightSource;

impl NodeSource for SingleFlightSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        vec![NodeRegistration {
            static_def: NodeDef {
                type_id: "single_flight_value".into(),
                version: 1,
                source: NodeSourceKind::Builtin,
                name: "Single Flight Value".into(),
                category: "test".into(),
                requires: vec!["test.single_flight".into()],
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
                "test.single_flight".into(),
                Arc::new(SingleFlightExecutor),
            )],
        }]
    }
}

fn make_engine() -> Engine {
    let mut registry = NodeRegistry::new();
    registry.register(SingleFlightSource);
    Engine::from_registry_bundle(registry.build(), None)
}

fn preview_request(node_id: types::NodeId) -> ExecutionRequest {
    ExecutionRequest {
        target: ExecuteTarget::Node(node_id),
        mode: Some(ExecutionMode::OneShot {
            fidelity: EvaluationFidelity::Preview {
                max_side: Some(128),
                precision: engine::execution::PreviewPrecision::Float16,
            },
        }),
    }
}

fn full_request(node_id: types::NodeId) -> ExecutionRequest {
    ExecutionRequest {
        target: ExecuteTarget::Node(node_id),
        mode: Some(ExecutionMode::OneShot {
            fidelity: EvaluationFidelity::Full,
        }),
    }
}

#[tokio::test]
async fn full_request_is_rejected_while_full_is_running() {
    let mut engine = make_engine();
    let node_id = engine.add_node("single_flight_value").unwrap();
    let started = engine.execute_request(full_request(node_id)).await.unwrap();
    let started = match started {
        ExecutionRequestResult::Started(ticket) => ticket,
        ExecutionRequestResult::Queued { .. } => panic!("expected first full request to start"),
    };

    let second = engine.execute_request(full_request(node_id)).await;
    assert!(matches!(second, Err(EngineError::RunInFlight)));

    engine.cancel_execution(started.execution_id).unwrap();
    assert_eq!(
        engine.await_execution(started.execution_id).unwrap(),
        engine::execution::ExecutionTerminalStatus::Cancelled
    );
}

#[tokio::test]
async fn full_request_queues_behind_preview() {
    let mut engine = make_engine();
    let node_id = engine.add_node("single_flight_value").unwrap();
    let preview = engine
        .execute_request(preview_request(node_id))
        .await
        .unwrap();
    let preview = match preview {
        ExecutionRequestResult::Started(ticket) => ticket,
        ExecutionRequestResult::Queued { .. } => panic!("expected preview to start"),
    };

    let queued = engine.execute_request(full_request(node_id)).await.unwrap();
    let pending_id = match queued {
        ExecutionRequestResult::Started(_) => {
            panic!("expected full request to queue behind preview")
        }
        ExecutionRequestResult::Queued { pending_id, reason } => {
            assert_eq!(reason, QueueReason::BehindPreview);
            pending_id
        }
    };

    let state = engine.query_state();
    assert_eq!(state.status, ExecutionStatus::Running);
    assert!(matches!(state.pending_full, Some(ref pending) if pending.pending_id == pending_id));

    engine.await_execution(preview.execution_id).unwrap();

    let mut saw_resubmitted_start = false;
    for _ in 0..20 {
        let state = engine.query_state();
        if matches!(state.current, Some(ref current) if current.from_pending_id == Some(pending_id))
        {
            saw_resubmitted_start = true;
            engine
                .await_execution(state.current.unwrap().execution_id)
                .unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(
        saw_resubmitted_start,
        "pending full should be resubmitted after preview completes"
    );
}

#[tokio::test]
async fn preview_preempts_running_full_and_keeps_full_pending() {
    let mut engine = make_engine();
    let node_id = engine.add_node("single_flight_value").unwrap();
    let full = engine.execute_request(full_request(node_id)).await.unwrap();
    let full = match full {
        ExecutionRequestResult::Started(ticket) => ticket,
        ExecutionRequestResult::Queued { .. } => panic!("expected full to start"),
    };

    let preview = engine
        .execute_request(preview_request(node_id))
        .await
        .unwrap();
    let preview = match preview {
        ExecutionRequestResult::Started(ticket) => ticket,
        ExecutionRequestResult::Queued { .. } => {
            panic!("expected preview to start after preempting full")
        }
    };

    let state = engine.query_state();
    assert!(
        matches!(state.current, Some(ref current) if current.execution_id == preview.execution_id)
    );
    assert!(state.pending_full.is_some());

    let cancelled = engine.await_execution(full.execution_id).unwrap();
    assert_eq!(
        cancelled,
        engine::execution::ExecutionTerminalStatus::Cancelled
    );

    let events = engine.engine_events_snapshot();
    assert!(events.iter().any(|record| matches!(
        &record.payload,
        EngineEvent::ExecutionCancelled {
            subject: CancellationSubject::Execution(id),
            reason: CancellationReason::ReplacedByPreview,
        } if *id == full.execution_id
    )));

    engine.await_execution(preview.execution_id).unwrap();
}

#[tokio::test]
async fn cancel_execution_marks_running_run_as_cancelled() {
    let mut engine = make_engine();
    let node_id = engine.add_node("single_flight_value").unwrap();
    let started = engine.execute_request(full_request(node_id)).await.unwrap();
    let started = match started {
        ExecutionRequestResult::Started(ticket) => ticket,
        ExecutionRequestResult::Queued { .. } => panic!("expected run to start"),
    };

    engine.cancel_execution(started.execution_id).unwrap();
    assert_eq!(engine.query_state().status, ExecutionStatus::Cancelling);
    assert_eq!(
        engine.await_execution(started.execution_id).unwrap(),
        engine::execution::ExecutionTerminalStatus::Cancelled
    );
}
