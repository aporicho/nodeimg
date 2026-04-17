use crate::execution::{CookingContextRange, ExecutionMode};
use crate::facade::ExecutionRequest;
use crate::graph::Graph;
use crate::node_manager::NodeManager;

pub(crate) struct PlanRequest<'a> {
    pub graph: &'a Graph,
    pub request: &'a ExecutionRequest,
    pub mode: ExecutionMode,
    pub cooking_range: &'a CookingContextRange,
    pub node_manager: &'a NodeManager,
}
