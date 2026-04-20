use crate::graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};
use types::NodeId;

#[derive(Debug)]
pub struct CycleError;

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cycle detected in node graph")
    }
}

impl std::error::Error for CycleError {}

pub fn topo_sort(graph: &Graph, target: NodeId) -> Result<Vec<NodeId>, CycleError> {
    let mut deps: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
    let mut all_nodes: HashSet<NodeId> = HashSet::new();
    let mut to_visit = VecDeque::new();
    to_visit.push_back(target);
    let mut visited = HashSet::new();

    while let Some(node) = to_visit.pop_front() {
        if !visited.insert(node) {
            continue;
        }
        all_nodes.insert(node);
        for conn in &graph.connections {
            if conn.to.node == node {
                deps.entry(node).or_default().insert(conn.from.node);
                to_visit.push_back(conn.from.node);
            }
        }
    }

    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
    for &n in &all_nodes {
        let deg = deps.get(&n).map_or(0, |d| d.len());
        in_degree.insert(n, deg);
    }

    let mut queue: VecDeque<NodeId> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&n, _)| n)
        .collect();

    let mut order = Vec::new();
    while let Some(n) = queue.pop_front() {
        order.push(n);
        for (&node, node_deps) in &deps {
            if node_deps.contains(&n) {
                let deg = in_degree.get_mut(&node).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(node);
                }
            }
        }
    }

    if order.len() != all_nodes.len() {
        return Err(CycleError);
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Connection, Graph, PinRef};

    #[test]
    fn test_topo_sort_linear() {
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
        let order = topo_sort(&g, c).unwrap();
        assert_eq!(order, vec![a, b, c]);
    }

    #[test]
    fn test_topo_sort_cycle() {
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
        let g = g.connect(Connection {
            from: PinRef {
                node: b,
                interface: "out".into(),
            },
            to: PinRef {
                node: a,
                interface: "in".into(),
            },
        });
        assert!(topo_sort(&g, a).is_err());
    }

    #[test]
    fn test_topo_sort_single_node() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let order = topo_sort(&g, a).unwrap();
        assert_eq!(order, vec![a]);
    }

    #[test]
    fn test_topo_sort_diamond() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let (g, c) = g.add_node("c", Default::default());
        let (g, d) = g.add_node("d", Default::default());
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
                node: d,
                interface: "in1".into(),
            },
        });
        let g = g.connect(Connection {
            from: PinRef {
                node: c,
                interface: "out".into(),
            },
            to: PinRef {
                node: d,
                interface: "in2".into(),
            },
        });
        let order = topo_sort(&g, d).unwrap();
        assert_eq!(order[0], a);
        assert_eq!(*order.last().unwrap(), d);
    }
}
