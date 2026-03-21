use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "emboss".into(),
        title: "Emboss".into(),
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
        params: vec![
            ParamDef {
                name: "strength".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.0, max: 5.0 },
                default: Value::Float(1.0),
                widget_override: None,
            },
            ParamDef {
                name: "angle".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: 0.0,
                    max: 360.0,
                },
                default: Value::Float(135.0),
                widget_override: None,
            },
        ],
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
        let strength = match params.get("strength") {
            Some(Value::Float(v)) => *v,
            _ => 1.0,
        };
        let angle = match params.get("angle") {
            Some(Value::Float(v)) => *v,
            _ => 135.0,
        };
        let result = crate::processing::filter::emboss(img, strength, angle);
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
