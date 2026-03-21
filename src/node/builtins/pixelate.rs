use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "pixelate".into(),
        title: "Pixelate".into(),
        category: CategoryId::new("filter"),
        inputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: true,
        }],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![ParamDef {
            name: "block_size".into(),
            data_type: DataTypeId::new("int"),
            constraint: Constraint::Range {
                min: 1.0,
                max: 256.0,
            },
            default: Value::Int(8),
            widget_override: None,
        }],
        has_preview: false,
        process: Some(Box::new(process)),
        gpu_process: None,
    });
}

fn process(
    inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();
    if let Some(Value::Image(img)) = inputs.get("image") {
        let block_size = match params.get("block_size") {
            Some(Value::Int(v)) => *v as u32,
            _ => 8,
        };
        let result = crate::processing::filter::pixelate(img, block_size);
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
