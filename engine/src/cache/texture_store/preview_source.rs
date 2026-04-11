use types::Value;

use crate::cache::model::{CacheKey, PreviewRequest};
use crate::cache::result_store::ResultStore;

pub fn select_preview_source(store: &ResultStore, request: &PreviewRequest) -> Option<Value> {
    let key = CacheKey::new(
        request.node_id,
        request.output_pin.clone(),
        request.exec_signature,
    );
    let value = store.get(&key)?;

    if !is_previewable(value.as_ref()) {
        return None;
    }

    Some(value.as_ref().clone())
}

fn is_previewable(value: &Value) -> bool {
    matches!(value, Value::Image(_))
}

#[cfg(test)]
mod tests {
    use types::{Image, NodeId, Value};

    use crate::cache::model::{ExecSignature, GenerationId, PreviewRequest};
    use crate::cache::result_store::ResultStore;

    use super::select_preview_source;

    #[test]
    fn selects_only_matching_signature() {
        let store = ResultStore::new();
        store
            .put(
                NodeId(1),
                "image",
                ExecSignature::new(1, 1, 10, 20),
                GenerationId::initial(),
                GenerationId::initial(),
                Value::Image(Image::from_cpu(image::DynamicImage::new_rgba8(1, 1))),
            )
            .expect("write should succeed");

        let request = PreviewRequest::new(
            NodeId(1),
            "image",
            ExecSignature::new(1, 1, 11, 20),
            GenerationId::initial(),
        );

        assert!(select_preview_source(&store, &request).is_none());
    }

    #[test]
    fn rejects_non_previewable_values() {
        let store = ResultStore::new();
        store
            .put(
                NodeId(1),
                "value",
                ExecSignature::new(1, 1, 10, 20),
                GenerationId::initial(),
                GenerationId::initial(),
                Value::Int(7),
            )
            .expect("write should succeed");

        let request = PreviewRequest::new(
            NodeId(1),
            "value",
            ExecSignature::new(1, 1, 10, 20),
            GenerationId::initial(),
        );

        assert!(select_preview_source(&store, &request).is_none());
    }
}
