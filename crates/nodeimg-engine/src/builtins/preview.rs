use nodeimg_types::category::CategoryId;
use crate::registry::{NodeDef, NodeRegistry, PinDef};
use nodeimg_types::data_type::DataTypeId;
use std::collections::HashMap;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "preview".into(),
        title: "Preview".into(),
        category: CategoryId::new("tool"),
        inputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: true,
        }],
        outputs: vec![],
        params: vec![],
        has_preview: true,
        process: Some(Box::new(|inputs, _params| {
            let mut outputs = HashMap::new();
            if let Some(img) = inputs.get("image") {
                outputs.insert("image".into(), img.clone());
            }
            outputs
        })),
        gpu_process: None,
    });
}
