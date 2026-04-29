use crate::geometry::TransformSpec;
use crate::renderer::{Point, Rect};
use crate::tree::layout::Decoration;
use crate::tree::NodeId;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct CanvasSceneModel {
    canvas_transform: Option<TransformSpec>,
    grid_rect: Option<Rect>,
    nodes: BTreeMap<String, CanvasSceneNode>,
    connections: BTreeMap<String, CanvasSceneConnection>,
    pending_connection: Option<CanvasPendingConnectionState>,
}

#[derive(Clone, Debug)]
pub struct CanvasSceneNode {
    pub owner_id: String,
    pub root_node: NodeId,
    pub state: CanvasSceneNodeState,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasSceneNodeState {
    pub rect: Option<Rect>,
    pub z_index: Option<i32>,
    pub label: Option<String>,
    pub card_decoration: Option<Option<Decoration>>,
    pub param_texts: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasSceneConnection {
    pub id: String,
    pub from_port: String,
    pub to_port: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasPendingConnectionState {
    pub from_port: String,
    pub cursor_canvas: Point,
}

impl CanvasSceneModel {
    pub fn set_canvas_transform(&mut self, transform: TransformSpec) -> bool {
        if self.canvas_transform == Some(transform) {
            return false;
        }
        self.canvas_transform = Some(transform);
        true
    }

    pub fn set_grid_rect(&mut self, rect: Rect) -> bool {
        if self.grid_rect == Some(rect) {
            return false;
        }
        self.grid_rect = Some(rect);
        true
    }

    pub fn node(&self, owner_id: &str) -> Option<&CanvasSceneNode> {
        self.nodes.get(owner_id)
    }

    pub fn node_state(&self, owner_id: &str) -> Option<&CanvasSceneNodeState> {
        self.nodes.get(owner_id).map(|node| &node.state)
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&str, &CanvasSceneNode)> {
        self.nodes
            .iter()
            .map(|(owner_id, node)| (owner_id.as_str(), node))
    }

    pub fn insert_node(&mut self, owner_id: impl Into<String>, root_node: NodeId) {
        self.insert_node_state(owner_id, root_node, CanvasSceneNodeState::default());
    }

    pub fn insert_node_state(
        &mut self,
        owner_id: impl Into<String>,
        root_node: NodeId,
        state: CanvasSceneNodeState,
    ) {
        let owner_id = owner_id.into();
        self.nodes.insert(
            owner_id.clone(),
            CanvasSceneNode {
                owner_id,
                root_node,
                state,
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
        self.insert_connection_state(connection_id, String::new(), String::new());
    }

    pub fn insert_connection_state(
        &mut self,
        connection_id: impl Into<String>,
        from_port: String,
        to_port: String,
    ) {
        let id = connection_id.into();
        self.connections.insert(
            id.clone(),
            CanvasSceneConnection {
                id,
                from_port,
                to_port,
            },
        );
    }

    pub fn has_connection(&self, connection_id: &str) -> bool {
        self.connections.contains_key(connection_id)
    }

    pub fn connection(&self, connection_id: &str) -> Option<&CanvasSceneConnection> {
        self.connections.get(connection_id)
    }

    pub fn connections(&self) -> impl Iterator<Item = &str> {
        self.connections.keys().map(String::as_str)
    }

    pub fn remove_connection(&mut self, connection_id: &str) -> Option<CanvasSceneConnection> {
        self.connections.remove(connection_id)
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub fn set_pending_connection(&mut self, state: Option<CanvasPendingConnectionState>) -> bool {
        if self.pending_connection == state {
            return false;
        }
        self.pending_connection = state;
        true
    }

    pub fn pending_connection(&self) -> Option<&CanvasPendingConnectionState> {
        self.pending_connection.as_ref()
    }
}
