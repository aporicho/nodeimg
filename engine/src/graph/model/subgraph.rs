use types::NodeId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecuteTarget {
    Graph,
    Node(NodeId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubgraphInfo {
    pub target: ExecuteTarget,
    pub nodes: Vec<NodeId>,
    pub order: Vec<NodeId>,
}
