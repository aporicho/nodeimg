use crate::control::ResizeEdge;
use crate::gesture::Gesture;
use crate::tree::TargetChain;
use winit::window::CursorIcon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorKind {
    Default,
    Pointer,
    Text,
    Move,
    Resize(ResizeEdge),
}

impl CursorKind {
    fn to_winit(self) -> CursorIcon {
        match self {
            Self::Default => CursorIcon::Default,
            Self::Pointer => CursorIcon::Pointer,
            Self::Text => CursorIcon::Text,
            Self::Move => CursorIcon::Move,
            Self::Resize(ResizeEdge::Top | ResizeEdge::Bottom) => CursorIcon::NsResize,
            Self::Resize(ResizeEdge::Left | ResizeEdge::Right) => CursorIcon::EwResize,
            Self::Resize(ResizeEdge::TopLeft | ResizeEdge::BottomRight) => CursorIcon::NwseResize,
            Self::Resize(ResizeEdge::TopRight | ResizeEdge::BottomLeft) => CursorIcon::NeswResize,
        }
    }
}

pub struct CursorState {
    desired: CursorKind,
    applied: CursorKind,
}

impl CursorState {
    pub fn new() -> Self {
        Self {
            desired: CursorKind::Default,
            applied: CursorKind::Default,
        }
    }

    pub fn desired(&self) -> CursorKind {
        self.desired
    }

    pub fn set(&mut self, cursor: CursorKind) -> bool {
        let changed = self.desired != cursor;
        self.desired = cursor;
        changed
    }

    pub(crate) fn apply_to_window(&mut self, window: &winit::window::Window) {
        if self.desired != self.applied {
            tracing::trace!(
                target: "gui::cursor",
                desired = ?self.desired,
                applied = ?self.applied,
                "apply cursor to window"
            );
            window.set_cursor(self.desired.to_winit());
            self.applied = self.desired;
        } else {
            tracing::trace!(
                target: "gui::cursor",
                desired = ?self.desired,
                applied = ?self.applied,
                "skip cursor apply because cached cursor matches desired cursor"
            );
        }
    }
}

impl Default for CursorState {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn resolve_cursor(resize_edge: Option<ResizeEdge>, targets: &TargetChain) -> CursorKind {
    if let Some(edge) = resize_edge {
        return CursorKind::Resize(edge);
    }

    for target in targets.iter() {
        if target.uses_text_cursor() {
            return CursorKind::Text;
        }
        if target.is_draggable() {
            return CursorKind::Move;
        }
        if target.uses_pointer_cursor()
            || target.has_gesture(Gesture::Tap)
            || target.has_gesture(Gesture::DoubleTap)
            || target.has_gesture(Gesture::Drag)
        {
            return CursorKind::Pointer;
        }
    }

    CursorKind::Default
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::tree::layout::BoxStyle;
    use crate::tree::{HitChain, SemanticRole, Tree, TreeNode, TreeNodeBuilder};

    fn node(id: &'static str, role: Option<SemanticRole>, style: BoxStyle) -> TreeNode {
        let mut builder = TreeNodeBuilder::container(id, style).rect(Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 40.0,
        });
        if let Some(role) = role {
            builder = builder.semantic_role(role);
        }
        builder.build()
    }

    fn targets(nodes_from_leaf_to_root: Vec<TreeNode>) -> (Tree, HitChain) {
        let mut tree = Tree::new();
        let ids = nodes_from_leaf_to_root
            .into_iter()
            .map(|node| tree.insert(node))
            .collect();
        (tree, HitChain::new(ids))
    }

    fn resolve_for_nodes(resize_edge: Option<ResizeEdge>, nodes: Vec<TreeNode>) -> CursorKind {
        let (tree, chain) = targets(nodes);
        let targets = TargetChain::from_hit_chain(&tree, &chain);
        resolve_cursor(resize_edge, &targets)
    }

    fn plain_node() -> TreeNode {
        node("plain", None, BoxStyle::default())
    }

    fn role_node(role: SemanticRole) -> TreeNode {
        node(
            "role",
            Some(role),
            BoxStyle {
                hittable: true,
                ..Default::default()
            },
        )
    }

    fn draggable_node() -> TreeNode {
        node(
            "drag",
            None,
            BoxStyle {
                draggable: true,
                gestures: vec![Gesture::Tap, Gesture::Drag],
                ..Default::default()
            },
        )
    }

    fn gesture_node() -> TreeNode {
        node(
            "tap",
            None,
            BoxStyle {
                gestures: vec![Gesture::Tap],
                ..Default::default()
            },
        )
    }

    fn text_child_node() -> (Tree, HitChain) {
        let mut tree = Tree::new();
        tree.insert(node(
            "field",
            Some(SemanticRole::TextInput),
            BoxStyle::default(),
        ));
        let child = tree.insert(node("field::value", None, BoxStyle::default()));
        (tree, HitChain::new(vec![child]))
    }

    #[test]
    fn cursor_uses_semantic_root_role_for_prefixed_parts() {
        let (tree, chain) = text_child_node();
        let targets = TargetChain::from_hit_chain(&tree, &chain);

        assert_eq!(resolve_cursor(None, &targets), CursorKind::Text);
    }

    #[test]
    fn resize_cursor_has_priority_over_draggable() {
        assert_eq!(
            resolve_for_nodes(Some(ResizeEdge::Right), vec![draggable_node()]),
            CursorKind::Resize(ResizeEdge::Right)
        );
    }

    #[test]
    fn draggable_node_resolves_to_move() {
        assert_eq!(
            resolve_for_nodes(None, vec![draggable_node()]),
            CursorKind::Move
        );
    }

    #[test]
    fn interactive_role_resolves_to_pointer() {
        assert_eq!(
            resolve_for_nodes(None, vec![role_node(SemanticRole::Button)]),
            CursorKind::Pointer
        );
    }

    #[test]
    fn gesture_resolves_to_pointer() {
        assert_eq!(
            resolve_for_nodes(None, vec![gesture_node()]),
            CursorKind::Pointer
        );
    }

    #[test]
    fn text_roles_resolve_to_text() {
        for role in [
            SemanticRole::TextInput,
            SemanticRole::TextArea,
            SemanticRole::NumberInput,
        ] {
            assert_eq!(
                resolve_for_nodes(None, vec![role_node(role)]),
                CursorKind::Text
            );
        }
    }

    #[test]
    fn plain_hit_resolves_to_default() {
        assert_eq!(
            resolve_for_nodes(None, vec![plain_node()]),
            CursorKind::Default
        );
    }

    #[test]
    fn cursor_state_persists_desired_cursor_until_changed() {
        let mut state = CursorState::new();

        assert!(state.set(CursorKind::Pointer));

        assert_eq!(state.desired(), CursorKind::Pointer);
        assert_eq!(state.desired(), CursorKind::Pointer);
        assert!(!state.set(CursorKind::Pointer));
    }
}
