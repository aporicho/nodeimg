use super::layout::{CanvasNodeIdentity, CanvasNodeLayout};
use super::port::canvas_port_group_stable_id;
use super::port::{parse_canvas_port_id, CanvasPendingConnectionView, CanvasPortGroupView};
use crate::renderer::Rect;
use crate::tree::{PersistenceClass, RuntimeRetention, RuntimeSlot, RuntimeSlotPolicy, UndoClass};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub(crate) struct CanvasNodeRuntime {
    pub(crate) owner_id: String,
    pub(crate) rect: Rect,
    pub(crate) z_index: i32,
    pub(crate) collapsed: bool,
    pub(crate) user_min_height: Option<f32>,
}

impl CanvasNodeRuntime {
    pub(crate) fn from_identity(identity: &CanvasNodeIdentity, z_index: i32) -> Self {
        Self {
            owner_id: identity.owner_id.clone(),
            rect: identity.default_rect,
            z_index,
            collapsed: false,
            user_min_height: None,
        }
    }

    pub(crate) fn to_layout(&self) -> CanvasNodeLayout {
        CanvasNodeLayout {
            owner_id: self.owner_id.clone(),
            rect: self.rect,
            z_index: self.z_index,
            collapsed: self.collapsed,
            user_min_height: self.user_min_height,
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
            user_min_height: None,
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

#[derive(Clone, Debug, Default)]
pub(crate) struct CanvasInteractionRuntime {
    selected_owner_ids: HashSet<String>,
    open_port_group_ids: HashSet<String>,
    pending_connection: Option<CanvasPendingConnectionView>,
    hovered_port_id: Option<String>,
}

impl CanvasInteractionRuntime {
    pub(crate) fn select_single(&mut self, owner_id: &str) {
        self.selected_owner_ids.clear();
        self.selected_owner_ids.insert(owner_id.to_string());
    }

    pub(crate) fn clear(&mut self) -> bool {
        if self.selected_owner_ids.is_empty() {
            return false;
        }
        self.selected_owner_ids.clear();
        true
    }

    pub(crate) fn is_selected(&self, owner_id: &str) -> bool {
        self.selected_owner_ids.contains(owner_id)
    }

    pub(crate) fn port_group_view(
        &self,
        owner_id: &str,
        side: super::port::CanvasPortSide,
    ) -> CanvasPortGroupView {
        CanvasPortGroupView {
            open: self
                .open_port_group_ids
                .contains(&canvas_port_group_stable_id(owner_id, side)),
        }
    }

    pub(crate) fn toggle_port_group(
        &mut self,
        owner_id: &str,
        side: super::port::CanvasPortSide,
    ) -> bool {
        let stable_id = canvas_port_group_stable_id(owner_id, side);
        if self.open_port_group_ids.contains(&stable_id) {
            self.open_port_group_ids.remove(&stable_id);
            false
        } else {
            self.open_port_group_ids.insert(stable_id);
            true
        }
    }

    pub(crate) fn pending_connection(&self) -> Option<&CanvasPendingConnectionView> {
        self.pending_connection.as_ref()
    }

    pub(crate) fn begin_pending_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        let Some(port) = parse_canvas_port_id(from_port_id) else {
            return false;
        };
        if port.side != super::port::CanvasPortSide::Output {
            return false;
        }
        self.pending_connection = Some(CanvasPendingConnectionView {
            from_port_id: from_port_id.to_string(),
            cursor_canvas,
        });
        true
    }

    pub(crate) fn update_pending_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        let Some(pending) = &mut self.pending_connection else {
            return false;
        };
        if pending.cursor_canvas == cursor_canvas {
            return false;
        }
        pending.cursor_canvas = cursor_canvas;
        true
    }

    pub(crate) fn end_pending_connection(&mut self) -> Option<CanvasPendingConnectionView> {
        let pending = self.pending_connection.take();
        if pending.is_some() {
            self.hovered_port_id = None;
        }
        pending
    }

    pub(crate) fn cancel_pending_connection(&mut self) -> bool {
        let had_pending = self.pending_connection.take().is_some();
        if had_pending {
            self.hovered_port_id = None;
        }
        had_pending
    }

    pub(crate) fn hovered_port_id(&self) -> Option<&str> {
        self.hovered_port_id.as_deref()
    }

    pub(crate) fn set_hovered_port(&mut self, port_id: Option<&str>) -> bool {
        if let Some(port_id) = port_id {
            if parse_canvas_port_id(port_id).is_none() {
                return false;
            }
        }
        if self.hovered_port_id.as_deref() == port_id {
            return false;
        }
        self.hovered_port_id = port_id.map(str::to_string);
        true
    }

    pub(crate) fn retain_owner_ids(&mut self, owner_ids: &HashSet<&str>) {
        self.selected_owner_ids
            .retain(|owner_id| owner_ids.contains(owner_id.as_str()));
        self.open_port_group_ids.retain(|stable_id| {
            let Some(owner_id) = stable_id.strip_prefix("canvas_node::").and_then(|id| {
                id.rsplit_once("::port_group::")
                    .map(|(owner_id, _)| owner_id)
            }) else {
                return false;
            };
            owner_ids.contains(owner_id)
        });
        let pending_owner_id = self
            .pending_connection
            .as_ref()
            .and_then(|pending| parse_canvas_port_id(&pending.from_port_id))
            .map(|port| port.owner_id);
        if pending_owner_id.is_some_and(|owner_id| !owner_ids.contains(owner_id.as_str())) {
            self.pending_connection = None;
        }
        let hovered_owner_id = self
            .hovered_port_id
            .as_ref()
            .and_then(|port_id| parse_canvas_port_id(port_id))
            .map(|port| port.owner_id);
        if hovered_owner_id.is_some_and(|owner_id| !owner_ids.contains(owner_id.as_str())) {
            self.hovered_port_id = None;
        }
    }
}

impl RuntimeSlot for CanvasInteractionRuntime {
    fn default_policy() -> RuntimeSlotPolicy {
        RuntimeSlotPolicy {
            retention: RuntimeRetention::KeepForSession,
            persistence: PersistenceClass::SessionOnly,
            undo: UndoClass::NonUndoable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUTPUT_PORT: &str = "canvas_node::node::1::port::output::image";

    #[test]
    fn hovered_port_update_is_idempotent() {
        let mut runtime = CanvasInteractionRuntime::default();

        assert!(runtime.set_hovered_port(Some(OUTPUT_PORT)));
        assert!(!runtime.set_hovered_port(Some(OUTPUT_PORT)));
        assert!(runtime.set_hovered_port(None));
        assert!(!runtime.set_hovered_port(None));
    }

    #[test]
    fn pending_connection_update_is_idempotent() {
        let mut runtime = CanvasInteractionRuntime::default();

        assert!(runtime.begin_pending_connection(OUTPUT_PORT, [1.0, 2.0]));
        assert!(!runtime.update_pending_connection([1.0, 2.0]));
        assert!(runtime.update_pending_connection([2.0, 3.0]));
    }
}
