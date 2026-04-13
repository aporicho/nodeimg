use crate::graph::{Connection, Graph};
use crate::node_manager::NodeManager;
use types::NodeId;

#[derive(Debug)]
pub enum ConnectionError {
    NodeNotFound(NodeId),
    ConnectionNotFound,
    CycleDetected,
    SelfConnection,
    InvalidInterface {
        node_id: NodeId,
        interface: String,
    },
    InvalidDirection {
        node_id: NodeId,
        interface: String,
        expected: &'static str,
    },
    TypeIncompatible {
        from: String,
        to: String,
    },
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node {:?} not found", id),
            Self::ConnectionNotFound => write!(f, "Connection not found"),
            Self::CycleDetected => write!(f, "Connection would create a cycle"),
            Self::SelfConnection => write!(f, "Cannot connect a node to itself"),
            Self::InvalidInterface { node_id, interface } => {
                write!(
                    f,
                    "Interface '{}' not found on node {:?}",
                    interface, node_id
                )
            }
            Self::InvalidDirection {
                node_id,
                interface,
                expected,
            } => {
                write!(
                    f,
                    "Interface '{}' on node {:?} is not a valid {}",
                    interface, node_id, expected
                )
            }
            Self::TypeIncompatible { from, to } => {
                write!(
                    f,
                    "Type mismatch: '{}' is not compatible with '{}'",
                    from, to
                )
            }
        }
    }
}

pub fn validate_formal_connection(
    node_manager: &NodeManager,
    graph: &Graph,
    conn: &Connection,
) -> Result<(), ConnectionError> {
    let from_node = graph
        .nodes
        .get(&conn.from.node)
        .ok_or(ConnectionError::NodeNotFound(conn.from.node))?;
    let to_node = graph
        .nodes
        .get(&conn.to.node)
        .ok_or(ConnectionError::NodeNotFound(conn.to.node))?;

    if node_manager
        .get_exposed_pin(&from_node.type_id, &conn.from.interface)
        .is_none()
    {
        return Err(ConnectionError::InvalidInterface {
            node_id: conn.from.node,
            interface: conn.from.interface.clone(),
        });
    }

    if node_manager
        .get_exposed_output_pin(&from_node.type_id, &conn.from.interface)
        .is_none()
    {
        return Err(ConnectionError::InvalidDirection {
            node_id: conn.from.node,
            interface: conn.from.interface.clone(),
            expected: "output",
        });
    }

    if node_manager
        .get_exposed_pin(&to_node.type_id, &conn.to.interface)
        .is_none()
    {
        return Err(ConnectionError::InvalidInterface {
            node_id: conn.to.node,
            interface: conn.to.interface.clone(),
        });
    }

    if node_manager
        .get_exposed_input_pin(&to_node.type_id, &conn.to.interface)
        .is_none()
    {
        return Err(ConnectionError::InvalidDirection {
            node_id: conn.to.node,
            interface: conn.to.interface.clone(),
            expected: "input",
        });
    }

    let from_type = node_manager
        .pin_type(&from_node.type_id, &conn.from.interface)
        .ok_or(ConnectionError::InvalidInterface {
            node_id: conn.from.node,
            interface: conn.from.interface.clone(),
        })?;
    let to_type = node_manager
        .pin_type(&to_node.type_id, &conn.to.interface)
        .ok_or(ConnectionError::InvalidInterface {
            node_id: conn.to.node,
            interface: conn.to.interface.clone(),
        })?;

    if from_type != to_type {
        return Err(ConnectionError::TypeIncompatible {
            from: from_type.0.clone(),
            to: to_type.0.clone(),
        });
    }

    Ok(())
}

pub fn validate_disconnect_request(
    node_manager: &NodeManager,
    graph: &Graph,
    conn: &Connection,
) -> Result<(), ConnectionError> {
    validate_formal_connection(node_manager, graph, conn)?;

    if !graph.connections.iter().any(|existing| existing == conn) {
        return Err(ConnectionError::ConnectionNotFound);
    }

    Ok(())
}

impl std::error::Error for ConnectionError {}

pub fn validate_connection_basic(graph: &Graph, conn: &Connection) -> Result<(), ConnectionError> {
    if conn.from.node == conn.to.node {
        return Err(ConnectionError::SelfConnection);
    }

    if !graph.nodes.contains_key(&conn.from.node) {
        return Err(ConnectionError::NodeNotFound(conn.from.node));
    }
    if !graph.nodes.contains_key(&conn.to.node) {
        return Err(ConnectionError::NodeNotFound(conn.to.node));
    }

    let test_graph = graph.connect(conn.clone());
    if crate::graph::query::topo_sort::topo_sort(&test_graph, conn.to.node).is_err() {
        return Err(ConnectionError::CycleDetected);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::PinRef;
    use crate::node_manager::{ExecutorType, NodeDef, NodeManager, ParamDef, ParamExpose, PinDef};
    use types::{DataType, Value};

    fn make_manager() -> NodeManager {
        let mut nm = NodeManager::new();
        nm.register(NodeDef {
            type_id: "src".into(),
            version: 1,
            source: crate::node_manager::NodeSourceKind::Builtin,
            name: "src".into(),
            category: "test".into(),
            executor_type: ExecutorType::Image,
            requires: vec![],
            purity: crate::node_manager::Purity::Pure,
            cooking_sensitivity: vec![],
            realtime_capable: true,
            execution: crate::node_manager::ExecutionPolicy::default(),
            api: None,
            inputs: vec![],
            outputs: vec![PinDef {
                name: "image".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "strength".into(),
                data_type: DataType::float(),
                constraint: None,
                default_value: Value::Float(0.0),
                expose: vec![ParamExpose::Output],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        });
        nm.register(NodeDef {
            type_id: "dst".into(),
            version: 1,
            source: crate::node_manager::NodeSourceKind::Builtin,
            name: "dst".into(),
            category: "test".into(),
            executor_type: ExecutorType::Image,
            requires: vec![],
            purity: crate::node_manager::Purity::Pure,
            cooking_sensitivity: vec![],
            realtime_capable: true,
            execution: crate::node_manager::ExecutionPolicy::default(),
            api: None,
            inputs: vec![PinDef {
                name: "image".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            outputs: vec![],
            params: vec![ParamDef {
                name: "strength".into(),
                data_type: DataType::float(),
                constraint: None,
                default_value: Value::Float(0.0),
                expose: vec![ParamExpose::Input],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        });
        nm
    }

    #[test]
    fn test_self_connection_rejected() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let result = validate_connection_basic(
            &g,
            &Connection {
                from: PinRef {
                    node: a,
                    interface: "out".into(),
                },
                to: PinRef {
                    node: a,
                    interface: "in".into(),
                },
            },
        );
        assert!(matches!(result, Err(ConnectionError::SelfConnection)));
    }

    #[test]
    fn test_node_not_found() {
        let g = Graph::new();
        let result = validate_connection_basic(
            &g,
            &Connection {
                from: PinRef {
                    node: NodeId(99),
                    interface: "out".into(),
                },
                to: PinRef {
                    node: NodeId(100),
                    interface: "in".into(),
                },
            },
        );
        assert!(matches!(result, Err(ConnectionError::NodeNotFound(_))));
    }

    #[test]
    fn test_cycle_detected() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let g = g.connect(Connection {
            from: PinRef {
                node: a,
                interface: "out".into(),
            },
            to: PinRef {
                node: b,
                interface: "in".into(),
            },
        });
        let result = validate_connection_basic(
            &g,
            &Connection {
                from: PinRef {
                    node: b,
                    interface: "out".into(),
                },
                to: PinRef {
                    node: a,
                    interface: "in".into(),
                },
            },
        );
        assert!(matches!(result, Err(ConnectionError::CycleDetected)));
    }

    #[test]
    fn test_valid_connection() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let result = validate_connection_basic(
            &g,
            &Connection {
                from: PinRef {
                    node: a,
                    interface: "out".into(),
                },
                to: PinRef {
                    node: b,
                    interface: "in".into(),
                },
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_formal_connection_rejects_wrong_direction() {
        let nm = make_manager();
        let g = Graph::new();
        let (g, a) = g.add_node("src", Default::default());
        let (g, b) = g.add_node("dst", Default::default());

        let result = validate_formal_connection(
            &nm,
            &g,
            &Connection {
                from: PinRef {
                    node: b,
                    interface: "image".into(),
                },
                to: PinRef {
                    node: a,
                    interface: "image".into(),
                },
            },
        );

        assert!(matches!(
            result,
            Err(ConnectionError::InvalidDirection { .. })
        ));
    }

    #[test]
    fn test_formal_connection_accepts_param_expose() {
        let nm = make_manager();
        let g = Graph::new();
        let (g, a) = g.add_node("src", Default::default());
        let (g, b) = g.add_node("dst", Default::default());

        let result = validate_formal_connection(
            &nm,
            &g,
            &Connection {
                from: PinRef {
                    node: a,
                    interface: "strength".into(),
                },
                to: PinRef {
                    node: b,
                    interface: "strength".into(),
                },
            },
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_disconnect_request_requires_existing_connection() {
        let nm = make_manager();
        let g = Graph::new();
        let (g, a) = g.add_node("src", Default::default());
        let (g, b) = g.add_node("dst", Default::default());

        let result = validate_disconnect_request(
            &nm,
            &g,
            &Connection {
                from: PinRef {
                    node: a,
                    interface: "strength".into(),
                },
                to: PinRef {
                    node: b,
                    interface: "strength".into(),
                },
            },
        );

        assert!(matches!(result, Err(ConnectionError::ConnectionNotFound)));
    }
}
