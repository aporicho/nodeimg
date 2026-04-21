use crate::action::ActionId;
use std::borrow::Cow;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeProps {
    pub action_id: Option<ActionId>,
    pub semantic_role: Option<Cow<'static, str>>,
    pub owner_id: Option<Cow<'static, str>>,
}

impl NodeProps {
    pub fn with_action(action_id: impl Into<ActionId>) -> Self {
        Self {
            action_id: Some(action_id.into()),
            ..Self::default()
        }
    }
}
