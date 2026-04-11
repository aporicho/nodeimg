use crate::graph::model::subgraph::{ExecuteTarget, SubgraphInfo};
use crate::graph::query::topo_sort::{topo_sort, CycleError};
use crate::graph::Graph;
use std::collections::{HashMap, VecDeque};

pub fn resolve_subgraph(graph: &Graph, target: ExecuteTarget) -> Result<SubgraphInfo, CycleError> {
    let (nodes, order) = match target.clone() {
        ExecuteTarget::Graph => {
            let order = topo_sort_all(graph)?;
            let nodes = order.clone();
            (nodes, order)
        }
        ExecuteTarget::Node(node_id) => {
            let order = topo_sort(graph, node_id)?;
            let nodes = order.clone();
            (nodes, order)
        }
    };

    Ok(SubgraphInfo {
        target,
        nodes,
        order,
    })
}

fn topo_sort_all(graph: &Graph) -> Result<Vec<types::NodeId>, CycleError> {
    let mut in_degree: HashMap<types::NodeId, usize> =
        graph.nodes.keys().copied().map(|id| (id, 0)).collect();
    let mut outgoing: HashMap<types::NodeId, Vec<types::NodeId>> = HashMap::new();

    for conn in &graph.connections {
        *in_degree.entry(conn.to.node).or_insert(0) += 1;
        outgoing
            .entry(conn.from.node)
            .or_default()
            .push(conn.to.node);
    }

    let mut queue: VecDeque<_> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut order = Vec::new();

    while let Some(node) = queue.pop_front() {
        order.push(node);
        if let Some(nexts) = outgoing.get(&node) {
            for next in nexts {
                if let Some(deg) = in_degree.get_mut(next) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(*next);
                    }
                }
            }
        }
    }

    if order.len() != graph.nodes.len() {
        return Err(CycleError);
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Connection, Graph, PinRef};
    use std::collections::HashSet;

    #[test]
    fn resolve_node_target_returns_upstream_subgraph() {
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

        let info = resolve_subgraph(&g, ExecuteTarget::Node(c)).unwrap();
        assert_eq!(info.nodes, vec![a, b, c]);
        assert_eq!(info.order, vec![a, b, c]);
    }

    #[test]
    fn resolve_graph_target_returns_all_nodes() {
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

        let info = resolve_subgraph(&g, ExecuteTarget::Graph).unwrap();
        let nodes: HashSet<_> = info.nodes.into_iter().collect();
        assert_eq!(nodes, [a, b].into_iter().collect());
        assert_eq!(info.order, vec![a, b]);
    }
}
