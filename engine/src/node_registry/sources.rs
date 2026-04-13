use std::sync::Arc;

use crate::executors::raster::ColorAdjustExecutor;
use crate::executors::remote_image::{
    providers, ImageGenerationMode, ImageGenerationSchemaProvider, ProviderRegistry,
    RemoteImageGenerationExecutor,
};
use crate::executors::remote_video::{
    providers as video_providers, ProviderRegistry as VideoProviderRegistry,
    RemoteVideoGenerationExecutor, VideoGenerationSchemaProvider,
};
use crate::executors::video::SaveVideoExecutor;
use crate::node_manager::{
    ExecutionPolicy, ExecutorType, NodeDef, NodeSourceKind, ParamDef, ParamExpose, PinDef, Purity,
    TriggerPolicy,
};
use types::Value;

use super::{NodeRegistration, NodeSource};

pub struct InventoryNodeSource;

impl NodeSource for InventoryNodeSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        crate::node_manager::collect::collect_inventory_defs::collect_inventory_defs()
            .into_iter()
            .map(|static_def| NodeRegistration {
                static_def,
                schema_provider: None,
                presentation_provider: None,
                capability_bindings: Vec::new(),
            })
            .collect()
    }
}

pub struct GenericImageGenerationSource;

impl NodeSource for GenericImageGenerationSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        let mut providers_registry = ProviderRegistry::new();
        for provider in providers::builtin_providers() {
            providers_registry.register(provider);
        }
        let providers_registry = Arc::new(providers_registry);
        let schema_provider = Arc::new(ImageGenerationSchemaProvider::new(Arc::clone(
            &providers_registry,
        )));
        let executor = Arc::new(RemoteImageGenerationExecutor::new(Arc::clone(
            &providers_registry,
        )));

        vec![NodeRegistration {
            static_def: NodeDef {
                type_id: "image_gen".into(),
                version: 1,
                source: NodeSourceKind::Api,
                name: "Image Generation".into(),
                category: "image/generation".into(),
                executor_type: ExecutorType::Api,
                requires: vec!["image.generate".into()],
                purity: Purity::Impure,
                cooking_sensitivity: Vec::new(),
                realtime_capable: false,
                execution: ExecutionPolicy {
                    timeout_ms: Some(180_000),
                    trigger_policy: TriggerPolicy::ManualOnly,
                    ..ExecutionPolicy::default()
                },
                api: None,
                inputs: vec![
                    PinDef {
                        name: "image".into(),
                        data_type: types::DataType::image(),
                        optional: true,
                    },
                    PinDef {
                        name: "mask".into(),
                        data_type: types::DataType::image(),
                        optional: true,
                    },
                ],
                outputs: vec![PinDef {
                    name: "image".into(),
                    data_type: types::DataType::image(),
                    optional: false,
                }],
                params: vec![
                    ParamDef {
                        name: "provider".into(),
                        data_type: types::DataType::string(),
                        constraint: None,
                        default_value: Value::String("libtv".into()),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "mode".into(),
                        data_type: types::DataType::string(),
                        constraint: Some(types::Constraint::enum_options(vec![
                            ImageGenerationMode::TextToImage.as_param_value().into(),
                        ])),
                        default_value: Value::String(
                            ImageGenerationMode::TextToImage.as_param_value().into(),
                        ),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "prompt".into(),
                        data_type: types::DataType::string(),
                        constraint: None,
                        default_value: Value::String(String::new()),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "negative_prompt".into(),
                        data_type: types::DataType::string(),
                        constraint: None,
                        default_value: Value::String(String::new()),
                        expose: vec![ParamExpose::Control],
                    },
                ],
                execute: Box::new(|_ctx, _inputs| {
                    Box::pin(async move {
                        Err(Box::<dyn std::error::Error + Send + Sync>::from(
                            "image_gen must be executed via capability routing",
                        ))
                    })
                }),
            },
            schema_provider: Some(schema_provider),
            presentation_provider: None,
            capability_bindings: vec![("image.generate".into(), executor)],
        }]
    }
}

pub struct ApiVideoGenerationSource;

impl ApiVideoGenerationSource {
    pub fn new() -> Self {
        Self
    }

    pub fn with_provider_registry(
        providers_registry: Arc<VideoProviderRegistry>,
    ) -> ApiVideoGenerationSourceWithRegistry {
        ApiVideoGenerationSourceWithRegistry { providers_registry }
    }
}

impl Default for ApiVideoGenerationSource {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeSource for ApiVideoGenerationSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        let mut providers_registry = VideoProviderRegistry::new();
        for provider in video_providers::builtin_providers() {
            providers_registry.register(provider);
        }
        build_api_video_generation_registrations(Arc::new(providers_registry))
    }
}

pub struct ApiVideoGenerationSourceWithRegistry {
    providers_registry: Arc<VideoProviderRegistry>,
}

impl NodeSource for ApiVideoGenerationSourceWithRegistry {
    fn collect(&self) -> Vec<NodeRegistration> {
        build_api_video_generation_registrations(Arc::clone(&self.providers_registry))
    }
}

pub struct ColorAdjustSource;

impl NodeSource for ColorAdjustSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        let executor = Arc::new(ColorAdjustExecutor::new());

        vec![NodeRegistration {
            static_def: NodeDef {
                type_id: "color_adjust".into(),
                version: 1,
                source: NodeSourceKind::Builtin,
                name: "Color Adjust".into(),
                category: "image/color".into(),
                executor_type: ExecutorType::Image,
                requires: vec!["raster.color_adjust".into()],
                purity: Purity::Pure,
                cooking_sensitivity: vec![],
                realtime_capable: true,
                execution: ExecutionPolicy::default(),
                api: None,
                inputs: vec![PinDef {
                    name: "image".into(),
                    data_type: types::DataType::image(),
                    optional: false,
                },
                PinDef {
                    name: "fps".into(),
                    data_type: types::DataType::int(),
                    optional: true,
                }],
                outputs: vec![PinDef {
                    name: "image".into(),
                    data_type: types::DataType::image(),
                    optional: false,
                }],
                params: vec![
                    ParamDef {
                        name: "brightness".into(),
                        data_type: types::DataType::float(),
                        constraint: Some(types::Constraint::range(-1.0, 1.0)),
                        default_value: Value::Float(0.0),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "contrast".into(),
                        data_type: types::DataType::float(),
                        constraint: Some(types::Constraint::range(0.0, 3.0)),
                        default_value: Value::Float(1.0),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "saturation".into(),
                        data_type: types::DataType::float(),
                        constraint: Some(types::Constraint::range(0.0, 3.0)),
                        default_value: Value::Float(1.0),
                        expose: vec![ParamExpose::Control],
                    },
                ],
                execute: Box::new(|_ctx, _inputs| {
                    Box::pin(async move {
                        Err(Box::<dyn std::error::Error + Send + Sync>::from(
                            "color_adjust must be executed via capability routing",
                        ))
                    })
                }),
            },
            schema_provider: None,
            presentation_provider: None,
            capability_bindings: vec![("raster.color_adjust".into(), executor)],
        }]
    }
}

pub struct SaveVideoSource;

impl NodeSource for SaveVideoSource {
    fn collect(&self) -> Vec<NodeRegistration> {
        let executor = Arc::new(SaveVideoExecutor::new());

        vec![NodeRegistration {
            static_def: NodeDef {
                type_id: "save_video".into(),
                version: 1,
                source: NodeSourceKind::Builtin,
                name: "Save Video".into(),
                category: "io/output".into(),
                executor_type: ExecutorType::Image,
                requires: vec!["video.encode".into()],
                purity: Purity::Impure,
                cooking_sensitivity: vec!["frame".into()],
                realtime_capable: false,
                execution: ExecutionPolicy {
                    trigger_policy: TriggerPolicy::ManualOnly,
                    ..ExecutionPolicy::default()
                },
                api: None,
                inputs: vec![
                    PinDef {
                        name: "image".into(),
                        data_type: types::DataType::image(),
                        optional: false,
                    },
                    PinDef {
                        name: "fps".into(),
                        data_type: types::DataType::int(),
                        optional: true,
                    },
                ],
                outputs: vec![PinDef {
                    name: "path".into(),
                    data_type: types::DataType::string(),
                    optional: false,
                }],
                params: vec![
                    ParamDef {
                        name: "path".into(),
                        data_type: types::DataType::string(),
                        constraint: Some(types::Constraint::file_path(vec!["mp4".into()])),
                        default_value: Value::String("/tmp/nodeimg_output.mp4".into()),
                        expose: vec![ParamExpose::Control],
                    },
                    ParamDef {
                        name: "fps".into(),
                        data_type: types::DataType::int(),
                        constraint: Some(types::Constraint::range(0.0, 60.0)),
                        default_value: Value::Int(0),
                        expose: vec![ParamExpose::Control],
                    },
                ],
                execute: Box::new(|_ctx, _inputs| {
                    Box::pin(async move {
                        Err(Box::<dyn std::error::Error + Send + Sync>::from(
                            "save_video must be executed via capability routing",
                        ))
                    })
                }),
            },
            schema_provider: None,
            presentation_provider: None,
            capability_bindings: vec![("video.encode".into(), executor)],
        }]
    }
}

fn build_api_video_generation_registrations(
    providers_registry: Arc<VideoProviderRegistry>,
) -> Vec<NodeRegistration> {
    let schema_provider = Arc::new(VideoGenerationSchemaProvider::new(Arc::clone(
        &providers_registry,
    )));
    let executor = Arc::new(RemoteVideoGenerationExecutor::new(Arc::clone(
        &providers_registry,
    )));

    vec![NodeRegistration {
        static_def: NodeDef {
            type_id: "ai_video_generate_api".into(),
            version: 1,
            source: NodeSourceKind::Api,
            name: "AI Video Generate API".into(),
            category: "ai/generation".into(),
            executor_type: ExecutorType::Api,
            requires: vec!["ai.video_generate".into()],
            purity: Purity::Impure,
            cooking_sensitivity: vec!["frame".into()],
            realtime_capable: false,
            execution: ExecutionPolicy {
                timeout_ms: Some(300_000),
                trigger_policy: TriggerPolicy::ManualOnly,
                ..ExecutionPolicy::default()
            },
            api: None,
            inputs: vec![],
            outputs: vec![
                PinDef {
                    name: "image".into(),
                    data_type: types::DataType::image(),
                    optional: false,
                },
                PinDef {
                    name: "fps".into(),
                    data_type: types::DataType::int(),
                    optional: false,
                },
            ],
            params: vec![
                ParamDef {
                    name: "provider".into(),
                    data_type: types::DataType::string(),
                    constraint: None,
                    default_value: Value::String("libtv".into()),
                    expose: vec![ParamExpose::Control],
                },
                ParamDef {
                    name: "prompt".into(),
                    data_type: types::DataType::string(),
                    constraint: None,
                    default_value: Value::String(String::new()),
                    expose: vec![ParamExpose::Control],
                },
                ParamDef {
                    name: "duration_seconds".into(),
                    data_type: types::DataType::int(),
                    constraint: Some(types::Constraint::range(1.0, 30.0)),
                    default_value: Value::Int(2),
                    expose: vec![ParamExpose::Control],
                },
                ParamDef {
                    name: "fps".into(),
                    data_type: types::DataType::int(),
                    constraint: Some(types::Constraint::range(1.0, 60.0)),
                    default_value: Value::Int(12),
                    expose: vec![ParamExpose::Control],
                },
                ParamDef {
                    name: "width".into(),
                    data_type: types::DataType::int(),
                    constraint: Some(types::Constraint::range(64.0, 2048.0)),
                    default_value: Value::Int(256),
                    expose: vec![ParamExpose::Control],
                },
                ParamDef {
                    name: "height".into(),
                    data_type: types::DataType::int(),
                    constraint: Some(types::Constraint::range(64.0, 2048.0)),
                    default_value: Value::Int(256),
                    expose: vec![ParamExpose::Control],
                },
                ParamDef {
                    name: "seed".into(),
                    data_type: types::DataType::int(),
                    constraint: None,
                    default_value: Value::Int(0),
                    expose: vec![ParamExpose::Control],
                },
            ],
            execute: Box::new(|_ctx, _inputs| {
                Box::pin(async move {
                    Err(Box::<dyn std::error::Error + Send + Sync>::from(
                        "ai_video_generate_api must be executed via capability routing",
                    ))
                })
            }),
        },
        schema_provider: Some(schema_provider),
        presentation_provider: None,
        capability_bindings: vec![("ai.video_generate".into(), executor)],
    }]
}
