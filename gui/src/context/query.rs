use super::Context;
use crate::control::{ControlRole, ResizeEdge};
use crate::cursor::{resolve_cursor, CursorHitDescriptor, CursorHitNode, CursorKind};
use crate::gesture::Gesture;
use crate::renderer::Rect;
use crate::tree::{hit_test_with_animations, resize_hit_at_screen_point, HitChain, NodeId};

pub struct PointerHitQueryResult {
    x: f32,
    y: f32,
    chain: HitChain,
    resize_hit: Option<(NodeId, ResizeEdge)>,
}

impl PointerHitQueryResult {
    fn new(x: f32, y: f32, chain: HitChain, resize_hit: Option<(NodeId, ResizeEdge)>) -> Self {
        Self {
            x,
            y,
            chain,
            resize_hit,
        }
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn matches_point(&self, x: f32, y: f32) -> bool {
        self.x == x && self.y == y
    }

    pub fn chain(&self) -> &HitChain {
        &self.chain
    }

    pub fn resize_hit(&self) -> Option<(NodeId, ResizeEdge)> {
        self.resize_hit
    }
}

impl Context {
    pub(crate) fn hit_test(&self, x: f32, y: f32) -> HitChain {
        let Some(root) = self.tree.root() else {
            return HitChain::empty();
        };
        hit_test_with_animations(&self.tree, root, x, y, Some(&self.animations))
    }

    pub(crate) fn root(&self) -> Option<NodeId> {
        self.tree.root()
    }

    pub(crate) fn node_id_by_name(&self, id: &str) -> Option<NodeId> {
        self.tree.node_by_str(id)
    }

    pub(crate) fn node_exists(&self, id: &str) -> bool {
        self.node_id_by_name(id).is_some()
    }

    pub(crate) fn node_rect(&self, id: &str) -> Option<Rect> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.node_rect_by_node(node_id))
    }

    pub(crate) fn node_rect_by_node(&self, node_id: NodeId) -> Option<Rect> {
        self.tree.get(node_id).map(|node| node.rect)
    }

    pub(crate) fn node_name(&self, node_id: NodeId) -> Option<&str> {
        self.tree.get(node_id).map(|node| node.id.as_ref())
    }

    pub(crate) fn node_scroll_offset(&self, id: &str) -> Option<f32> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.tree.get(node_id))
            .map(|node| node.scroll_offset())
    }

    pub(crate) fn resize_hit_at_screen_point(
        &self,
        x: f32,
        y: f32,
    ) -> Option<(NodeId, ResizeEdge)> {
        let Some(root) = self.tree.root() else {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                x,
                y,
                "query resize edge at screen point: root missing"
            );
            return None;
        };
        let hit = resize_hit_at_screen_point(&self.tree, root, x, y, Some(&self.animations));
        let root_node = self.tree.get(root).map(|node| node.id.to_string());
        let hit_node =
            hit.and_then(|hit| self.tree.get(hit.node_id).map(|node| node.id.to_string()));
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            x,
            y,
            root_node = root_node.as_deref(),
            hit_node = hit_node.as_deref(),
            edge = ?hit.map(|hit| hit.edge),
            "query resize edge at screen point"
        );
        hit.map(|hit| (hit.node_id, hit.edge))
    }

    pub(crate) fn pointer_hit_at(&self, x: f32, y: f32) -> PointerHitQueryResult {
        PointerHitQueryResult::new(
            x,
            y,
            self.hit_test(x, y),
            self.resize_hit_at_screen_point(x, y),
        )
    }

    pub(crate) fn cursor_for_hit(&self, hit: &PointerHitQueryResult) -> CursorKind {
        let desc = self.cursor_hit_descriptor(hit);
        resolve_cursor(&desc)
    }

    pub(crate) fn cursor_at(&self, x: f32, y: f32) -> CursorKind {
        let hit = self.pointer_hit_at(x, y);
        self.cursor_for_hit(&hit)
    }

    fn cursor_hit_descriptor(&self, hit: &PointerHitQueryResult) -> CursorHitDescriptor {
        let nodes_from_leaf_to_root = hit
            .chain()
            .iter()
            .filter_map(|node_id| {
                let node = self.tree.get(node_id)?;
                let gestures = &node.style.gestures;
                Some(CursorHitNode {
                    role: self.node_root_control_role(node_id),
                    draggable: node.style.draggable,
                    has_tap: gestures.contains(&Gesture::Tap),
                    has_double_tap: gestures.contains(&Gesture::DoubleTap),
                    has_drag: gestures.contains(&Gesture::Drag),
                })
            })
            .collect();

        CursorHitDescriptor {
            resize_edge: hit.resize_hit().map(|(_, edge)| edge),
            nodes_from_leaf_to_root,
        }
    }

    pub(crate) fn node_root_control_role(&self, node_id: NodeId) -> Option<ControlRole> {
        let mut candidate = self.node_name(node_id)?;

        loop {
            if let Some(root_id) = self.node_id_by_name(candidate) {
                if let Some(root_node) = self.tree.get(root_id) {
                    if let Some(role) = root_node.props.semantic_role {
                        return Some(role);
                    }
                }
            }

            let (prefix, _) = candidate.rsplit_once("::")?;
            candidate = prefix;
        }
    }

    pub(crate) fn focused_node(&self) -> Option<NodeId> {
        self.interaction.focused()
    }

    pub(crate) fn focused_control_id(&self) -> Option<&str> {
        self.node_name_for(self.focused_node())
    }

    pub(crate) fn hovered_node(&self) -> Option<NodeId> {
        self.interaction.hovered()
    }

    pub(crate) fn hovered_control_id(&self) -> Option<&str> {
        self.node_name_for(self.hovered_node())
    }

    pub(crate) fn captured_node(&self) -> Option<NodeId> {
        self.interaction.captured()
    }

    pub(crate) fn captured_control_id(&self) -> Option<&str> {
        self.node_name_for(self.captured_node())
    }

    fn node_name_for(&self, node_id: Option<NodeId>) -> Option<&str> {
        node_id.and_then(|id| self.node_name(id))
    }
}
