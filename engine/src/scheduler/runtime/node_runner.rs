use std::collections::HashMap;

use crate::executors::image::ImageExecutor;
use crate::node_manager::NodeManager;
use crate::scheduler::model::{ExecutorType, PlannedNode};
use crate::scheduler::runtime::executor_registry::{ExecutorDescriptor, ExecutorRegistry};
use types::Value;

#[derive(Debug)]
pub enum NodeRunError {
    ExecutorNotRegistered { executor_type: ExecutorType },
    UnsupportedExecutorType { executor_type: ExecutorType },
    NodeNotFound { type_id: String },
    ExecutionFailed(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for NodeRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutorNotRegistered { executor_type } => {
                write!(f, "executor not registered for type: {executor_type:?}")
            }
            Self::UnsupportedExecutorType { executor_type } => {
                write!(f, "unsupported executor type: {executor_type:?}")
            }
            Self::NodeNotFound { type_id } => write!(f, "node type '{}' not registered", type_id),
            Self::ExecutionFailed(error) => write!(f, "node execution failed: {error}"),
        }
    }
}

impl std::error::Error for NodeRunError {}

pub async fn run_planned_node(
    executor_registry: &ExecutorRegistry,
    node_manager: &NodeManager,
    image_executor: &ImageExecutor,
    node_type_id: &str,
    planned_node: &PlannedNode,
    inputs: HashMap<String, Value>,
) -> Result<HashMap<String, Value>, NodeRunError> {
    let descriptor = ExecutorDescriptor {
        executor_type: planned_node.executor_type,
        has_gpu: image_executor.context().has_gpu(),
    };

    let entry = executor_registry
        .resolve(descriptor)
        .ok_or(NodeRunError::ExecutorNotRegistered {
            executor_type: planned_node.executor_type,
        })?;

    match entry.executor_type {
        ExecutorType::Image => run_image_node(node_manager, image_executor, node_type_id, inputs).await,
        executor_type => Err(NodeRunError::UnsupportedExecutorType { executor_type }),
    }
}

async fn run_image_node(
    node_manager: &NodeManager,
    image_executor: &ImageExecutor,
    node_type_id: &str,
    inputs: HashMap<String, Value>,
) -> Result<HashMap<String, Value>, NodeRunError> {
    let node_def = node_manager
        .get_node_def(node_type_id)
        .ok_or_else(|| NodeRunError::NodeNotFound {
            type_id: node_type_id.to_string(),
        })?;

    let ctx = image_executor.context();
    (node_def.execute)(ctx, inputs)
        .await
        .map_err(NodeRunError::ExecutionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executors::image::GpuExecutor;
    use crate::node_manager::{ExecutorType, NodeDef, ParamDef, ParamExpose, PinDef};
    use crate::scheduler::runtime::executor_registry::{ExecutorEntry, ExecutorRegistry};
    use types::{DataType, NodeId};

    fn make_test_def(type_id: &str) -> NodeDef {
        NodeDef {
            type_id: type_id.into(),
            name: type_id.into(),
            category: "test".into(),
            executor_type: ExecutorType::Image,
            inputs: vec![PinDef {
                name: "in".into(),
                data_type: DataType::float(),
                optional: false,
            }],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataType::float(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "amount".into(),
                data_type: DataType::float(),
                constraint: None,
                default_value: Value::Float(1.0),
                expose: vec![ParamExpose::Control],
            }],
            execute: Box::new(|_ctx, mut inputs| {
                Box::pin(async move {
                    let value = match inputs.remove("in") {
                        Some(Value::Float(v)) => v,
                        _ => 0.0,
                    };
                    Ok(HashMap::from([(String::from("out"), Value::Float(value + 1.0))]))
                })
            }),
        }
    }

    #[tokio::test]
    async fn runs_image_node_successfully() {
        let mut node_manager = NodeManager::new();
        node_manager.register(make_test_def("test.node"));
        let image_executor = ImageExecutor::new(None::<GpuExecutor>);
        let mut registry = ExecutorRegistry::new();
        registry.register(ExecutorEntry::new("image", ExecutorType::Image, |_| true));
        let planned_node = PlannedNode {
            node_id: NodeId(1),
            executor_type: ExecutorType::Image,
            exec_signature: crate::cache::model::ExecSignature::new(1, 1, 0, 0),
        };

        let outputs = run_planned_node(
            &registry,
            &node_manager,
            &image_executor,
            "test.node",
            &planned_node,
            HashMap::from([(String::from("in"), Value::Float(2.0))]),
        )
        .await
        .unwrap();

        assert!(matches!(outputs.get("out"), Some(Value::Float(v)) if *v == 3.0));
    }

    #[tokio::test]
    async fn rejects_when_executor_not_registered() {
        let mut node_manager = NodeManager::new();
        node_manager.register(make_test_def("test.node"));
        let image_executor = ImageExecutor::new(None::<GpuExecutor>);
        let registry = ExecutorRegistry::new();
        let planned_node = PlannedNode {
            node_id: NodeId(1),
            executor_type: ExecutorType::Image,
            exec_signature: crate::cache::model::ExecSignature::new(1, 1, 0, 0),
        };

        let error = run_planned_node(
            &registry,
            &node_manager,
            &image_executor,
            "test.node",
            &planned_node,
            HashMap::new(),
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            NodeRunError::ExecutorNotRegistered {
                executor_type: ExecutorType::Image
            }
        ));
    }

    #[tokio::test]
    async fn rejects_unmatched_variant_of_same_type() {
        let mut node_manager = NodeManager::new();
        node_manager.register(make_test_def("test.node"));
        let image_executor = ImageExecutor::new(None::<GpuExecutor>);
        let mut registry = ExecutorRegistry::new();
        registry.register(ExecutorEntry::new(
            "gpu-image",
            ExecutorType::Image,
            |descriptor| descriptor.has_gpu,
        ));
        let planned_node = PlannedNode {
            node_id: NodeId(1),
            executor_type: ExecutorType::Image,
            exec_signature: crate::cache::model::ExecSignature::new(1, 1, 0, 0),
        };

        let error = run_planned_node(
            &registry,
            &node_manager,
            &image_executor,
            "test.node",
            &planned_node,
            HashMap::new(),
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            NodeRunError::ExecutorNotRegistered {
                executor_type: ExecutorType::Image
            }
        ));
    }
}
