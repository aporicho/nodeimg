use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "histogram".into(),
        title: "Histogram".into(),
        category: CategoryId::new("tool"),
        inputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: true,
        }],
        outputs: vec![],
        params: vec![ParamDef {
            name: "channel".into(),
            data_type: DataTypeId::new("string"),
            constraint: Constraint::Enum {
                options: vec![
                    ("rgb".into(), "rgb".into()),
                    ("red".into(), "red".into()),
                    ("green".into(), "green".into()),
                    ("blue".into(), "blue".into()),
                    ("luminance".into(), "luminance".into()),
                ],
            },
            default: Value::String("rgb".into()),
            widget_override: None,
        }],
        has_preview: true,
        process: Some(Box::new(|inputs, params| {
            let mut outputs = HashMap::new();
            if let Some(Value::Image(img)) = inputs.get("image") {
                let channel = match params.get("channel") {
                    Some(Value::String(s)) => s.as_str(),
                    _ => "rgb",
                };
                let histograms = crate::processing::color::compute_histogram(img, channel);
                let hist_img =
                    crate::processing::color::render_histogram_image(&histograms, channel);
                outputs.insert("image".into(), Value::Image(Arc::new(hist_img)));
            }
            outputs
        })),
        gpu_process: None,
    });
}
