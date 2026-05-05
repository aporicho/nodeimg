use std::borrow::Cow;

use crate::action::ActionId;

use super::SemanticRole;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeProps {
    pub action_id: Option<ActionId>,
    pub semantic_role: Option<SemanticRole>,
    pub owner_id: Option<Cow<'static, str>>,
    pub enabled: bool,
}

impl Default for NodeProps {
    fn default() -> Self {
        Self {
            action_id: None,
            semantic_role: None,
            owner_id: None,
            enabled: true,
        }
    }
}

impl NodeProps {
    pub fn with_action(action_id: impl Into<ActionId>) -> Self {
        Self {
            action_id: Some(action_id.into()),
            enabled: true,
            ..Self::default()
        }
    }
}
