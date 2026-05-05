use super::Context;
use crate::event::router;
use crate::event::signal_output;
use crate::gesture::GestureSessionUpdate;
use crate::input::{
    PointerHitRequest, PointerHitResolver, PointerHitSnapshot, ScrollRequest, ScrollTargetResolver,
};
use crate::output::FrameworkOutput;
use crate::renderer::Rect;
use crate::runtime::{RuntimeEventCx, RuntimeEventResult};
use crate::shell::AppEvent;
use crate::tree::NodeId;

pub struct ImeRequest {
    pub allowed: bool,
    pub cursor_area: Option<Rect>,
}

impl Context {
    pub(crate) fn handle_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        let focused_before = self.focused_control_id().map(str::to_string);
        let output = router::handle_event(self, event);
        self.sync_overlay_tree();
        if matches!(
            event,
            AppEvent::MousePress { .. }
                | AppEvent::KeyPress { .. }
                | AppEvent::TextInput { .. }
                | AppEvent::ImePreedit { .. }
        ) {
            tracing::trace!(
                target: "nodeimg::render_trace::input",
                ?event,
                focused_before = focused_before.as_deref(),
                focused_after = self.focused_control_id(),
                events = output.events.len(),
                consumed = output.consumed,
                "gui input event handled"
            );
        }
        output
    }

    pub(crate) fn ime_request(&self) -> ImeRequest {
        self.systems
            .ime_request(&self.tree, self.interaction.focused())
    }

    pub(crate) fn paste_focused_text(&mut self, text: &str) -> FrameworkOutput {
        self.systems
            .paste_focused_text(&self.tree, self.interaction.focused(), text)
    }

    pub fn request_focus(&mut self, node_id: NodeId) {
        self.interaction.focus(node_id);
    }

    pub fn clear_focus(&mut self) {
        self.interaction.blur();
    }

    pub(crate) fn pointer_hit_snapshot(&self, event: &AppEvent) -> Option<PointerHitSnapshot> {
        PointerHitRequest::from_event(event)
            .map(|request| self.pointer_hit_resolver().snapshot_for_request(request))
    }

    pub(crate) fn pointer_hit_resolver(&self) -> PointerHitResolver<'_> {
        PointerHitResolver::new(&self.tree, Some(&self.animations))
    }

    pub(crate) fn scroll_target_for_request(
        &self,
        hit: Option<&PointerHitSnapshot>,
        request: ScrollRequest,
    ) -> Option<NodeId> {
        ScrollTargetResolver::new(&self.tree, Some(&self.animations))
            .target_for_request(hit, request)
    }

    pub(crate) fn handle_interaction_event(
        &mut self,
        event: &AppEvent,
        hit: Option<&PointerHitSnapshot>,
    ) {
        self.interaction
            .handle_event(&self.tree, Some(&self.animations), event, hit);
    }

    pub(crate) fn handle_runtime_pre_gesture_event(
        &mut self,
        event: &AppEvent,
        hit: Option<&PointerHitSnapshot>,
    ) -> RuntimeEventResult {
        self.systems.handle_pre_gesture_event(
            RuntimeEventCx {
                tree: &mut self.tree,
                interaction: &mut self.interaction,
                animations: Some(&self.animations),
                pointer_hit: hit,
            },
            event,
        )
    }

    pub(crate) fn handle_gesture_event(
        &mut self,
        event: &AppEvent,
        hit: Option<&PointerHitSnapshot>,
    ) -> FrameworkOutput {
        let update = self.handle_gesture_session_event(event, hit);
        let output = update
            .signal
            .as_ref()
            .map(|signal| signal_output::gesture_signal_output(&self.tree, signal))
            .unwrap_or_default();
        output.with_consumed(update.consumed)
    }

    pub(crate) fn handle_gesture_session_event(
        &mut self,
        event: &AppEvent,
        hit: Option<&PointerHitSnapshot>,
    ) -> GestureSessionUpdate {
        self.gesture_session
            .handle_event(&self.tree, Some(&self.animations), event, hit)
    }

    pub(crate) fn cancel_gesture(&mut self) {
        self.gesture_session.cancel();
    }
}
