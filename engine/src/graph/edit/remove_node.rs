use crate::graph::Graph;
use types::NodeId;

impl Graph {
    pub fn remove_node(&self, id: NodeId) -> Graph {
        let mut g = self.clone();
        g.nodes.remove(&id);
        g.connections
            .retain(|c| c.from.node != id && c.to.node != id);
        g
    }
}
