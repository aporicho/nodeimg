use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "crop".into(),
        title: "Crop".into(),
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
                name: "x".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 0.0,
                    max: 8192.0,
                },
                default: Value::Int(0),
                widget_override: None,
            },
            ParamDef {
                name: "y".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 0.0,
                    max: 8192.0,
                },
                default: Value::Int(0),
                widget_override: None,
            },
            ParamDef {
                name: "width".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 1.0,
                    max: 8192.0,
                },
                default: Value::Int(256),
                widget_override: None,
            },
            ParamDef {
                name: "height".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 1.0,
                    max: 8192.0,
                },
                default: Value::Int(256),
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
        let x = match params.get("x") {
            Some(Value::Int(v)) => *v as u32,
            _ => 0,
        };
        let y = match params.get("y") {
            Some(Value::Int(v)) => *v as u32,
            _ => 0,
        };
        let w = match params.get("width") {
            Some(Value::Int(v)) => *v as u32,
            _ => 256,
        };
        let h = match params.get("height") {
            Some(Value::Int(v)) => *v as u32,
            _ => 256,
        };
        let result = crate::processing::transform::crop(img, x, y, w, h);
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
