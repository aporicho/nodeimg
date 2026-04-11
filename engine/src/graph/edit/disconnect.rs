use crate::graph::{Graph, PinRef};

impl Graph {
    pub fn disconnect(&self, from: &PinRef, to: &PinRef) -> Graph {
        let mut g = self.clone();
        g.connections.retain(|c| !(c.from == *from && c.to == *to));
        g
    }
}
