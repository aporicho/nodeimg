use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use types::Value;

use super::backend::{FfmpegVideoBackend, VideoEncodeBackend, VideoEncodeSession};
use crate::capability::{Capability, CapabilityId, SideEffect};
use crate::execution::{
    ExecutionOutputs, ExecutionTerminalStatus, ExecutorError, NodeExecutionRequest,
    PlanLifecycleContext, RunId,
};
use crate::executors::{Executor, ExecutorFuture, LocalityProfile};

pub struct SaveVideoExecutor {
    backend: Box<dyn VideoEncodeBackend>,
    sessions: Mutex<HashMap<(RunId, types::NodeId), Box<dyn VideoEncodeSession>>>,
}

impl SaveVideoExecutor {
    pub fn new() -> Self {
        Self {
            backend: Box::new(FfmpegVideoBackend),
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for SaveVideoExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor for SaveVideoExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "video.encode",
            vec![types::DataType::image()],
            vec![types::DataType::string()],
            vec![SideEffect::FileWrite("*".into())],
        )]
    }

    fn execute<'a>(
        &'a self,
        cap_id: &'a CapabilityId,
        req: NodeExecutionRequest<'a>,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            if cap_id != "video.encode" {
                return Err(ExecutorError::Unavailable {
                    message: format!("unsupported capability '{cap_id}'"),
                });
            }

            let image = match req.inputs.get("image") {
                Some(Value::Image(image)) => image,
                _ => {
                    return Err(ExecutorError::InvalidInput {
                        message: "save_video requires an 'image' input".into(),
                    });
                }
            };
            let cpu = image
                .cpu_data()
                .ok_or_else(|| ExecutorError::MaterializationFailed {
                    message: "save_video currently requires CPU-backed images".into(),
                })?;

            let mut sessions = self.sessions.lock().expect("save video sessions poisoned");
            if !sessions.contains_key(&(req.run_id, req.node_id)) {
                let path = required_string(&req.inputs, "path")?;
                let fps = required_positive_or_inherited_fps(&req.inputs)?;
                sessions.insert(
                    (req.run_id, req.node_id),
                    self.backend.new_session(PathBuf::from(path), fps)?,
                );
            }
            let session = sessions
                .get_mut(&(req.run_id, req.node_id))
                .expect("save_video session inserted or preexisting");
            let frame_index = match req.cooking_context.dimensions.get("frame") {
                Some(crate::execution::DimensionValue::Int(value)) if *value >= 0 => *value as u32,
                _ => 0,
            };
            session.push_frame(frame_index, cpu)?;
            let path = session.path().display().to_string();

            Ok(ExecutionOutputs::full(HashMap::from([(
                String::from("path"),
                Value::String(path),
            )])))
        })
    }

    fn on_plan_started(&self, ctx: &PlanLifecycleContext<'_>) -> Result<(), ExecutorError> {
        let mut sessions = self.sessions.lock().expect("save video sessions poisoned");
        for node in ctx.my_nodes {
            sessions.remove(&(ctx.run_id, node.node_id));
        }
        Ok(())
    }

    fn on_plan_finished(&self, ctx: &PlanLifecycleContext<'_>, status: ExecutionTerminalStatus) {
        let mut sessions = self.sessions.lock().expect("save video sessions poisoned");
        for node in ctx.my_nodes {
            if let Some(session) = sessions.remove(&(ctx.run_id, node.node_id)) {
                if status == ExecutionTerminalStatus::Finished {
                    let _ = session.finish();
                }
            }
        }
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

fn required_string(params: &HashMap<String, Value>, key: &str) -> Result<String, ExecutorError> {
    match params.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(ExecutorError::InvalidInput {
            message: format!("missing required string parameter '{key}'"),
        }),
    }
}

fn required_positive_or_inherited_fps(inputs: &HashMap<String, Value>) -> Result<u32, ExecutorError> {
    match inputs.get("fps") {
        Some(Value::Int(value)) if *value > 0 => Ok(*value as u32),
        Some(Value::Float(value)) if *value > 0.0 => Ok(*value as u32),
        _ => Err(ExecutorError::InvalidInput {
            message: "save_video requires a positive fps, either from upstream connection or param override".into(),
        }),
    }
}
