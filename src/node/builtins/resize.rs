use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "resize".into(),
        title: "Resize".into(),
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
                name: "width".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 1.0,
                    max: 8192.0,
                },
                default: Value::Int(512),
                widget_override: None,
            },
            ParamDef {
                name: "height".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 1.0,
                    max: 8192.0,
                },
                default: Value::Int(512),
                widget_override: None,
            },
            ParamDef {
                name: "method".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::Enum {
                    options: vec![
                        ("nearest".into(), "nearest".into()),
                        ("bilinear".into(), "bilinear".into()),
                        ("bicubic".into(), "bicubic".into()),
                        ("lanczos3".into(), "lanczos3".into()),
                    ],
                },
                default: Value::String("bilinear".into()),
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
        let w = match params.get("width") {
            Some(Value::Int(v)) => *v as u32,
            _ => 512,
        };
        let h = match params.get("height") {
            Some(Value::Int(v)) => *v as u32,
            _ => 512,
        };
        let method = match params.get("method") {
            Some(Value::String(s)) => s.as_str(),
            _ => "bilinear",
        };
        let result = crate::processing::transform::resize(img, w, h, method);
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
