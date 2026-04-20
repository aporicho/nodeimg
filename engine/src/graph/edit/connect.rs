use crate::graph::{Connection, Graph};

impl Graph {
    pub fn connect(&self, conn: Connection) -> Graph {
        let mut g = self.clone();
        g.connections
            .retain(|c| !(c.to.node == conn.to.node && c.to.interface == conn.to.interface));
        g.connections.push(conn);
        g
    }
}
