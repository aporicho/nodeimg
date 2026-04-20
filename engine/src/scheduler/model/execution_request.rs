use types::NodeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecuteTarget {
    Graph,
    Node(NodeId),
    Force(NodeId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionRequest {
    pub target: ExecuteTarget,
}

impl ExecutionRequest {
    pub fn new(target: ExecuteTarget) -> Self {
        Self { target }
    }
}
