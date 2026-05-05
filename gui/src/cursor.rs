use crate::control::ResizeEdge;
use crate::tree::SemanticRole;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CursorHitNode {
    pub(crate) role: Option<SemanticRole>,
    pub(crate) draggable: bool,
    pub(crate) has_tap: bool,
    pub(crate) has_double_tap: bool,
    pub(crate) has_drag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CursorHitDescriptor {
    pub(crate) resize_edge: Option<ResizeEdge>,
    pub(crate) nodes_from_leaf_to_root: Vec<CursorHitNode>,
}

pub(crate) fn resolve_cursor(desc: &CursorHitDescriptor) -> CursorKind {
    if let Some(edge) = desc.resize_edge {
        return CursorKind::Resize(edge);
    }

    for node in &desc.nodes_from_leaf_to_root {
        if node.role.is_some_and(semantic_role_uses_text_cursor) {
            return CursorKind::Text;
        }
        if node.draggable {
            return CursorKind::Move;
        }
        if node.role.is_some_and(semantic_role_uses_pointer_cursor)
            || node.has_tap
            || node.has_double_tap
            || node.has_drag
        {
            return CursorKind::Pointer;
        }
    }

    CursorKind::Default
}

fn semantic_role_uses_text_cursor(role: SemanticRole) -> bool {
    matches!(
        role,
        SemanticRole::TextInput | SemanticRole::TextArea | SemanticRole::NumberInput
    )
}

fn semantic_role_uses_pointer_cursor(role: SemanticRole) -> bool {
    matches!(
        role,
        SemanticRole::Button
            | SemanticRole::Checkbox
            | SemanticRole::Collapsible
            | SemanticRole::Dropdown
            | SemanticRole::Radio
            | SemanticRole::Slider
            | SemanticRole::Toggle
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(role: Option<SemanticRole>) -> CursorHitNode {
        CursorHitNode {
            role,
            draggable: false,
            has_tap: false,
            has_double_tap: false,
            has_drag: false,
        }
    }

    fn descriptor(nodes_from_leaf_to_root: Vec<CursorHitNode>) -> CursorHitDescriptor {
        CursorHitDescriptor {
            resize_edge: None,
            nodes_from_leaf_to_root,
        }
    }

    #[test]
    fn resize_cursor_has_priority_over_draggable() {
        let desc = CursorHitDescriptor {
            resize_edge: Some(ResizeEdge::Right),
            nodes_from_leaf_to_root: vec![CursorHitNode {
                draggable: true,
                ..node(None)
            }],
        };

        assert_eq!(resolve_cursor(&desc), CursorKind::Resize(ResizeEdge::Right));
    }

    #[test]
    fn draggable_node_resolves_to_move() {
        let desc = descriptor(vec![CursorHitNode {
            draggable: true,
            has_tap: true,
            has_drag: true,
            ..node(None)
        }]);

        assert_eq!(resolve_cursor(&desc), CursorKind::Move);
    }

    #[test]
    fn interactive_role_resolves_to_pointer() {
        let desc = descriptor(vec![node(Some(SemanticRole::Button))]);

        assert_eq!(resolve_cursor(&desc), CursorKind::Pointer);
    }

    #[test]
    fn gesture_resolves_to_pointer() {
        let desc = descriptor(vec![CursorHitNode {
            has_tap: true,
            ..node(None)
        }]);

        assert_eq!(resolve_cursor(&desc), CursorKind::Pointer);
    }

    #[test]
    fn text_roles_resolve_to_text() {
        for role in [
            SemanticRole::TextInput,
            SemanticRole::TextArea,
            SemanticRole::NumberInput,
        ] {
            let desc = descriptor(vec![node(Some(role))]);

            assert_eq!(resolve_cursor(&desc), CursorKind::Text);
        }
    }

    #[test]
    fn plain_hit_resolves_to_default() {
        let desc = descriptor(vec![node(None)]);

        assert_eq!(resolve_cursor(&desc), CursorKind::Default);
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
