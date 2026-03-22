use nodeimg_types::category::CategoryId;
use nodeimg_types::constraint::Constraint;
use crate::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use nodeimg_types::data_type::DataTypeId;
use nodeimg_types::value::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "load_image".into(),
        title: "Load Image".into(),
        category: CategoryId::new("data"),
        inputs: vec![],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![ParamDef {
            name: "path".into(),
            data_type: DataTypeId::new("string"),
            constraint: Constraint::FilePath {
                filters: vec![
                    "png".into(),
                    "jpg".into(),
                    "jpeg".into(),
                    "bmp".into(),
                    "webp".into(),
                ],
            },
            default: Value::String(String::new()),
            widget_override: None,
        }],
        has_preview: true,
        process: Some(Box::new(process)),
        gpu_process: None,
    });
}

fn process(
    _inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();
    if let Some(Value::String(path)) = params.get("path") {
        if !path.is_empty() {
            if let Ok(img) = image::open(path) {
                outputs.insert("image".into(), Value::Image(Arc::new(img)));
            }
        }
    }
    outputs
}
