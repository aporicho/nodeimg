use crate::tree::NodeId;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct CanvasSceneModel {
    nodes: BTreeMap<String, CanvasSceneNode>,
    connections: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct CanvasSceneNode {
    pub owner_id: String,
    pub root_node: NodeId,
}

#[derive(Clone, Debug)]
pub struct CanvasSceneConnection {
    pub id: String,
    pub root_node: NodeId,
}

impl CanvasSceneModel {
    pub fn node(&self, owner_id: &str) -> Option<&CanvasSceneNode> {
        self.nodes.get(owner_id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&str, &CanvasSceneNode)> {
        self.nodes
            .iter()
            .map(|(owner_id, node)| (owner_id.as_str(), node))
    }

    pub fn insert_node(&mut self, owner_id: impl Into<String>, root_node: NodeId) {
        let owner_id = owner_id.into();
        self.nodes.insert(
            owner_id.clone(),
            CanvasSceneNode {
                owner_id,
                root_node,
            },
        );
    }

    pub fn remove_node(&mut self, owner_id: &str) -> Option<CanvasSceneNode> {
        self.nodes.remove(owner_id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn insert_connection(&mut self, connection_id: impl Into<String>) {
        self.connections.insert(connection_id.into());
    }

    pub fn has_connection(&self, connection_id: &str) -> bool {
        self.connections.contains(connection_id)
    }

    pub fn connections(&self) -> impl Iterator<Item = &str> {
        self.connections.iter().map(String::as_str)
    }

    pub fn remove_connection(&mut self, connection_id: &str) -> bool {
        self.connections.remove(connection_id)
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}
