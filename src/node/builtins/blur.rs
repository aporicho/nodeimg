use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "blur".into(),
        title: "Blur".into(),
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
                name: "radius".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: 0.0,
                    max: 100.0,
                },
                default: Value::Float(5.0),
                widget_override: None,
            },
            ParamDef {
                name: "method".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::Enum {
                    options: vec![
                        ("gaussian".into(), "gaussian".into()),
                        ("box".into(), "box".into()),
                    ],
                },
                default: Value::String("gaussian".into()),
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
        let radius = match params.get("radius") {
            Some(Value::Float(v)) => *v,
            _ => 5.0,
        };
        let method = match params.get("method") {
            Some(Value::String(s)) => s.as_str(),
            _ => "gaussian",
        };
        let result = if method == "box" {
            crate::processing::filter::box_blur(img, radius)
        } else {
            crate::processing::filter::gaussian_blur(img, radius)
        };
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
