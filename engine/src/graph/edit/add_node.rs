use crate::graph::{Graph, NodeInstance};
use std::collections::HashMap;
use std::sync::Arc;
use types::{NodeId, Value};

impl Graph {
    pub fn add_node(&self, type_id: &str, defaults: HashMap<String, Value>) -> (Graph, NodeId) {
        let (next, id) = self.alloc_id();
        let node = Arc::new(NodeInstance {
            id,
            type_id: type_id.to_string(),
            params: defaults,
        });
        let mut g = self.clone();
        g.next_id = next;
        g.nodes.insert(id, node);
        (g, id)
    }
}
