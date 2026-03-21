use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "lut_apply".into(),
        title: "LUT Apply".into(),
        category: CategoryId::new("color"),
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
                name: "path".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::FilePath {
                    filters: vec!["cube".into()],
                },
                default: Value::String(String::new()),
                widget_override: None,
            },
            ParamDef {
                name: "intensity".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.0, max: 1.0 },
                default: Value::Float(1.0),
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
        let path = match params.get("path") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        let intensity = match params.get("intensity") {
            Some(Value::Float(v)) => *v,
            _ => 1.0,
        };

        if !path.is_empty() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                match crate::processing::color::parse_cube_lut(&content) {
                    Ok(lut) => {
                        let result = crate::processing::color::lut_apply(img, &lut, intensity);
                        outputs.insert("image".into(), Value::Image(Arc::new(result)));
                    }
                    Err(e) => {
                        eprintln!("LUT parse error: {e}");
                    }
                }
            }
        }
    }
    outputs
}
