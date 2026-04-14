use std::path::Path;

use engine::execution::{EvaluationFidelity, ExecutionMode};
use engine::executors::remote_video::{providers, ProviderRegistry as VideoProviderRegistry};
use engine::executors::video::decode_video_frames;
use engine::facade::{EngineFacade, ExecutionRequest};
use engine::graph::model::subgraph::ExecuteTarget;
use engine::graph::{Connection, PinRef};
use engine::node_registry::{sources, NodeRegistry};
use engine::Engine;
use types::Value;

#[tokio::test]
async fn test_headless_video_pipeline_writes_animated_output() {
    let output_path = "/tmp/nodeimg_mock_video.mp4";
    let _ = std::fs::remove_file(output_path);

    let mut video_providers = VideoProviderRegistry::new();
    video_providers.register(providers::mock_provider());
    let mut registry = NodeRegistry::new();
    registry.register(sources::InventoryNodeSource);
    registry.register(sources::ApiVideoGenerationSource::with_provider_registry(std::sync::Arc::new(video_providers)));
    registry.register(sources::ColorAdjustSource);
    registry.register(sources::SaveVideoSource);
    let mut engine = Engine::from_registry_bundle(registry.build(), None);

    let gen = engine.add_node("ai_video_generate_api").unwrap();
    let grade = engine.add_node("color_adjust").unwrap();
    let save = engine.add_node("save_video").unwrap();

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
    engine
        .connect(Connection {
            from: PinRef {
                node: gen,
                interface: "fps".into(),
            },
            to: PinRef {
                node: save,
                interface: "fps".into(),
            },
        })
        .unwrap();

    engine.set_param(gen, "provider", Value::String("mock".into()), false);
    engine.set_param(gen, "model", Value::String("mock-wave".into()), false);
    engine.set_param(
        gen,
        "prompt",
        Value::String("ocean waves at sunset".into()),
        false,
    );
    engine.set_param(gen, "duration_seconds", Value::Int(1), false);
    engine.set_param(gen, "fps", Value::Int(8), false);
    engine.set_param(gen, "width", Value::Int(64), false);
    engine.set_param(gen, "height", Value::Int(64), false);
    engine.set_param(gen, "seed", Value::Int(7), false);

    engine.set_param(grade, "brightness", Value::Float(0.1), false);
    engine.set_param(grade, "contrast", Value::Float(1.2), false);
    engine.set_param(grade, "saturation", Value::Float(1.1), false);

    engine.set_param(save, "path", Value::String(output_path.into()), false);

    let ticket = engine
        .execute_request(ExecutionRequest {
            target: ExecuteTarget::Graph,
            mode: Some(ExecutionMode::OneShot {
                fidelity: EvaluationFidelity::Full,
            }),
        })
        .await
        .unwrap();
    engine.await_execution(ticket.execution_id).unwrap();

    let outputs = engine
        .query_execution_outputs(ticket.execution_id, save)
        .unwrap();
    assert!(matches!(
        outputs.get("path"),
        Some(Value::String(path)) if path == output_path
    ));
    assert!(Path::new(output_path).exists());

    let frames = decode_video_frames(Path::new(output_path)).unwrap();
    assert_eq!(frames.len(), 8);
    assert_ne!(
        frames[0].to_rgba8().get_pixel(0, 0),
        frames[7].to_rgba8().get_pixel(0, 0)
    );
}
