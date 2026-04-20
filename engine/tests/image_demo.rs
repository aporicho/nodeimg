use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use engine::capability::{Capability, CapabilityId, SideEffect};
use engine::execution::{
    EvaluationFidelity, ExecutionMode, ExecutionOutputs, NodeExecutionRequest,
};
use engine::executors::{Executor, ExecutorFuture, LocalityProfile};
use engine::facade::{
    EngineFacade, EngineResourceConfig, ExecutionRequest, ExecutionRequestResult,
};
use engine::graph::model::subgraph::ExecuteTarget;
use engine::graph::{Connection, PinRef};
use engine::node_manager::{
    ArtifactPolicy, ExecutionPolicy, NodeDef, NodeSourceKind, ParamDef, ParamExpose, PinDef,
    Purity, TriggerPolicy,
};
use engine::node_registry::{sources, NodeRegistration, NodeRegistry, NodeSource};
use engine::Engine;
use image::{DynamicImage, RgbaImage};
use types::{DataType, Value};

struct DemoImageGenExecutor {
    call_count: Arc<AtomicUsize>,
}

impl Executor for DemoImageGenExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "demo.image.generate",
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
                Some(Value::String(prompt)) => prompt.as_str(),
                _ => "",
            };
            let color = match prompt {
                "blue candidate" => [0, 0, 255, 255],
                "green candidate" => [0, 255, 0, 255],
                _ => [255, 0, 0, 255],
            };
            let rgba = RgbaImage::from_pixel(8, 8, image::Rgba(color));
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

struct DemoImageSource {
    executor: Arc<dyn Executor>,
}

impl NodeSource for DemoImageSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        vec![NodeRegistration {
            static_def: NodeDef {
                type_id: "image_gen".into(),
                version: 1,
                source: NodeSourceKind::Api,
                name: "Demo Image Generate".into(),
                category: "demo/image".into(),
                requires: vec!["demo.image.generate".into()],
                purity: Purity::Impure,
                cooking_sensitivity: Vec::new(),
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
            capability_bindings: vec![("demo.image.generate".into(), Arc::clone(&self.executor))],
        }]
    }
}

fn build_registry(call_count: Arc<AtomicUsize>) -> engine::node_registry::RegistryBundle {
    let mut registry = NodeRegistry::new();
    registry.register(sources::InventoryNodeSource);
    registry.register(sources::ColorAdjustSource);
    registry.register(DemoImageSource {
        executor: Arc::new(DemoImageGenExecutor { call_count }),
    });
    registry.build()
}

fn started_ticket(result: ExecutionRequestResult) -> engine::facade::ExecutionTicket {
    match result {
        ExecutionRequestResult::Started(ticket) => ticket,
        ExecutionRequestResult::Queued { .. } => panic!("expected execution to start immediately"),
    }
}

fn make_temp_root(prefix: &str) -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("{prefix}-{unique}-{seq}"));
    std::fs::create_dir_all(&root).expect("create temp root");
    root
}

#[tokio::test]
async fn image_demo_generates_history_selects_version_processes_and_exports() {
    let generate_count = Arc::new(AtomicUsize::new(0));
    let artifact_root = make_temp_root("nodeimg-image-demo-artifacts");
    let output_root = make_temp_root("nodeimg-image-demo-output");
    let output_path = output_root.join("selected.png");

    let mut engine = Engine::from_registry_bundle_with_resources(
        build_registry(Arc::clone(&generate_count)),
        None,
        EngineResourceConfig {
            artifact_root: Some(artifact_root),
        },
    );

    let gen = engine.add_node("image_gen").unwrap();
    let grade = engine.add_node("color_adjust").unwrap();
    let save = engine.add_node("save_image").unwrap();

    engine
        .connect(Connection {
            from: PinRef {
                node: gen,
                interface: "image".into(),
            },
            to: PinRef {
                node: grade,
                interface: "image".into(),
            },
        })
        .unwrap();
    engine
        .connect(Connection {
            from: PinRef {
                node: grade,
                interface: "image".into(),
            },
            to: PinRef {
                node: save,
                interface: "image".into(),
            },
        })
        .unwrap();

    engine.set_param(grade, "brightness", Value::Float(0.05), false);
    engine.set_param(grade, "contrast", Value::Float(1.0), false);
    engine.set_param(grade, "saturation", Value::Float(1.0), false);
    engine.set_param(
        save,
        "path",
        Value::String(output_path.to_string_lossy().into_owned()),
        false,
    );

    engine.set_param(gen, "prompt", Value::String("red candidate".into()), false);
    let first = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Graph,
                mode: Some(ExecutionMode::OneShot {
                    fidelity: EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(first.execution_id).unwrap();

    engine.set_param(gen, "prompt", Value::String("blue candidate".into()), false);
    let second = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Graph,
                mode: Some(ExecutionMode::OneShot {
                    fidelity: EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(second.execution_id).unwrap();

    assert_eq!(generate_count.load(Ordering::SeqCst), 2);
    let history = engine.query_artifact_history(gen, "image").unwrap();
    assert_eq!(history.len(), 2);

    let red = history
        .iter()
        .find(|record| record.params_snapshot.get("prompt") == Some(&String::from("red candidate")))
        .expect("red candidate artifact");
    engine
        .select_artifact_version(gen, "image", &red.artifact_id)
        .unwrap();

    let third = started_ticket(
        engine
            .execute_request(ExecutionRequest {
                target: ExecuteTarget::Graph,
                mode: Some(ExecutionMode::OneShot {
                    fidelity: EvaluationFidelity::Full,
                }),
            })
            .await
            .unwrap(),
    );
    engine.await_execution(third.execution_id).unwrap();

    assert_eq!(
        generate_count.load(Ordering::SeqCst),
        2,
        "selecting an existing artifact should restore generation output without rerunning generator"
    );
    assert_eq!(
        engine
            .query_selected_artifact(gen, "image")
            .unwrap()
            .unwrap()
            .artifact_id,
        red.artifact_id
    );
    assert!(output_path.exists());

    let exported = image::open(&output_path).unwrap().to_rgba8();
    let pixel = exported.get_pixel(0, 0);
    assert!(
        pixel[0] > pixel[2],
        "selected red artifact should flow through color_adjust into exported image, got {pixel:?}"
    );
}
