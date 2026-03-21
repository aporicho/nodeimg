use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "flip".into(),
        title: "Flip".into(),
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
        params: vec![ParamDef {
            name: "direction".into(),
            data_type: DataTypeId::new("string"),
            constraint: Constraint::Enum {
                options: vec![
                    ("horizontal".into(), "horizontal".into()),
                    ("vertical".into(), "vertical".into()),
                    ("both".into(), "both".into()),
                ],
            },
            default: Value::String("horizontal".into()),
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
        let direction = match params.get("direction") {
            Some(Value::String(s)) => s.as_str(),
            _ => "horizontal",
        };
        let result = crate::processing::transform::flip(img, direction);
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
