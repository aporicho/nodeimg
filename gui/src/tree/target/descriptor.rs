use crate::tree::layout::Gesture;
use crate::tree::{NodeId, SemanticRole, Tree};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetDescriptor {
    node_id: NodeId,
    stable_id: String,
    owner_id: Option<String>,
    own_role: Option<SemanticRole>,
    root_role: Option<SemanticRole>,
    enabled: bool,
    hittable: bool,
    draggable: bool,
    resizable: bool,
    has_action: bool,
    gestures: Vec<Gesture>,
}

impl TargetDescriptor {
    pub(crate) fn from_node(tree: &Tree, node_id: NodeId) -> Option<Self> {
        let node = tree.get(node_id)?;
        Some(Self {
            node_id,
            stable_id: node.id.to_string(),
            owner_id: node.props.owner_id.as_ref().map(ToString::to_string),
            own_role: node.props.semantic_role,
            root_role: semantic_role_for_stable_id(tree, node.id.as_ref()),
            enabled: node.props.enabled,
            hittable: node.style.hittable,
            draggable: node.style.draggable,
            resizable: node.style.resizable,
            has_action: node.props.action_id.is_some(),
            gestures: node.style.gestures.clone(),
        })
    }

    pub(crate) fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub(crate) fn stable_id(&self) -> &str {
        &self.stable_id
    }

    pub(crate) fn owner_id(&self) -> Option<&str> {
        self.owner_id.as_deref()
    }

    pub(crate) fn own_role(&self) -> Option<SemanticRole> {
        self.own_role
    }

    pub(crate) fn root_role(&self) -> Option<SemanticRole> {
        self.root_role
    }

    pub(crate) fn has_own_semantic_role(&self) -> bool {
        self.own_role().is_some()
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn is_hittable(&self) -> bool {
        self.hittable
    }

    pub(crate) fn is_draggable(&self) -> bool {
        self.draggable
    }

    pub(crate) fn is_resizable(&self) -> bool {
        self.resizable
    }

    pub(crate) fn has_gesture(&self, gesture: Gesture) -> bool {
        self.gestures.contains(&gesture)
    }

    pub(crate) fn has_any_gesture(&self) -> bool {
        !self.gestures.is_empty()
    }

    pub(crate) fn has_action(&self) -> bool {
        self.has_action
    }

    pub(crate) fn is_focusable(&self) -> bool {
        self.enabled && self.own_role().is_some_and(|role| role.is_focusable())
    }

    pub(crate) fn uses_text_cursor(&self) -> bool {
        self.root_role().is_some_and(|role| {
            role.is_text_input() || role.is_text_area() || role.is_number_input()
        })
    }

    pub(crate) fn uses_pointer_cursor(&self) -> bool {
        self.root_role()
            .is_some_and(semantic_role_uses_pointer_cursor)
    }

    pub(crate) fn is_retained_interaction_target(&self) -> bool {
        self.is_hittable()
            || self.draggable
            || self.resizable
            || self.has_any_gesture()
            || self.has_action()
    }
}

pub(crate) fn semantic_role_for_stable_id(tree: &Tree, stable_id: &str) -> Option<SemanticRole> {
    let mut candidate = stable_id;

    loop {
        if let Some(role) = tree
            .node_by_str(candidate)
            .and_then(|node_id| tree.get(node_id))
            .and_then(|node| node.props.semantic_role)
        {
            return Some(role);
        }

        let (prefix, _) = candidate.rsplit_once("::")?;
        candidate = prefix;
    }
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
