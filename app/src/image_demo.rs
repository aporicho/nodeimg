use engine::execution::{EvaluationFidelity, ExecutionMode, ExecutionTerminalStatus};
use engine::facade::{EngineFacade, ExecutionRequest, ExecutionRequestResult};
use engine::graph::model::subgraph::ExecuteTarget;
use engine::graph::{Connection, PinRef};
use engine::Engine;
use std::fmt;
use std::path::PathBuf;
use types::{NodeId, Value};

pub(crate) const ADD_IMAGE_DEMO_GRAPH_ID: &str = "image_demo_add_graph";
pub(crate) const RUN_IMAGE_DEMO_ID: &str = "image_demo_run";

#[derive(Debug, Clone)]
pub(crate) struct ImageDemoController {
    graph: Option<ImageDemoGraph>,
    output_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ImageDemoGraph {
    pub(crate) generator: NodeId,
    pub(crate) color_adjust: NodeId,
    pub(crate) save: NodeId,
}

#[derive(Debug, Clone)]
pub(crate) enum ImageDemoOutcome {
    GraphReady(ImageDemoGraph),
    RunFinished(PathBuf),
    RunQueued(String),
    RunCancelled,
    RunFailed(String),
}

impl fmt::Display for ImageDemoOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GraphReady(graph) => write!(
                f,
                "Image demo graph ready: {:?} -> {:?} -> {:?}",
                graph.generator, graph.color_adjust, graph.save
            ),
            Self::RunFinished(path) => write!(f, "Image demo exported {}", path.display()),
            Self::RunQueued(message) => write!(f, "Image demo queued: {message}"),
            Self::RunCancelled => write!(f, "Image demo cancelled"),
            Self::RunFailed(message) => write!(f, "Image demo run failed: {message}"),
        }
    }
}

impl Default for ImageDemoController {
    fn default() -> Self {
        Self {
            graph: None,
            output_path: std::env::temp_dir().join("nodeimg-image-demo-output.png"),
        }
    }
}

impl ImageDemoController {
    pub(crate) fn handle_button(
        &mut self,
        id: &str,
        engine: &mut Engine,
    ) -> Option<ImageDemoOutcome> {
        match id {
            ADD_IMAGE_DEMO_GRAPH_ID => Some(self.add_graph(engine)),
            RUN_IMAGE_DEMO_ID => Some(self.run_graph(engine)),
            _ => None,
        }
    }

    pub(crate) fn add_graph(&mut self, engine: &mut Engine) -> ImageDemoOutcome {
        match self.ensure_graph(engine) {
            Ok(graph) => ImageDemoOutcome::GraphReady(graph),
            Err(error) => ImageDemoOutcome::RunFailed(format!("graph setup failed: {error}")),
        }
    }

    pub(crate) fn run_graph(&mut self, engine: &mut Engine) -> ImageDemoOutcome {
        let Ok(_) = self.ensure_graph(engine) else {
            return ImageDemoOutcome::RunFailed("graph setup failed".to_string());
        };

        let request = ExecutionRequest {
            target: ExecuteTarget::Graph,
            mode: Some(ExecutionMode::OneShot {
                fidelity: EvaluationFidelity::Full,
            }),
        };

        let result = match pollster::block_on(engine.execute_request(request)) {
            Ok(result) => result,
            Err(error) => return ImageDemoOutcome::RunFailed(error.to_string()),
        };

        let execution_id = match result {
            ExecutionRequestResult::Started(ticket) => ticket.execution_id,
            ExecutionRequestResult::Queued { pending_id, reason } => {
                return ImageDemoOutcome::RunQueued(format!("{pending_id:?} ({reason:?})"));
            }
        };

        match engine.await_execution(execution_id) {
            Ok(ExecutionTerminalStatus::Finished) => {
                ImageDemoOutcome::RunFinished(self.output_path.clone())
            }
            Ok(ExecutionTerminalStatus::Cancelled) => ImageDemoOutcome::RunCancelled,
            Ok(ExecutionTerminalStatus::Error) => {
                ImageDemoOutcome::RunFailed("check image provider/API configuration".to_string())
            }
            Err(error) => ImageDemoOutcome::RunFailed(error.to_string()),
        }
    }

    fn ensure_graph(&mut self, engine: &mut Engine) -> Result<ImageDemoGraph, String> {
        if let Some(graph) = self.graph {
            return Ok(graph);
        }

        let generator = engine
            .add_node("image_gen")
            .map_err(|error| error.to_string())?;
        let color_adjust = engine
            .add_node("color_adjust")
            .map_err(|error| error.to_string())?;
        let save = engine
            .add_node("save_image")
            .map_err(|error| error.to_string())?;

        connect(engine, generator, "image", color_adjust, "image")?;
        connect(engine, color_adjust, "image", save, "image")?;

        engine.set_param(
            generator,
            "prompt",
            Value::String("red product mockup on a clean studio background".into()),
            false,
        );
        engine.set_param(color_adjust, "brightness", Value::Float(0.04), false);
        engine.set_param(color_adjust, "contrast", Value::Float(1.08), false);
        engine.set_param(color_adjust, "saturation", Value::Float(1.1), false);
        engine.set_param(
            save,
            "path",
            Value::String(self.output_path.to_string_lossy().into_owned()),
            false,
        );

        let graph = ImageDemoGraph {
            generator,
            color_adjust,
            save,
        };
        self.graph = Some(graph);
        Ok(graph)
    }
}

fn connect(
    engine: &mut Engine,
    from_node: NodeId,
    from_interface: &str,
    to_node: NodeId,
    to_interface: &str,
) -> Result<(), String> {
    engine
        .connect(Connection {
            from: PinRef {
                node: from_node,
                interface: from_interface.into(),
            },
            to: PinRef {
                node: to_node,
                interface: to_interface.into(),
            },
        })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_demo_graph_is_built_once_with_expected_chain() {
        let mut engine = Engine::new(None);
        let mut controller = ImageDemoController::default();

        let graph = controller.ensure_graph(&mut engine).unwrap();
        let same_graph = controller.ensure_graph(&mut engine).unwrap();
        assert_eq!(graph.generator, same_graph.generator);
        assert_eq!(graph.color_adjust, same_graph.color_adjust);
        assert_eq!(graph.save, same_graph.save);

        let snapshot = engine.query_graph_snapshot();
        assert_eq!(snapshot.nodes.len(), 3);
        assert_eq!(snapshot.connections.len(), 2);
        assert!(snapshot.connections.iter().any(|connection| {
            connection.from.node == graph.generator && connection.to.node == graph.color_adjust
        }));
        assert!(snapshot
            .connections
            .iter()
            .any(|connection| connection.from.node == graph.color_adjust
                && connection.to.node == graph.save));
    }
}
