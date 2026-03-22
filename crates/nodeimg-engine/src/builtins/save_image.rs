use nodeimg_types::category::CategoryId;
use nodeimg_types::constraint::Constraint;
use crate::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use nodeimg_types::data_type::DataTypeId;
use nodeimg_types::value::Value;
use std::collections::HashMap;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "save_image".into(),
        title: "Save Image".into(),
        category: CategoryId::new("data"),
        inputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: true,
        }],
        outputs: vec![],
        params: vec![ParamDef {
            name: "path".into(),
            data_type: DataTypeId::new("string"),
            constraint: Constraint::FilePath {
                filters: vec!["png".into(), "jpg".into(), "bmp".into(), "webp".into()],
            },
            default: Value::String(String::new()),
            widget_override: None,
        }],
        has_preview: false,
        process: Some(Box::new(|inputs, params| {
            if let (Some(Value::Image(img)), Some(Value::String(path))) =
                (inputs.get("image"), params.get("path"))
            {
                if !path.is_empty() {
                    let _ = img.save(path);
                }
            }
            HashMap::new()
        })),
        gpu_process: None,
    });
}
