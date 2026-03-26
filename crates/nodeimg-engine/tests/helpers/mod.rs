use std::collections::HashMap;
use std::sync::Arc;

use image::DynamicImage;
use nodeimg_engine::_test_support::{register_all, Cache, Connection, EvalEngine};
use nodeimg_engine::{NodeId, NodeRegistry};
use nodeimg_types::node_instance::NodeInstance;
use nodeimg_gpu::GpuContext;
use nodeimg_types::data_type::DataTypeRegistry;
use nodeimg_types::gpu_texture::GpuTexture;
use nodeimg_types::value::Value;

/// Run a single node and return its outputs.
///
/// `inputs`: named input images, keyed by pin name (e.g. `{"image": img}` or `{"base": img, "layer": img}`).
///   Pass empty HashMap for generator nodes (no inputs).
///
/// For GPU nodes, images are auto-uploaded to GPU textures.
/// For CPU nodes, pass `gpu_ctx` as None -- images stay as Value::Image.
#[allow(dead_code)]
pub fn run_node_test(
    type_id: &str,
    params: HashMap<String, Value>,
    inputs: HashMap<String, DynamicImage>,
    gpu_ctx: Option<&Arc<GpuContext>>,
) -> HashMap<String, Value> {
    let mut reg = NodeRegistry::new();
    register_all(&mut reg);
    let type_reg = DataTypeRegistry::with_builtins();

    let _def = reg
        .get(type_id)
        .unwrap_or_else(|| panic!("node '{}' not registered", type_id));

    let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
    let mut connections: Vec<Connection> = Vec::new();
    let mut cache = Cache::new();

    if inputs.is_empty() {
        // Generator node -- no source needed
        nodes.insert(
            0,
            NodeInstance {
                type_id: type_id.into(),
                params,
            },
        );

        EvalEngine::evaluate(
            0,
            &nodes,
            &connections,
            &reg,
            &type_reg,
            &mut cache,
            None,
            gpu_ctx,
        )
        .unwrap_or_else(|e| panic!("evaluate '{}' failed: {}", type_id, e));

        cache
            .get(0)
            .unwrap_or_else(|| panic!("node '{}' produced no output", type_id))
            .clone()
    } else {
        // Create one source node per input, pre-populate cache
        // Source nodes get IDs 0..n-1, target node gets ID n
        let target_id = inputs.len();
        for (i, (pin_name, img)) in inputs.iter().enumerate() {
            let source_id = i;
            let value = if let Some(ctx) = gpu_ctx {
                let tex = GpuTexture::from_dynamic_image(&ctx.device, &ctx.queue, img);
                Value::GpuImage(Arc::new(tex))
            } else {
                Value::Image(Arc::new(img.clone()))
            };
            cache.insert(source_id, HashMap::from([("out".into(), value)]));

            // Placeholder node (won't execute, already cached)
            nodes.insert(
                source_id,
                NodeInstance {
                    type_id: "solid_color".into(),
                    params: HashMap::new(),
                },
            );

            connections.push(Connection {
                from_node: source_id,
                from_pin: "out".into(),
                to_node: target_id,
                to_pin: pin_name.clone(),
            });
        }

        nodes.insert(
            target_id,
            NodeInstance {
                type_id: type_id.into(),
                params,
            },
        );

        EvalEngine::evaluate(
            target_id,
            &nodes,
            &connections,
            &reg,
            &type_reg,
            &mut cache,
            None,
            gpu_ctx,
        )
        .unwrap_or_else(|e| panic!("evaluate '{}' failed: {}", type_id, e));

        cache
            .get(target_id)
            .unwrap_or_else(|| panic!("node '{}' produced no output", type_id))
            .clone()
    }
}

/// Run a multi-node pipeline and return the target node's outputs.
#[allow(dead_code)]
pub fn run_pipeline_test(
    node_specs: Vec<(NodeId, &str, HashMap<String, Value>)>,
    connections: Vec<Connection>,
    target: NodeId,
    pre_cache: Option<HashMap<NodeId, HashMap<String, Value>>>,
    gpu_ctx: &Arc<GpuContext>,
) -> HashMap<String, Value> {
    let mut reg = NodeRegistry::new();
    register_all(&mut reg);
    let type_reg = DataTypeRegistry::with_builtins();

    let mut nodes: HashMap<NodeId, NodeInstance> = HashMap::new();
    for (id, type_id, params) in node_specs {
        nodes.insert(
            id,
            NodeInstance {
                type_id: type_id.into(),
                params,
            },
        );
    }

    let mut cache = Cache::new();
    if let Some(entries) = pre_cache {
        for (id, outputs) in entries {
            cache.insert(id, outputs);
        }
    }

    EvalEngine::evaluate(
        target,
        &nodes,
        &connections,
        &reg,
        &type_reg,
        &mut cache,
        None,
        Some(gpu_ctx),
    )
    .unwrap_or_else(|e| panic!("pipeline evaluate failed: {}", e));

    cache
        .get(target)
        .unwrap_or_else(|| panic!("target node {} produced no output", target))
        .clone()
}
