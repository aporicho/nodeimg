use super::Graph;
use types::NodeId;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug)]
pub struct CycleError;

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cycle detected in node graph")
    }
}
impl std::error::Error for CycleError {}

/// 从 target 回溯上游，拓扑排序返回执行顺序。
pub fn topo_sort(graph: &Graph, target: NodeId) -> Result<Vec<NodeId>, CycleError> {
    let mut deps: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
    let mut all_nodes: HashSet<NodeId> = HashSet::new();
    let mut to_visit = VecDeque::new();
    to_visit.push_back(target);
    let mut visited = HashSet::new();

    while let Some(node) = to_visit.pop_front() {
        if !visited.insert(node) { continue; }
        all_nodes.insert(node);
        for conn in &graph.connections {
            if conn.to_node == node {
                deps.entry(node).or_default().insert(conn.from_node);
                to_visit.push_back(conn.from_node);
            }
        }
    }

    // Kahn's algorithm
    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
    for &n in &all_nodes {
        let deg = deps.get(&n).map_or(0, |d| d.len());
        in_degree.insert(n, deg);
    }

    let mut queue: VecDeque<NodeId> = in_degree.iter()
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
                if *deg == 0 { queue.push_back(node); }
            }
        }
    }

    if order.len() != all_nodes.len() {
        return Err(CycleError);
    }
    Ok(order)
}

/// target 的所有上游节点（不含 target 自身）。
pub fn upstream(graph: &Graph, id: NodeId) -> HashSet<NodeId> {
    let mut result = HashSet::new();
    let mut to_visit = VecDeque::new();
    to_visit.push_back(id);
    while let Some(node) = to_visit.pop_front() {
        for conn in &graph.connections {
            if conn.to_node == node && result.insert(conn.from_node) {
                to_visit.push_back(conn.from_node);
            }
        }
    }
    result
}

/// target 的所有下游节点（不含 target 自身）。
pub fn downstream(graph: &Graph, id: NodeId) -> HashSet<NodeId> {
    let mut result = HashSet::new();
    let mut to_visit = VecDeque::new();
    to_visit.push_back(id);
    while let Some(node) = to_visit.pop_front() {
        for conn in &graph.connections {
            if conn.from_node == node && result.insert(conn.to_node) {
                to_visit.push_back(conn.to_node);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Connection};
    use types::Vec2;

    #[test]
    fn test_topo_sort_linear() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Vec2::default(), Default::default());
        let (g, b) = g.add_node("b", Vec2::default(), Default::default());
        let (g, c) = g.add_node("c", Vec2::default(), Default::default());
        let g = g.connect(Connection { from_node: a, from_pin: "out".into(), to_node: b, to_pin: "in".into() });
        let g = g.connect(Connection { from_node: b, from_pin: "out".into(), to_node: c, to_pin: "in".into() });
        let order = topo_sort(&g, c).unwrap();
        assert_eq!(order, vec![a, b, c]);
    }

    #[test]
    fn test_topo_sort_cycle() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Vec2::default(), Default::default());
        let (g, b) = g.add_node("b", Vec2::default(), Default::default());
        let g = g.connect(Connection { from_node: a, from_pin: "out".into(), to_node: b, to_pin: "in".into() });
        let g = g.connect(Connection { from_node: b, from_pin: "out".into(), to_node: a, to_pin: "in".into() });
        assert!(topo_sort(&g, a).is_err());
    }

    #[test]
    fn test_topo_sort_single_node() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Vec2::default(), Default::default());
        let order = topo_sort(&g, a).unwrap();
        assert_eq!(order, vec![a]);
    }

    #[test]
    fn test_topo_sort_diamond() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Vec2::default(), Default::default());
        let (g, b) = g.add_node("b", Vec2::default(), Default::default());
        let (g, c) = g.add_node("c", Vec2::default(), Default::default());
        let (g, d) = g.add_node("d", Vec2::default(), Default::default());
        let g = g.connect(Connection { from_node: a, from_pin: "out".into(), to_node: b, to_pin: "in".into() });
        let g = g.connect(Connection { from_node: a, from_pin: "out".into(), to_node: c, to_pin: "in".into() });
        let g = g.connect(Connection { from_node: b, from_pin: "out".into(), to_node: d, to_pin: "in1".into() });
        let g = g.connect(Connection { from_node: c, from_pin: "out".into(), to_node: d, to_pin: "in2".into() });
        let order = topo_sort(&g, d).unwrap();
        assert_eq!(order[0], a); // a must be first
        assert_eq!(*order.last().unwrap(), d); // d must be last
    }

    #[test]
    fn test_upstream_downstream() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Vec2::default(), Default::default());
        let (g, b) = g.add_node("b", Vec2::default(), Default::default());
        let (g, c) = g.add_node("c", Vec2::default(), Default::default());
        let g = g.connect(Connection { from_node: a, from_pin: "out".into(), to_node: b, to_pin: "in".into() });
        let g = g.connect(Connection { from_node: b, from_pin: "out".into(), to_node: c, to_pin: "in".into() });
        assert_eq!(upstream(&g, c), [a, b].into_iter().collect());
        assert_eq!(downstream(&g, a), [b, c].into_iter().collect());
    }
}
