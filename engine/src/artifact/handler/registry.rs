use std::collections::HashMap;
use std::sync::Arc;

use types::DataType;

use crate::artifact::handler::handler::ArtifactHandler;
use crate::artifact::handler::image::ImageArtifactHandler;
use crate::artifact::model::ArtifactError;

#[derive(Default)]
pub struct ArtifactRegistry {
    handlers: HashMap<DataType, Arc<dyn ArtifactHandler>>,
}

impl ArtifactRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(ImageArtifactHandler::new());
        registry
    }

    pub fn register<H>(&mut self, handler: H)
    where
        H: ArtifactHandler + 'static,
    {
        let data_type = handler.data_type();
        self.handlers.insert(data_type, Arc::new(handler));
    }

    pub fn get(&self, data_type: &DataType) -> Result<&dyn ArtifactHandler, ArtifactError> {
        self.handlers
            .get(data_type)
            .map(|handler| handler.as_ref())
            .ok_or_else(|| ArtifactError::HandlerNotRegistered {
                data_type: data_type.to_string(),
            })
    }

    pub fn contains(&self, data_type: &DataType) -> bool {
        self.handlers.contains_key(data_type)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use types::{DataType, Value};

    use crate::artifact::handler::handler::ArtifactHandler;
    use crate::artifact::model::ArtifactError;

    use super::ArtifactRegistry;

    struct DummyStringHandler;

    impl ArtifactHandler for DummyStringHandler {
        fn data_type(&self) -> DataType {
            DataType::string()
        }

        fn extension(&self) -> &'static str {
            "txt"
        }

        fn serialize(&self, _value: &Value, _path: &Path) -> Result<(), ArtifactError> {
            Ok(())
        }

        fn deserialize(&self, _path: &Path) -> Result<Value, ArtifactError> {
            Ok(Value::String("dummy".into()))
        }
    }

    #[test]
    fn test_registry_defaults_include_image_handler() {
        let registry = ArtifactRegistry::with_defaults();
        assert!(registry.contains(&DataType::image()));
    }

    #[test]
    fn test_registry_can_register_custom_handler() {
        let mut registry = ArtifactRegistry::new();
        registry.register(DummyStringHandler);

        assert!(registry.contains(&DataType::string()));
        assert_eq!(
            registry
                .get(&DataType::string())
                .expect("string handler")
                .extension(),
            "txt"
        );
    }

    #[test]
    fn test_registry_returns_error_for_missing_handler() {
        let registry = ArtifactRegistry::new();
        match registry.get(&DataType::float()) {
            Ok(_) => panic!("missing handler should error"),
            Err(err) => assert!(matches!(err, ArtifactError::HandlerNotRegistered { .. })),
        }
    }
}
