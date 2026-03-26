mod helpers;

use std::collections::HashMap;
use std::sync::Arc;

use nodeimg_engine::_test_support::{register_all, Cache, Connection, EvalEngine};
use nodeimg_engine::{NodeId, NodeRegistry};
use nodeimg_types::node_instance::NodeInstance;
use nodeimg_gpu::test_utils::try_create_headless_context;
use nodeimg_gpu::GpuContext;
use nodeimg_types::data_type::DataTypeRegistry;
use nodeimg_types::value::Value;

fn gpu() -> Option<Arc<GpuContext>> {
    let ctx = try_create_headless_context();
    if ctx.is_none() {
        eprintln!("SKIPPED: no GPU adapter available");
    }
    ctx
}

/// solid_color(red, 16x16) -> invert => cyan
#[test]
fn test_two_node_pipeline() {
    let Some(ctx) = gpu() else { return };

    let node_specs = vec![
        (
            0,
            "solid_color",
            HashMap::from([
                ("color".into(), Value::Color([1.0, 0.0, 0.0, 1.0])),
                ("width".into(), Value::Int(16)),
                ("height".into(), Value::Int(16)),
            ]),
        ),
        (1, "invert", HashMap::new()),
    ];

    let connections = vec![Connection {
        from_node: 0,
        from_pin: "image".into(),
        to_node: 1,
        to_pin: "image".into(),
    }];

    let result = helpers::run_pipeline_test(node_specs, connections, 1, None, &ctx);

    // Verify output is a 16x16 GpuImage
    match result.get("image") {
        Some(Value::GpuImage(tex)) => {
            assert_eq!(tex.width, 16);
            assert_eq!(tex.height, 16);

            // Readback pixel check: inverted red = cyan (0, 255, 255)
            let pixels = tex.download_rgba(&ctx.device, &ctx.queue);
            let (r, g, b) = (pixels[0], pixels[1], pixels[2]);
            assert!(r <= 1, "expected R ~0 (cyan), got {}", r);
            assert!(g >= 254, "expected G ~255 (cyan), got {}", g);
            assert!(b >= 254, "expected B ~255 (cyan), got {}", b);
        }
        other => panic!("expected GpuImage for 'image', got {:?}", other),
    }
}

/// solid_color(gray, 16x16) -> blur(radius 2, gaussian) -> sharpen(amount 1.0) => 16x16 GpuImage
#[test]
fn test_three_node_chain() {
    let Some(ctx) = gpu() else { return };

    let node_specs = vec![
        (
            0,
            "solid_color",
            HashMap::from([
                ("color".into(), Value::Color([0.5, 0.5, 0.5, 1.0])),
                ("width".into(), Value::Int(16)),
                ("height".into(), Value::Int(16)),
            ]),
        ),
        (
            1,
            "blur",
            HashMap::from([
                ("radius".into(), Value::Float(2.0)),
                ("method".into(), Value::String("gaussian".into())),
            ]),
        ),
        (
            2,
            "sharpen",
            HashMap::from([
                ("amount".into(), Value::Float(1.0)),
                ("radius".into(), Value::Float(1.0)),
            ]),
        ),
    ];

    let connections = vec![
        Connection {
            from_node: 0,
            from_pin: "image".into(),
            to_node: 1,
            to_pin: "image".into(),
        },
        Connection {
            from_node: 1,
            from_pin: "image".into(),
            to_node: 2,
            to_pin: "image".into(),
        },
    ];

    let result = helpers::run_pipeline_test(node_specs, connections, 2, None, &ctx);

    match result.get("image") {
        Some(Value::GpuImage(tex)) => {
            assert_eq!(tex.width, 16, "expected width 16, got {}", tex.width);
            assert_eq!(tex.height, 16, "expected height 16, got {}", tex.height);
        }
        other => panic!("expected GpuImage for 'image', got {:?}", other),
    }
}

/// Create solid_color(red), evaluate, invalidate_all, change to green, re-evaluate,
/// verify pixel is green not red.
#[test]
fn test_cache_invalidation() {
    let Some(ctx) = gpu() else { return };

    let mut reg = NodeRegistry::new();
    register_all(&mut reg);
    let type_reg = DataTypeRegistry::with_builtins();

    // Build a single solid_color node with red
    let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
    nodes.insert(
        0,
        NodeInstance {
            type_id: "solid_color".into(),
            params: HashMap::from([
                ("color".into(), Value::Color([1.0, 0.0, 0.0, 1.0])),
                ("width".into(), Value::Int(16)),
                ("height".into(), Value::Int(16)),
            ]),
        },
    );
    let connections: Vec<Connection> = vec![];
    let mut cache = Cache::new();

    // First evaluation: should produce red
    EvalEngine::evaluate(0, &nodes, &connections, &reg, &type_reg, &mut cache, None, Some(&ctx))
        .expect("first evaluate failed");

    let outputs = cache.get(0).expect("node 0 should have cached output");
    if let Some(Value::GpuImage(tex)) = outputs.get("image") {
        let pixels = tex.download_rgba(&ctx.device, &ctx.queue);
        assert!(pixels[0] >= 254, "first eval: expected R ~255, got {}", pixels[0]);
        assert!(pixels[1] <= 1, "first eval: expected G ~0, got {}", pixels[1]);
    } else {
        panic!("first eval: expected GpuImage output");
    }

    // Invalidate all cached results
    cache.invalidate_all();
    assert!(cache.get(0).is_none(), "cache should be empty after invalidate_all");

    // Change color to green
    nodes.get_mut(&0).unwrap().params.insert(
        "color".into(),
        Value::Color([0.0, 1.0, 0.0, 1.0]),
    );

    // Re-evaluate
    EvalEngine::evaluate(0, &nodes, &connections, &reg, &type_reg, &mut cache, None, Some(&ctx))
        .expect("second evaluate failed");

    let outputs = cache.get(0).expect("node 0 should have cached output after re-eval");
    if let Some(Value::GpuImage(tex)) = outputs.get("image") {
        let pixels = tex.download_rgba(&ctx.device, &ctx.queue);
        assert!(pixels[0] <= 1, "second eval: expected R ~0 (green), got {}", pixels[0]);
        assert!(
            pixels[1] >= 254,
            "second eval: expected G ~255 (green), got {}",
            pixels[1]
        );
        assert!(pixels[2] <= 1, "second eval: expected B ~0 (green), got {}", pixels[2]);
    } else {
        panic!("second eval: expected GpuImage output");
    }
}

/// Create invert node with no input connected, evaluate.
/// Invert's gpu_process returns empty HashMap when input is missing.
#[test]
fn test_missing_input_produces_empty_output() {
    let Some(ctx) = gpu() else { return };

    let mut reg = NodeRegistry::new();
    register_all(&mut reg);
    let type_reg = DataTypeRegistry::with_builtins();

    let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
    nodes.insert(
        0,
        NodeInstance {
            type_id: "invert".into(),
            params: HashMap::new(),
        },
    );
    let connections: Vec<Connection> = vec![];
    let mut cache = Cache::new();

    EvalEngine::evaluate(0, &nodes, &connections, &reg, &type_reg, &mut cache, None, Some(&ctx))
        .expect("evaluate should not error for missing input");

    let outputs = cache.get(0).expect("node 0 should have cached output (even if empty)");
    assert!(
        outputs.is_empty(),
        "invert with no input should produce empty outputs, got {} entries",
        outputs.len()
    );
}
