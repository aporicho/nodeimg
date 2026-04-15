use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use engine::capability::{Capability, CapabilityId, SideEffect};
use engine::events::ExecutionStatus;
use engine::execution::{ExecutionOutputs, NodeExecutionRequest};
use engine::executors::{Executor, ExecutorFuture, LocalityProfile};
use engine::facade::{EngineFacade, EngineResourceConfig, ExecutionRequest, ExecutionRequestResult};
use engine::graph::model::subgraph::ExecuteTarget;
use engine::node_manager::{
    ArtifactPolicy, ExecutionPolicy, NodeDef, NodeSourceKind, ParamDef, ParamExpose, PinDef,
    Purity, TriggerPolicy,
};
use engine::node_registry::{NodeRegistration, NodeRegistry, NodeSource};
use engine::Engine;
use image::{DynamicImage, RgbaImage};
use types::{DataType, Value};

struct ArtifactImageGenExecutor {
    call_count: Arc<AtomicUsize>,
}

impl Executor for ArtifactImageGenExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "test.artifact.image_gen",
            Vec::new(),
            vec![DataType::image()],
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
            let prompt = match req.inputs.get("prompt") {
                Some(Value::String(prompt)) => prompt.clone(),
                _ => String::new(),
            };
            let color = match prompt.as_str() {
                "blue" => [0, 0, 255, 255],
                "green" => [0, 255, 0, 255],
                _ => [255, 0, 0, 255],
            };
            let rgba = RgbaImage::from_pixel(4, 4, image::Rgba(color));
            Ok(ExecutionOutputs::full(HashMap::from([(
                String::from("image"),
                Value::Image(types::Image::from_cpu(DynamicImage::ImageRgba8(rgba))),
            )])))
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

struct ArtifactPassthroughExecutor {
    call_count: Arc<AtomicUsize>,
}

impl Executor for ArtifactPassthroughExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "test.artifact.passthrough",
            vec![DataType::image()],
            vec![DataType::image()],
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
            let image = req
                .inputs
                .get("image")
                .cloned()
                .ok_or_else(|| engine::execution::ExecutorError::InvalidInput {
                    message: "missing image input".into(),
                })?;
            Ok(ExecutionOutputs::full(HashMap::from([(
                String::from("image"),
                image,
            )])))
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

struct RuntimeArtifactSource {
    image_executor: Arc<dyn Executor>,
    passthrough_executor: Arc<dyn Executor>,
}

impl NodeSource for RuntimeArtifactSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        vec![
            NodeRegistration {
                static_def: NodeDef {
                    type_id: "image_gen".into(),
                    version: 1,
                    source: NodeSourceKind::Api,
                    name: "Image Generation".into(),
                    category: "image/generation".into(),
                    requires: vec!["test.artifact.image_gen".into()],
                    purity: Purity::Impure,
                    cooking_sensitivity: vec![],
                    realtime_capable: false,
                    execution: ExecutionPolicy {
                        timeout_ms: Some(30_000),
                        artifact_policy: Some(ArtifactPolicy::Persist),
                        trigger_policy: TriggerPolicy::ManualOnly,
                        ..ExecutionPolicy::default()
                    },
                    api: None,
                    inputs: vec![],
                    outputs: vec![PinDef {
                        name: "image".into(),
                        data_type: DataType::image(),
                        optional: false,
                    }],
                    params: vec![ParamDef {
                        name: "prompt".into(),
                        data_type: DataType::string(),
                        constraint: None,
                        default_value: Value::String(String::new()),
                        expose: vec![ParamExpose::Control],
                    }],
                    execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
                },
                schema_provider: None,
                presentation_provider: None,
                capability_bindings: vec![(
                    "test.artifact.image_gen".into(),
                    Arc::clone(&self.image_executor),
                )],
            },
            NodeRegistration {
                static_def: NodeDef {
                    type_id: "artifact_passthrough".into(),
                    version: 1,
                    source: NodeSourceKind::Builtin,
                    name: "Artifact Passthrough".into(),
                    category: "test".into(),
                    requires: vec!["test.artifact.passthrough".into()],
                    purity: Purity::Pure,
                    cooking_sensitivity: vec![],
                    realtime_capable: true,
                    execution: ExecutionPolicy::default(),
                    api: None,
                    inputs: vec![PinDef {
                        name: "image".into(),
                        data_type: DataType::image(),
                        optional: false,
                    }],
                    outputs: vec![PinDef {
                        name: "image".into(),
                        data_type: DataType::image(),
                        optional: false,
                    }],
                    params: vec![],
                    execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
                },
                schema_provider: None,
                presentation_provider: None,
                capability_bindings: vec![(
                    "test.artifact.passthrough".into(),
                    Arc::clone(&self.passthrough_executor),
                )],
            },
        ]
    }
}

fn build_registry(
    image_call_count: Arc<AtomicUsize>,
    passthrough_call_count: Arc<AtomicUsize>,
) -> engine::node_registry::RegistryBundle {
    let image_executor: Arc<dyn Executor> = Arc::new(ArtifactImageGenExecutor {
        call_count: image_call_count,
    });
    let passthrough_executor: Arc<dyn Executor> = Arc::new(ArtifactPassthroughExecutor {
        call_count: passthrough_call_count,
    });
    let mut registry = NodeRegistry::new();
    registry.register(RuntimeArtifactSource {
        image_executor,
        passthrough_executor,
    });
    registry.build()
}

fn make_temp_root() -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("nodeimg-runtime-artifacts-{unique}-{seq}"));
    std::fs::create_dir_all(&root).expect("create artifact temp root");
    root
}

fn started_ticket(result: ExecutionRequestResult) -> engine::facade::ExecutionTicket {
    match result {
        ExecutionRequestResult::Started(ticket) => ticket,
        ExecutionRequestResult::Queued { .. } => panic!("expected execution to start immediately"),
    }
}

#[tokio::test]
async fn full_image_generation_persists_artifact_and_params_snapshot() {
    let image_call_count = Arc::new(AtomicUsize::new(0));
    let passthrough_call_count = Arc::new(AtomicUsize::new(0));
    let artifact_root = make_temp_root();
    let mut engine = Engine::from_registry_bundle_with_resources(
        build_registry(Arc::clone(&image_call_count), Arc::clone(&passthrough_call_count)),
        None,
        EngineResourceConfig {
            artifact_root: Some(artifact_root),
        },
    );
    let node_id = engine.add_node("image_gen").unwrap();
    engine.set_param(node_id, "prompt", Value::String("red".into()), false);

    let ticket = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(node_id),
                mode: Some(engine::execution::ExecutionMode::OneShot {
                    fidelity: engine::execution::EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(ticket.execution_id).unwrap();

    assert_eq!(image_call_count.load(Ordering::SeqCst), 1);
    let history = engine.query_artifact_history(node_id, "image").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].params_snapshot.get("prompt"), Some(&String::from("red")));
    assert!(history[0].path.exists());
    let selected = engine.query_selected_artifact(node_id, "image").unwrap().unwrap();
    assert_eq!(selected.artifact_id, history[0].artifact_id);
}

#[tokio::test]
async fn full_image_generation_restores_selected_artifact_across_engine_instances() {
    let image_call_count = Arc::new(AtomicUsize::new(0));
    let passthrough_call_count = Arc::new(AtomicUsize::new(0));
    let artifact_root = make_temp_root();

    {
        let mut engine = Engine::from_registry_bundle_with_resources(
            build_registry(Arc::clone(&image_call_count), Arc::clone(&passthrough_call_count)),
            None,
            EngineResourceConfig {
                artifact_root: Some(artifact_root.clone()),
            },
        );
        let node_id = engine.add_node("image_gen").unwrap();
        engine.set_param(node_id, "prompt", Value::String("red".into()), false);
        let ticket = started_ticket(
            engine
                .execute_request(ExecutionRequest {
                    target: ExecuteTarget::Node(node_id),
                    mode: Some(engine::execution::ExecutionMode::OneShot {
                        fidelity: engine::execution::EvaluationFidelity::Full,
                    }),
                })
                .await
                .unwrap(),
        );
        engine.await_execution(ticket.execution_id).unwrap();
    }

    let mut engine = Engine::from_registry_bundle_with_resources(
        build_registry(Arc::clone(&image_call_count), Arc::clone(&passthrough_call_count)),
        None,
        EngineResourceConfig {
            artifact_root: Some(artifact_root),
        },
    );
    let node_id = engine.add_node("image_gen").unwrap();
    engine.set_param(node_id, "prompt", Value::String("red".into()), false);
    let ticket = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(node_id),
                mode: Some(engine::execution::ExecutionMode::OneShot {
                    fidelity: engine::execution::EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(ticket.execution_id).unwrap();

    assert_eq!(image_call_count.load(Ordering::SeqCst), 1);
    assert_eq!(engine.query_artifact_history(node_id, "image").unwrap().len(), 1);
}

#[tokio::test]
async fn selecting_older_artifact_restores_it_without_reexecuting_generator() {
    let image_call_count = Arc::new(AtomicUsize::new(0));
    let passthrough_call_count = Arc::new(AtomicUsize::new(0));
    let artifact_root = make_temp_root();
    let mut engine = Engine::from_registry_bundle_with_resources(
        build_registry(Arc::clone(&image_call_count), Arc::clone(&passthrough_call_count)),
        None,
        EngineResourceConfig {
            artifact_root: Some(artifact_root),
        },
    );
    let gen = engine.add_node("image_gen").unwrap();
    let pass = engine.add_node("artifact_passthrough").unwrap();
    engine
        .connect(engine::graph::Connection {
            from: engine::graph::PinRef {
                node: gen,
                interface: "image".into(),
            },
            to: engine::graph::PinRef {
                node: pass,
                interface: "image".into(),
            },
        })
        .unwrap();

    engine.set_param(gen, "prompt", Value::String("red".into()), false);
    let first = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(pass),
                mode: Some(engine::execution::ExecutionMode::OneShot {
                    fidelity: engine::execution::EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(first.execution_id).unwrap();

    engine.set_param(gen, "prompt", Value::String("blue".into()), false);
    let second = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(pass),
                mode: Some(engine::execution::ExecutionMode::OneShot {
                    fidelity: engine::execution::EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(second.execution_id).unwrap();

    assert_eq!(image_call_count.load(Ordering::SeqCst), 2);
    assert_eq!(passthrough_call_count.load(Ordering::SeqCst), 2);

    let history = engine.query_artifact_history(gen, "image").unwrap();
    assert_eq!(history.len(), 2);
    let red = history
        .iter()
        .find(|record| record.params_snapshot.get("prompt") == Some(&String::from("red")))
        .expect("red artifact");
    engine
        .select_artifact_version(gen, "image", &red.artifact_id)
        .unwrap();
    let graph = engine.query_graph_snapshot();
    assert!(matches!(
        graph.nodes.get(&gen).and_then(|node| node.params.get("prompt")),
        Some(Value::String(prompt)) if prompt == "red"
    ));

    let third = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(pass),
                mode: Some(engine::execution::ExecutionMode::OneShot {
                    fidelity: engine::execution::EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(third.execution_id).unwrap();

    assert_eq!(image_call_count.load(Ordering::SeqCst), 2);
    assert_eq!(passthrough_call_count.load(Ordering::SeqCst), 3);
    assert_eq!(engine.query_selected_artifact(gen, "image").unwrap().unwrap().artifact_id, red.artifact_id);
    assert_eq!(engine.execution_state().status, ExecutionStatus::Idle);
}

#[tokio::test]
async fn preview_never_persists_or_restores_artifacts() {
    let image_call_count = Arc::new(AtomicUsize::new(0));
    let passthrough_call_count = Arc::new(AtomicUsize::new(0));
    let artifact_root = make_temp_root();
    let mut engine = Engine::from_registry_bundle_with_resources(
        build_registry(Arc::clone(&image_call_count), Arc::clone(&passthrough_call_count)),
        None,
        EngineResourceConfig {
            artifact_root: Some(artifact_root),
        },
    );
    let node_id = engine.add_node("image_gen").unwrap();
    engine.set_param(node_id, "prompt", Value::String("red".into()), false);

    let preview = engine
        .execute_request(ExecutionRequest {
            target: ExecuteTarget::Node(node_id),
            mode: Some(engine::execution::ExecutionMode::OneShot {
                fidelity: engine::execution::EvaluationFidelity::Preview {
                    max_side: Some(256),
                    precision: engine::execution::PreviewPrecision::Float16,
                },
            }),
        })
        .await
        .unwrap();
    let preview = started_ticket(preview);
    engine.await_execution(preview.execution_id).unwrap();

    assert_eq!(image_call_count.load(Ordering::SeqCst), 1);
    assert!(engine.query_artifact_history(node_id, "image").unwrap().is_empty());
}
