use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "edge_detect".into(),
        title: "Edge Detect".into(),
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
        params: vec![ParamDef {
            name: "method".into(),
            data_type: DataTypeId::new("string"),
            constraint: Constraint::Enum {
                options: vec![
                    ("sobel".into(), "sobel".into()),
                    ("canny".into(), "canny".into()),
                    ("laplacian".into(), "laplacian".into()),
                ],
            },
            default: Value::String("sobel".into()),
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
        let method = match params.get("method") {
            Some(Value::String(s)) => s.as_str(),
            _ => "sobel",
        };
        let result = match method {
            "laplacian" => crate::processing::filter::edge_detect_laplacian(img),
            // TODO: implement full Canny edge detection; falling back to Sobel for now
            _ => crate::processing::filter::edge_detect_sobel(img),
        };
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
