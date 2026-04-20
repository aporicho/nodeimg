use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use engine::capability::{Capability, CapabilityId, SideEffect};
use engine::execution::{
    EvaluationFidelity, ExecutionMode, ExecutionOutputs, NodeExecutionRequest,
};
use engine::executors::video::encode_frames_to_mp4;
use engine::executors::{Executor, ExecutorFuture, LocalityProfile};
use engine::facade::{
    EngineFacade, EngineResourceConfig, ExecutionRequest, ExecutionRequestResult,
};
use engine::graph::model::subgraph::ExecuteTarget;
use engine::node_manager::{
    ArtifactPolicy, ExecutionPolicy, NodeDef, NodeSourceKind, ParamDef, ParamExpose, PinDef,
    Purity, TriggerPolicy,
};
use engine::node_registry::{NodeRegistration, NodeRegistry, NodeSource};
use engine::Engine;
use image::{DynamicImage, RgbaImage};
use types::{DataType, Value};

struct VideoArtifactExecutor {
    call_count: Arc<AtomicUsize>,
}

impl Executor for VideoArtifactExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "test.artifact.video_gen",
            vec![],
            vec![DataType::video(), DataType::image(), DataType::int()],
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
            let fps = match req.inputs.get("fps") {
                Some(Value::Int(value)) if *value > 0 => *value as u32,
                Some(Value::Float(value)) if *value > 0.0 => *value as u32,
                _ => 8,
            };
            let width = match req.inputs.get("width") {
                Some(Value::Int(value)) if *value > 0 => *value as u32,
                _ => 16,
            };
            let height = match req.inputs.get("height") {
                Some(Value::Int(value)) if *value > 0 => *value as u32,
                _ => 16,
            };
            let duration_seconds = match req.inputs.get("duration_seconds") {
                Some(Value::Int(value)) if *value > 0 => *value as u32,
                _ => 1,
            };
            let frame_count = duration_seconds.saturating_mul(fps).max(1);
            let frames = (0..frame_count)
                .map(|frame| build_frame(width, height, &prompt, frame))
                .collect::<Vec<_>>();
            let video_path = temp_video_path("nodeimg-video-artifact-runtime");
            encode_video(&video_path, &frames, fps)?;
            let preview = frames[0].clone();

            Ok(ExecutionOutputs::full(HashMap::from([
                (
                    String::from("video"),
                    Value::String(video_path.to_string_lossy().into_owned()),
                ),
                (
                    String::from("image"),
                    Value::Image(types::Image::from_cpu(preview)),
                ),
                (String::from("fps"), Value::Int(i64::from(fps))),
            ])))
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

struct VideoArtifactSource {
    executor: Arc<dyn Executor>,
}

impl NodeSource for VideoArtifactSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        vec![NodeRegistration {
            static_def: NodeDef {
                type_id: "ai_video_generate_api".into(),
                version: 1,
                source: NodeSourceKind::Api,
                name: "AI Video Generate API".into(),
                category: "ai/generation".into(),
                requires: vec!["test.artifact.video_gen".into()],
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
                outputs: vec![
                    PinDef {
                        name: "video".into(),
                        data_type: DataType::video(),
                        optional: false,
                    },
                    PinDef {
                        name: "image".into(),
                        data_type: DataType::image(),
                        optional: false,
                    },
                    PinDef {
                        name: "fps".into(),
                        data_type: DataType::int(),
                        optional: false,
                    },
                ],
                params: vec![
                    ParamDef {
                        name: "provider".into(),
                        data_type: DataType::string(),
                        constraint: None,
                        default_value: Value::String(String::from("mock")),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "model".into(),
                        data_type: DataType::string(),
                        constraint: None,
                        default_value: Value::String(String::from("mock-wave")),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "prompt".into(),
                        data_type: DataType::string(),
                        constraint: None,
                        default_value: Value::String(String::new()),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "duration_seconds".into(),
                        data_type: DataType::int(),
                        constraint: None,
                        default_value: Value::Int(1),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "fps".into(),
                        data_type: DataType::int(),
                        constraint: None,
                        default_value: Value::Int(0),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "width".into(),
                        data_type: DataType::int(),
                        constraint: None,
                        default_value: Value::Int(16),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "height".into(),
                        data_type: DataType::int(),
                        constraint: None,
                        default_value: Value::Int(16),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "seed".into(),
                        data_type: DataType::int(),
                        constraint: None,
                        default_value: Value::Int(0),
                        expose: vec![ParamExpose::Control],
                    },
                ],
                execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
            },
            schema_provider: None,
            presentation_provider: None,
            capability_bindings: vec![(
                String::from("test.artifact.video_gen"),
                Arc::clone(&self.executor),
            )],
        }]
    }
}

fn build_registry(call_count: Arc<AtomicUsize>) -> engine::node_registry::RegistryBundle {
    let executor: Arc<dyn Executor> = Arc::new(VideoArtifactExecutor { call_count });
    let mut registry = NodeRegistry::new();
    registry.register(VideoArtifactSource { executor });
    registry.build()
}

fn make_temp_root() -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("nodeimg-video-artifacts-{unique}-{seq}"));
    std::fs::create_dir_all(&root).expect("create artifact temp root");
    root
}

fn temp_video_path(prefix: &str) -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{unique}-{seq}.mp4"))
}

fn encode_video(
    path: &PathBuf,
    frames: &[DynamicImage],
    fps: u32,
) -> Result<(), engine::execution::ExecutorError> {
    let frames_dir = std::env::temp_dir().join(format!(
        "nodeimg-video-artifact-frames-{}-{}",
        std::process::id(),
        NEXT_TMP.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&frames_dir).expect("create frames dir");
    for (index, frame) in frames.iter().enumerate() {
        frame
            .save(frames_dir.join(format!("frame_{index:06}.png")))
            .expect("save frame");
    }
    let result = encode_frames_to_mp4(&frames_dir, path, fps);
    let _ = std::fs::remove_dir_all(&frames_dir);
    result
}

static NEXT_TMP: AtomicU64 = AtomicU64::new(1);

fn build_frame(width: u32, height: u32, prompt: &str, frame: u32) -> DynamicImage {
    let mut rgba = RgbaImage::new(width, height);
    let seed = prompt.bytes().fold(0_u32, |acc, byte| {
        acc.wrapping_mul(33).wrapping_add(byte as u32)
    });
    let tint = ((seed as u8).wrapping_add(frame as u8)).max(1);
    for pixel in rgba.pixels_mut() {
        *pixel = image::Rgba([tint, 255_u8.wrapping_sub(tint), tint / 2, 255]);
    }
    DynamicImage::ImageRgba8(rgba)
}

fn started_ticket(result: ExecutionRequestResult) -> engine::facade::ExecutionTicket {
    match result {
        ExecutionRequestResult::Started(ticket) => ticket,
        ExecutionRequestResult::Queued { .. } => panic!("expected execution to start immediately"),
    }
}

#[tokio::test]
async fn full_video_generation_persists_artifact_and_params_snapshot() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let artifact_root = make_temp_root();
    let mut engine = Engine::from_registry_bundle_with_resources(
        build_registry(Arc::clone(&call_count)),
        None,
        EngineResourceConfig {
            artifact_root: Some(artifact_root),
        },
    );
    let node_id = engine.add_node("ai_video_generate_api").unwrap();
    engine.set_param(
        node_id,
        "prompt",
        Value::String(String::from("ocean")),
        false,
    );
    engine.set_param(node_id, "fps", Value::Int(0), false);

    let ticket = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(node_id),
                mode: Some(ExecutionMode::OneShot {
                    fidelity: EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(ticket.execution_id).unwrap();

    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    let outputs = engine
        .query_execution_outputs(ticket.execution_id, node_id)
        .unwrap();
    let video_path = match outputs.get("video") {
        Some(Value::String(path)) => PathBuf::from(path),
        other => panic!("expected video output, got {other:?}"),
    };
    assert!(video_path.exists());

    let history = engine.query_artifact_history(node_id, "video").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(
        history[0].params_snapshot.get("prompt"),
        Some(&String::from("ocean"))
    );
    assert_eq!(
        history[0].params_snapshot.get("fps"),
        Some(&String::from("8"))
    );
    assert!(history[0].path.exists());
}

#[tokio::test]
async fn full_video_generation_restores_selected_artifact_across_engines() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let artifact_root = make_temp_root();

    {
        let mut engine = Engine::from_registry_bundle_with_resources(
            build_registry(Arc::clone(&call_count)),
            None,
            EngineResourceConfig {
                artifact_root: Some(artifact_root.clone()),
            },
        );
        let node_id = engine.add_node("ai_video_generate_api").unwrap();
        engine.set_param(
            node_id,
            "prompt",
            Value::String(String::from("ocean")),
            false,
        );
        engine.set_param(node_id, "fps", Value::Int(0), false);
        let ticket = started_ticket(
            engine
                .execute_request(ExecutionRequest {
                    target: ExecuteTarget::Node(node_id),
                    mode: Some(ExecutionMode::OneShot {
                        fidelity: EvaluationFidelity::Full,
                    }),
                })
                .await
                .unwrap(),
        );
        engine.await_execution(ticket.execution_id).unwrap();
    }

    let mut engine = Engine::from_registry_bundle_with_resources(
        build_registry(Arc::clone(&call_count)),
        None,
        EngineResourceConfig {
            artifact_root: Some(artifact_root),
        },
    );
    let node_id = engine.add_node("ai_video_generate_api").unwrap();
    engine.set_param(
        node_id,
        "prompt",
        Value::String(String::from("ocean")),
        false,
    );
    engine.set_param(node_id, "fps", Value::Int(0), false);

    let ticket = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(node_id),
                mode: Some(ExecutionMode::OneShot {
                    fidelity: EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(ticket.execution_id).unwrap();

    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    let outputs = engine
        .query_execution_outputs(ticket.execution_id, node_id)
        .unwrap();
    assert!(matches!(outputs.get("fps"), Some(Value::Int(8))));
    let video_path = match outputs.get("video") {
        Some(Value::String(path)) => PathBuf::from(path),
        other => panic!("expected video output, got {other:?}"),
    };
    assert!(video_path.exists());
    assert_eq!(
        engine
            .query_artifact_history(node_id, "video")
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn selecting_video_artifact_syncs_params_and_restores_without_reexecution() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let artifact_root = make_temp_root();
    let mut engine = Engine::from_registry_bundle_with_resources(
        build_registry(Arc::clone(&call_count)),
        None,
        EngineResourceConfig {
            artifact_root: Some(artifact_root),
        },
    );
    let node_id = engine.add_node("ai_video_generate_api").unwrap();

    engine.set_param(
        node_id,
        "prompt",
        Value::String(String::from("ocean")),
        false,
    );
    engine.set_param(node_id, "fps", Value::Int(0), false);
    let first = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(node_id),
                mode: Some(ExecutionMode::OneShot {
                    fidelity: EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(first.execution_id).unwrap();

    engine.set_param(
        node_id,
        "prompt",
        Value::String(String::from("forest")),
        false,
    );
    engine.set_param(node_id, "fps", Value::Int(12), false);
    let second = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(node_id),
                mode: Some(ExecutionMode::OneShot {
                    fidelity: EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(second.execution_id).unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 2);

    let history = engine.query_artifact_history(node_id, "video").unwrap();
    let ocean = history
        .iter()
        .find(|record| record.params_snapshot.get("prompt") == Some(&String::from("ocean")))
        .expect("ocean video artifact");
    engine
        .select_artifact_version(node_id, "video", &ocean.artifact_id)
        .unwrap();

    let graph = engine.query_graph_snapshot();
    let node = graph.nodes.get(&node_id).expect("video node");
    assert!(matches!(node.params.get("prompt"), Some(Value::String(prompt)) if prompt == "ocean"));
    assert!(matches!(node.params.get("fps"), Some(Value::Int(8))));

    let third = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Node(node_id),
                mode: Some(ExecutionMode::OneShot {
                    fidelity: EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(third.execution_id).unwrap();

    assert_eq!(call_count.load(Ordering::SeqCst), 2);
    let outputs = engine
        .query_execution_outputs(third.execution_id, node_id)
        .unwrap();
    let restored_path = match outputs.get("video") {
        Some(Value::String(path)) => PathBuf::from(path),
        other => panic!("expected video output, got {other:?}"),
    };
    assert_eq!(restored_path, ocean.path);
}
