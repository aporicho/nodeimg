use std::future::Future;
use std::pin::Pin;

use crate::capability::{Capability, CapabilityId};
use crate::execution::{
    ExecutionOutputs, ExecutionTerminalStatus, ExecutorError, NodeExecutionRequest,
    PlanLifecycleContext,
};

pub type ExecutorFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ExecutionOutputs, ExecutorError>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalityProfile {
    Any,
    Cpu,
    Gpu,
    Remote,
}

pub trait Executor: Send + Sync {
    fn provides(&self) -> Vec<Capability>;

    fn execute<'a>(
        &'a self,
        cap_id: &'a CapabilityId,
        req: NodeExecutionRequest<'a>,
    ) -> ExecutorFuture<'a>;

    fn on_plan_started(&self, _ctx: &PlanLifecycleContext<'_>) -> Result<(), ExecutorError> {
        Ok(())
    }

    fn on_plan_finished(&self, _ctx: &PlanLifecycleContext<'_>, _status: ExecutionTerminalStatus) {}

    fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Any
    }
}
