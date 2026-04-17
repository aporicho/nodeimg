use types::NodeId;

#[derive(Debug)]
pub(crate) enum PlannerError {
    Graph { message: String },
    Schema { message: String },
    NodeNotFound { node_id: NodeId },
    NodeTypeNotRegistered { type_id: String },
}

impl std::fmt::Display for PlannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlannerError::Graph { message } | PlannerError::Schema { message } => {
                write!(f, "{message}")
            }
            PlannerError::NodeNotFound { node_id } => {
                write!(f, "Node {:?} not found in graph", node_id)
            }
            PlannerError::NodeTypeNotRegistered { type_id } => {
                write!(f, "Node type '{type_id}' not registered")
            }
        }
    }
}

impl std::error::Error for PlannerError {}
