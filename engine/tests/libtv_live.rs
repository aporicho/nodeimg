use std::path::Path;

use engine::facade::{EngineFacade, ExecuteRequest};
use engine::graph::model::subgraph::ExecuteTarget;
use engine::{
    execution::{EvaluationFidelity, ExecutionMode},
    Engine,
};
use types::Value;

#[tokio::test]
#[ignore = "requires LibTV credentials and consumes remote credits"]
async fn test_libtv_image_gen_live() {
    let output_path = "/tmp/nodeimg_libtv_generated.png";
    let mut engine = Engine::new(None);
    let node_id = engine.add_node("image_gen").unwrap();
    engine.set_param(node_id, "provider", Value::String("libtv".into()), false);
    engine.set_param(node_id, "model", Value::String("z-image".into()), false);
    engine.set_param(node_id, "quality", Value::String("1K".into()), false);
    engine.set_param(node_id, "ratio", Value::String("1:1".into()), false);
    engine.set_param(
        node_id,
        "prompt",
        Value::String("A cinematic corgi astronaut portrait, highly detailed".into()),
        false,
    );

    let ticket = engine
        .execute_request(ExecuteRequest {
            target: ExecuteTarget::Node(node_id),
            mode: Some(ExecutionMode::OneShot {
                fidelity: EvaluationFidelity::Full,
            }),
        })
        .await
        .unwrap();
    let outputs = engine
        .get_execution_outputs(ticket.execution_id, node_id)
        .unwrap();
    let image = match outputs.get("image") {
        Some(Value::Image(image)) => image,
        other => panic!("expected image output, got {other:?}"),
    };

    let cpu_image = image
        .cpu_data()
        .expect("LibTV provider should return CPU-backed images");
    cpu_image.save(output_path).unwrap();

    assert!(Path::new(output_path).exists());
}
