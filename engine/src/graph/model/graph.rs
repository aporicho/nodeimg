use std::collections::HashMap;
use std::sync::Arc;
use types::{NodeId, Value};

#[derive(Clone, Debug)]
pub struct NodeInstance {
    pub id: NodeId,
    pub type_id: String,
    pub params: HashMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinRef {
    pub node: NodeId,
    pub interface: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Connection {
    pub from: PinRef,
    pub to: PinRef,
}

#[derive(Clone, Debug)]
pub struct Graph {
    pub nodes: HashMap<NodeId, Arc<NodeInstance>>,
    pub connections: Vec<Connection>,
    pub(crate) next_id: u64,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
            next_id: 0,
        }
    }

    pub(crate) fn alloc_id(&self) -> (u64, NodeId) {
        let id = self.next_id;
        (id + 1, NodeId(id))
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

pub type Node = NodeInstance;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_graph_is_empty() {
        let g = Graph::new();
        assert!(g.nodes.is_empty());
        assert!(g.connections.is_empty());
    }

    #[test]
    fn test_add_node_returns_new_graph_and_id() {
        let g = Graph::new();
        let (g2, id) = g.add_node("brightness", Default::default());
        assert_eq!(g.nodes.len(), 0);
        assert_eq!(g2.nodes.len(), 1);
        assert!(g2.nodes.contains_key(&id));
    }

    #[test]
    fn test_remove_node_cascades_connections() {
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
        assert_eq!(g.connections.len(), 1);
        let g = g.remove_node(a);
        assert_eq!(g.connections.len(), 0);
        assert!(!g.nodes.contains_key(&a));
    }

    #[test]
    fn test_connect_single_input_replaces() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let (g, c) = g.add_node("c", Default::default());
        let g = g.connect(Connection {
            from: PinRef {
                node: a,
                interface: "out".into(),
            },
            to: PinRef {
                node: c,
                interface: "in".into(),
            },
        });
        let g = g.connect(Connection {
            from: PinRef {
                node: b,
                interface: "out".into(),
            },
            to: PinRef {
                node: c,
                interface: "in".into(),
            },
        });
        assert_eq!(g.connections.len(), 1);
        assert_eq!(g.connections[0].from.node, b);
    }

    #[test]
    fn test_set_param_structural_sharing() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let old_b_ptr = Arc::as_ptr(g.nodes.get(&b).unwrap());
        let g2 = g.set_param(a, "val", Value::Float(1.0));
        let new_b_ptr = Arc::as_ptr(g2.nodes.get(&b).unwrap());
        assert_eq!(old_b_ptr, new_b_ptr);
    }

    #[test]
    fn test_original_graph_unchanged_after_edit() {
        let g = Graph::new();
        let (g, _) = g.add_node("a", Default::default());
        let g2 = g.set_param(NodeId(0), "x", Value::Float(5.0));
        assert!(g.nodes.get(&NodeId(0)).unwrap().params.get("x").is_none());
        assert!(g2.nodes.get(&NodeId(0)).unwrap().params.get("x").is_some());
    }
}
