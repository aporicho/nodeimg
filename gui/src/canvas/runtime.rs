use super::layout::{CanvasNodeIdentity, CanvasNodeLayout};
use crate::renderer::Rect;
use crate::tree::{PersistenceClass, RuntimeRetention, RuntimeSlot, RuntimeSlotPolicy, UndoClass};

#[derive(Clone, Debug)]
pub(crate) struct CanvasNodeRuntime {
    pub(crate) owner_id: String,
    pub(crate) rect: Rect,
    pub(crate) z_index: i32,
    pub(crate) collapsed: bool,
}

impl CanvasNodeRuntime {
    pub(crate) fn from_identity(identity: &CanvasNodeIdentity, z_index: i32) -> Self {
        Self {
            owner_id: identity.owner_id.clone(),
            rect: identity.default_rect,
            z_index,
            collapsed: false,
        }
    }

    pub(crate) fn to_layout(&self) -> CanvasNodeLayout {
        CanvasNodeLayout {
            owner_id: self.owner_id.clone(),
            rect: self.rect,
            z_index: self.z_index,
            collapsed: self.collapsed,
        }
    }
}

impl Default for CanvasNodeRuntime {
    fn default() -> Self {
        Self {
            owner_id: String::new(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            z_index: 0,
            collapsed: false,
        }
    }
}

impl RuntimeSlot for CanvasNodeRuntime {
    fn default_policy() -> RuntimeSlotPolicy {
        RuntimeSlotPolicy {
            retention: RuntimeRetention::KeepWhileOwnerExists("canvas_node".to_string()),
            persistence: PersistenceClass::ProjectLayout,
            undo: UndoClass::Undoable,
        }
    }
}
