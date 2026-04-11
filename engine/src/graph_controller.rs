use crate::graph::{validate, version::GraphState, Connection, Graph};
use crate::node_manager::NodeManager;
use std::sync::Arc;
use types::{NodeId, Value, Vec2};

pub struct GraphController {
    state: GraphState,
    node_manager: Arc<NodeManager>,
}

impl GraphController {
    pub fn new(node_manager: Arc<NodeManager>, max_undo: usize) -> Self {
        Self {
            state: GraphState::new(max_undo),
            node_manager,
        }
    }

    pub fn current(&self) -> &Graph {
        self.state.current()
    }
    pub fn snapshot(&self) -> Arc<Graph> {
        self.state.snapshot()
    }

    pub fn add_node(&mut self, type_id: &str, position: Vec2) -> Result<NodeId, String> {
        if self.node_manager.get(type_id).is_none() {
            return Err(format!("Unknown node type: {}", type_id));
        }
        let defaults = self
            .node_manager
            .default_params(type_id)
            .unwrap_or_default();
        let (graph, id) = self.state.current().add_node(type_id, position, defaults);
        self.state.commit(graph);
        Ok(id)
    }

    pub fn remove_node(&mut self, id: NodeId) -> Result<(), String> {
        if !self.state.current().nodes.contains_key(&id) {
            return Err(format!("Node {:?} not found", id));
        }
        let graph = self.state.current().remove_node(id);
        self.state.commit(graph);
        Ok(())
    }

    pub fn connect(&mut self, conn: Connection) -> Result<(), validate::ConnectionError> {
        // Basic validation (node existence, self-connection, cycle detection)
        validate::validate_connection_basic(self.state.current(), &conn)?;

        // Type compatibility check via NodeManager
        let from_node = self
            .state
            .current()
            .nodes
            .get(&conn.from_node)
            .ok_or(validate::ConnectionError::NodeNotFound(conn.from_node))?;
        let to_node = self
            .state
            .current()
            .nodes
            .get(&conn.to_node)
            .ok_or(validate::ConnectionError::NodeNotFound(conn.to_node))?;

        if let (Some(from_type), Some(to_type)) = (
            self.node_manager
                .pin_type(&from_node.type_id, &conn.from_pin),
            self.node_manager.pin_type(&to_node.type_id, &conn.to_pin),
        ) {
            if from_type != to_type {
                return Err(validate::ConnectionError::TypeIncompatible {
                    from: from_type.0.clone(),
                    to: to_type.0.clone(),
                });
            }
        }

        let graph = self.state.current().connect(conn);
        self.state.commit(graph);
        Ok(())
    }

    pub fn disconnect(&mut self, from: NodeId, from_pin: &str, to: NodeId, to_pin: &str) {
        let graph = self.state.current().disconnect(from, from_pin, to, to_pin);
        self.state.commit(graph);
    }

    pub fn set_param(&mut self, id: NodeId, key: &str, value: Value, preview: bool) {
        let graph = self.state.current().set_param(id, key, value);
        if preview {
            self.state.preview(graph);
        } else {
            self.state.commit(graph);
        }
    }

    pub fn move_node(&mut self, id: NodeId, position: Vec2) {
        let graph = self.state.current().move_node(id, position);
        self.state.preview(graph);
    }

    pub fn undo(&mut self) -> bool {
        self.state.undo()
    }
    pub fn redo(&mut self) -> bool {
        self.state.redo()
    }
    pub fn replace(&mut self, graph: Graph) {
        self.state.replace(graph);
    }
}
