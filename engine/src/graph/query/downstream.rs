use crate::graph::Graph;
use std::collections::{HashSet, VecDeque};
use types::NodeId;

pub fn downstream(graph: &Graph, id: NodeId) -> HashSet<NodeId> {
    let mut result = HashSet::new();
    let mut to_visit = VecDeque::new();
    to_visit.push_back(id);
    while let Some(node) = to_visit.pop_front() {
        for conn in &graph.connections {
            if conn.from.node == node && result.insert(conn.to.node) {
                to_visit.push_back(conn.to.node);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Connection, Graph, PinRef};

    #[test]
    fn test_downstream() {
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
                node: b,
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
        assert_eq!(downstream(&g, a), [b, c].into_iter().collect());
    }
}
