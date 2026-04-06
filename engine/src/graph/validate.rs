use super::{Connection, Graph};
use types::NodeId;

#[derive(Debug)]
pub enum ConnectionError {
    NodeNotFound(NodeId),
    CycleDetected,
    SelfConnection,
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node {:?} not found", id),
            Self::CycleDetected => write!(f, "Connection would create a cycle"),
            Self::SelfConnection => write!(f, "Cannot connect a node to itself"),
        }
    }
}
impl std::error::Error for ConnectionError {}

/// 基础连接校验：节点存在 + 自连接 + 环检测。
/// 类型兼容性检查将在 GraphController (Task 11) 层通过 NodeManager 处理。
pub fn validate_connection_basic(
    graph: &Graph,
    conn: &Connection,
) -> Result<(), ConnectionError> {
    if conn.from_node == conn.to_node {
        return Err(ConnectionError::SelfConnection);
    }

    if !graph.nodes.contains_key(&conn.from_node) {
        return Err(ConnectionError::NodeNotFound(conn.from_node));
    }
    if !graph.nodes.contains_key(&conn.to_node) {
        return Err(ConnectionError::NodeNotFound(conn.to_node));
    }

    // 环检测：假设新连线存在，跑 topo_sort
    let test_graph = graph.connect(conn.clone());
    if super::topology::topo_sort(&test_graph, conn.to_node).is_err() {
        return Err(ConnectionError::CycleDetected);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Connection};
    use types::Vec2;

    #[test]
    fn test_self_connection_rejected() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Vec2::default(), Default::default());
        let result = validate_connection_basic(&g, &Connection {
            from_node: a, from_pin: "out".into(),
            to_node: a, to_pin: "in".into(),
        });
        assert!(matches!(result, Err(ConnectionError::SelfConnection)));
    }

    #[test]
    fn test_node_not_found() {
        let g = Graph::new();
        let result = validate_connection_basic(&g, &Connection {
            from_node: NodeId(99), from_pin: "out".into(),
            to_node: NodeId(100), to_pin: "in".into(),
        });
        assert!(matches!(result, Err(ConnectionError::NodeNotFound(_))));
    }

    #[test]
    fn test_cycle_detected() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Vec2::default(), Default::default());
        let (g, b) = g.add_node("b", Vec2::default(), Default::default());
        let g = g.connect(Connection { from_node: a, from_pin: "out".into(), to_node: b, to_pin: "in".into() });
        let result = validate_connection_basic(&g, &Connection {
            from_node: b, from_pin: "out".into(),
            to_node: a, to_pin: "in".into(),
        });
        assert!(matches!(result, Err(ConnectionError::CycleDetected)));
    }

    #[test]
    fn test_valid_connection() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Vec2::default(), Default::default());
        let (g, b) = g.add_node("b", Vec2::default(), Default::default());
        let result = validate_connection_basic(&g, &Connection {
            from_node: a, from_pin: "out".into(),
            to_node: b, to_pin: "in".into(),
        });
        assert!(result.is_ok());
    }
}
