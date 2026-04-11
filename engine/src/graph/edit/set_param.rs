use crate::graph::Graph;
use types::{NodeId, Value};

impl Graph {
    pub fn set_param(&self, id: NodeId, key: &str, value: Value) -> Graph {
        let mut g = self.clone();
        if let Some(node_arc) = g.nodes.get_mut(&id) {
            let node = std::sync::Arc::make_mut(node_arc);
            node.params.insert(key.to_string(), value);
        }
        g
    }
}
