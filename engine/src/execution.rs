use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::cache::model::{ExecSignature, GenerationId};
use crate::capability::CapabilityId;
use crate::executors::image::ExecContext;
use crate::graph::model::subgraph::ExecuteTarget;
use crate::node_manager::{ArtifactPolicy, NodeDef};
use types::{NodeId, Value};

pub type RunId = u64;
pub type ExecutionId = u64;
pub type ExecutionInputs = HashMap<String, Value>;

#[derive(Clone, Debug)]
pub struct ExecutionOutputs {
    pub values: HashMap<String, Value>,
    pub fidelity_achieved: EvaluationFidelity,
}

impl ExecutionOutputs {
    pub fn full(values: HashMap<String, Value>) -> Self {
        Self {
            values,
            fidelity_achieved: EvaluationFidelity::Full,
        }
    }

    pub fn preview(values: HashMap<String, Value>, fidelity_achieved: EvaluationFidelity) -> Self {
        Self {
            values,
            fidelity_achieved,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EvaluationFidelity {
    Preview {
        max_side: Option<u32>,
        precision: PreviewPrecision,
    },
    Full,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PreviewPrecision {
    Float32,
    Float16,
    Int8,
}

impl Default for EvaluationFidelity {
    fn default() -> Self {
        Self::Full
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DimensionValue {
    Int(i64),
    String(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CookingContext {
    pub dimensions: HashMap<String, DimensionValue>,
}

impl CookingContext {
    pub fn hash_filtered(&self, sensitivity: &[String]) -> u64 {
        if sensitivity.is_empty() {
            return 0;
        }

        let mut dims: Vec<_> = sensitivity
            .iter()
            .filter_map(|dimension| {
                self.dimensions
                    .get(dimension)
                    .map(|value| (dimension.as_str(), value))
            })
            .collect();
        dims.sort_by(|left, right| left.0.cmp(right.0));

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (dimension, value) in dims {
            dimension.hash(&mut hasher);
            value.hash(&mut hasher);
        }
        hasher.finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CookingContextRange {
    pub axes: Vec<CookingAxis>,
}

impl CookingContextRange {
    pub fn frame_range(start: u64, end_exclusive: u64) -> Self {
        Self {
            axes: vec![CookingAxis {
                dimension: "frame".into(),
                values: (start..end_exclusive)
                    .map(|frame| DimensionValue::Int(frame as i64))
                    .collect(),
            }],
        }
    }

    pub fn enumerate(&self) -> Vec<CookingContext> {
        if self.axes.is_empty() {
            return vec![CookingContext::default()];
        }

        let mut contexts = vec![HashMap::new()];
        for axis in &self.axes {
            let mut next = Vec::new();
            for context in &contexts {
                for value in &axis.values {
                    let mut dims = context.clone();
                    dims.insert(axis.dimension.clone(), value.clone());
                    next.push(dims);
                }
            }
            contexts = next;
        }

        contexts
            .into_iter()
            .map(|dimensions| CookingContext { dimensions })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CookingAxis {
    pub dimension: String,
    pub values: Vec<DimensionValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionMode {
    OneShot {
        fidelity: EvaluationFidelity,
    },
    Continuous {
        tick_source: TickSource,
        target_fps: f32,
    },
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::OneShot {
            fidelity: EvaluationFidelity::Full,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TickSource {
    OsClock,
    External(String),
    DataPull(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionTerminalStatus {
    Finished,
    Cancelled,
    Error,
}

pub trait CancelToken: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

#[derive(Default)]
pub struct NoopCancelToken;

impl CancelToken for NoopCancelToken {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Clone, Default)]
pub struct SharedCancelToken {
    cancelled: Arc<AtomicBool>,
}

impl SharedCancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl CancelToken for SharedCancelToken {
    fn is_cancelled(&self) -> bool {
        self.is_cancelled()
    }
}

pub trait ProgressSink: Send + Sync {
    fn report(&self, event: ExecutorProgressEvent);
}

#[derive(Default)]
pub struct NoopProgressSink;

impl ProgressSink for NoopProgressSink {
    fn report(&self, _event: ExecutorProgressEvent) {}
}

#[derive(Clone, Debug)]
pub enum ExecutorProgressEvent {
    Message {
        text: String,
    },
    Fraction {
        current: u32,
        total: u32,
    },
    Preview {
        output: String,
        value: Value,
    },
    PreviewReady {
        node_id: NodeId,
        output: String,
        value: Value,
        exec_signature: ExecSignature,
        fidelity: EvaluationFidelity,
    },
}

#[derive(Clone, Debug)]
pub struct ExecutionPlan {
    pub target: ExecuteTarget,
    pub mode: ExecutionMode,
    pub subtasks: Vec<PlanSubtask>,
}

#[derive(Clone, Debug)]
pub struct PlanSubtask {
    pub cooking_context: CookingContext,
    pub order: Vec<PlannedNode>,
}

#[derive(Clone, Debug)]
pub struct PlannedNode {
    pub node_id: NodeId,
    pub type_id: String,
    pub params: HashMap<String, Value>,
    pub inputs: HashMap<String, PinSource>,
    pub exec_signature: ExecSignature,
    pub outputs: Vec<PlannedOutput>,
    pub requires: Vec<CapabilityId>,
    pub timeout_ms: Option<u64>,
    pub realtime_capable: bool,
    pub artifact_policy: ArtifactPolicy,
}

#[derive(Clone, Debug)]
pub struct PlannedOutput {
    pub name: String,
    pub optional: bool,
}

#[derive(Clone, Debug)]
pub enum PinSource {
    UpstreamOutput { node_id: NodeId, output_pin: String },
    Value(Value),
}

#[derive(Clone, Debug)]
pub struct PlanLifecycleNode {
    pub node_id: NodeId,
    pub type_id: String,
    pub resolved_params: HashMap<String, Value>,
}

pub struct PlanLifecycleContext<'a> {
    pub run_id: RunId,
    pub my_nodes: &'a [PlanLifecycleNode],
    pub mode: &'a ExecutionMode,
}

pub struct NodeExecutionRequest<'a> {
    pub run_id: RunId,
    pub node_id: NodeId,
    pub node_def: &'a NodeDef,
    pub exec_context: ExecContext<'a>,
    pub inputs: ExecutionInputs,
    pub exec_signature: ExecSignature,
    pub generation: GenerationId,
    pub cooking_context: CookingContext,
    pub fidelity: EvaluationFidelity,
    pub timeout_ms: Option<u64>,
    pub cancel_token: Arc<dyn CancelToken>,
    pub progress_sink: Arc<dyn ProgressSink>,
}

#[derive(Debug)]
pub enum ExecutorError {
    Unavailable { message: String },
    InvalidInput { message: String },
    Timeout,
    Cancelled,
    RuntimeFailed { message: String },
    MaterializationFailed { message: String },
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutorError::Unavailable { message }
            | ExecutorError::InvalidInput { message }
            | ExecutorError::RuntimeFailed { message }
            | ExecutorError::MaterializationFailed { message } => write!(f, "{message}"),
            ExecutorError::Timeout => write!(f, "execution timed out"),
            ExecutorError::Cancelled => write!(f, "execution cancelled"),
        }
    }
}

impl std::error::Error for ExecutorError {}
