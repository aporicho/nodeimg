use super::*;

impl Graph {
    pub fn add_node(
        &self,
        type_id: &str,
        position: Vec2,
        defaults: HashMap<String, Value>,
    ) -> (Graph, NodeId) {
        let (next, id) = self.alloc_id();
        let node = Arc::new(Node {
            id,
            type_id: type_id.to_string(),
            params: defaults,
            position,
        });
        let mut g = self.clone();
        g.next_id = next;
        g.nodes.insert(id, node);
        (g, id)
    }

    pub fn remove_node(&self, id: NodeId) -> Graph {
        let mut g = self.clone();
        g.nodes.remove(&id);
        g.connections.retain(|c| c.from_node != id && c.to_node != id);
        g
    }

    pub fn connect(&self, conn: Connection) -> Graph {
        let mut g = self.clone();
        // 单输入引脚：移除同一 to_pin 的旧连线
        g.connections.retain(|c| {
            !(c.to_node == conn.to_node && c.to_pin == conn.to_pin)
        });
        g.connections.push(conn);
        g
    }

    pub fn disconnect(
        &self,
        from: NodeId,
        from_pin: &str,
        to: NodeId,
        to_pin: &str,
    ) -> Graph {
        let mut g = self.clone();
        g.connections.retain(|c| {
            !(c.from_node == from && c.from_pin == from_pin
                && c.to_node == to && c.to_pin == to_pin)
        });
        g
    }

    pub fn set_param(&self, id: NodeId, key: &str, value: Value) -> Graph {
        let mut g = self.clone();
        if let Some(node_arc) = g.nodes.get_mut(&id) {
            let node = Arc::make_mut(node_arc);
            node.params.insert(key.to_string(), value);
        }
        g
    }

    pub fn move_node(&self, id: NodeId, position: Vec2) -> Graph {
        let mut g = self.clone();
        if let Some(node_arc) = g.nodes.get_mut(&id) {
            let node = Arc::make_mut(node_arc);
            node.position = position;
        }
        g
    }
}
