use std::path::Path;

use engine::execution::{EvaluationFidelity, ExecutionMode};
use engine::executors::video::decode_video_frames;
use engine::facade::{EngineFacade, ExecutionRequest, ExecutionRequestResult};
use engine::graph::model::subgraph::ExecuteTarget;
use engine::graph::{Connection, PinRef};
use engine::Engine;
use types::Value;

#[tokio::test]
#[ignore = "requires LibTV credentials and may consume remote credits"]
async fn test_headless_libtv_video_pipeline_writes_mp4() {
    let output_path = "/tmp/nodeimg_libtv_video.mp4";
    let _ = std::fs::remove_file(output_path);

    let mut engine = Engine::new(None);

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

    engine.set_param(gen, "provider", Value::String("libtv".into()), false);
    engine.set_param(
        gen,
        "model",
        Value::String("star-video2-fast".into()),
        false,
    );
    engine.set_param(
        gen,
        "prompt",
        Value::String("Ocean waves at sunset, cinematic, gentle camera movement".into()),
        false,
    );
    engine.set_param(gen, "duration_seconds", Value::Int(5), false);
    engine.set_param(gen, "fps", Value::Int(8), false);
    engine.set_param(gen, "resolution", Value::String("720p".into()), false);
    engine.set_param(gen, "ratio", Value::String("16:9".into()), false);

    engine.set_param(grade, "brightness", Value::Float(0.05), false);
    engine.set_param(grade, "contrast", Value::Float(1.1), false);
    engine.set_param(grade, "saturation", Value::Float(1.05), false);

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
    let ticket = match ticket {
        ExecutionRequestResult::Started(ticket) => ticket,
        ExecutionRequestResult::Queued { .. } => panic!("expected live video execution to start"),
    };
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
    assert_eq!(frames.len(), 40);
}
