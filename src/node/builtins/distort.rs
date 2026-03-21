use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "distort".into(),
        title: "Distort".into(),
        category: CategoryId::new("transform"),
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
                name: "type".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::Enum {
                    options: vec![
                        ("barrel".into(), "barrel".into()),
                        ("pincushion".into(), "pincushion".into()),
                        ("swirl".into(), "swirl".into()),
                    ],
                },
                default: Value::String("barrel".into()),
                widget_override: None,
            },
            ParamDef {
                name: "strength".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: -2.0,
                    max: 2.0,
                },
                default: Value::Float(0.5),
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
        let distort_type = match params.get("type") {
            Some(Value::String(s)) => s.as_str(),
            _ => "barrel",
        };
        let strength = match params.get("strength") {
            Some(Value::Float(v)) => *v,
            _ => 0.5,
        };
        let result = crate::processing::transform::distort(img, distort_type, strength);
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
