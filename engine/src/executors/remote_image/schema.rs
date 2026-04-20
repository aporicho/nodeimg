use std::collections::HashMap;
use std::sync::Arc;

use crate::execution::ExecutorError;
use crate::node_manager::{ParamDef, ParamExpose};
use crate::node_registry::{ResolvedSchema, SchemaError, SchemaProvider, SchemaQuery};
use types::{Constraint, DataType, Value};

use super::provider::{ImageGenerationMode, ProviderSchemaQuery};
use super::registry::ProviderRegistry;

pub struct ImageGenerationSchemaProvider {
    providers: Arc<ProviderRegistry>,
}

impl ImageGenerationSchemaProvider {
    pub fn new(providers: Arc<ProviderRegistry>) -> Self {
        Self { providers }
    }
}

impl SchemaProvider for ImageGenerationSchemaProvider {
    fn resolve_schema(&self, query: SchemaQuery<'_>) -> Result<ResolvedSchema, SchemaError> {
        let provider_ids = self.providers.provider_ids();
        if provider_ids.is_empty() {
            return Err(SchemaError {
                message: "no remote image generation providers registered".into(),
            });
        }

        let provider_id = query
            .current_params
            .get("provider")
            .and_then(as_string)
            .filter(|provider_id| {
                provider_ids
                    .iter()
                    .any(|candidate| candidate == provider_id)
            })
            .unwrap_or_else(|| provider_ids[0].clone());
        let mode = query
            .current_params
            .get("mode")
            .and_then(as_string)
            .and_then(|value| ImageGenerationMode::parse(&value))
            .unwrap_or(ImageGenerationMode::TextToImage);

        let provider = self
            .providers
            .get(&provider_id)
            .ok_or_else(|| SchemaError {
                message: format!("provider '{provider_id}' is not registered"),
            })?;
        let models = provider.models(mode).map_err(to_schema_error)?;
        if models.is_empty() {
            return Err(SchemaError {
                message: format!("provider '{provider_id}' exposes no models for {mode:?}"),
            });
        }

        let requested_model = query.current_params.get("model").and_then(as_string);
        let resolved_model = requested_model
            .filter(|model_id| models.iter().any(|model| model.id == *model_id))
            .unwrap_or_else(|| models[0].id.clone());
        let provider_schema = provider
            .param_schema(ProviderSchemaQuery {
                mode,
                requested_model: Some(resolved_model.as_str()),
                current_params: query.current_params,
            })
            .map_err(to_schema_error)?;

        let mut params = vec![
            ParamDef {
                name: "provider".into(),
                data_type: DataType::string(),
                constraint: Some(Constraint::enum_options(provider_ids.clone())),
                default_value: Value::String(provider_id.clone()),
                expose: vec![ParamExpose::Control],
            },
            ParamDef {
                name: "model".into(),
                data_type: DataType::string(),
                constraint: Some(Constraint::enum_options(
                    models.iter().map(|model| model.id.clone()).collect(),
                )),
                default_value: Value::String(resolved_model.clone()),
                expose: vec![ParamExpose::Control],
            },
        ];
        params.extend(provider_schema.params);

        let mut system_values = HashMap::new();
        system_values.insert("__resolved_provider".into(), Value::String(provider_id));
        system_values.insert("__resolved_model".into(), Value::String(resolved_model));
        system_values.insert(
            "__provider_schema_revision".into(),
            Value::Int(provider_schema.schema_revision as i64),
        );

        Ok(ResolvedSchema {
            params,
            system_values,
        })
    }
}

fn as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn to_schema_error(error: ExecutorError) -> SchemaError {
    SchemaError {
        message: error.to_string(),
    }
}
