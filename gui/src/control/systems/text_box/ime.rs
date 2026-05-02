use super::output::output_for_editor;
use super::system::TextBoxSystem;
use crate::context::ImeRequest;
use crate::control::systems::SystemCx;
use crate::output::FrameworkOutput;
use crate::tree::{NodeId, Tree};

impl TextBoxSystem {
    pub(crate) fn ime_request(&self, tree: &Tree, focused: Option<NodeId>) -> ImeRequest {
        let focused = self.focused_control_id(tree, focused);
        ImeRequest {
            allowed: focused.is_some(),
            cursor_area: focused
                .as_deref()
                .and_then(|control_id| self.store.text_box(control_id).map(|r| r.caret_rect())),
        }
    }

    pub(crate) fn paste_focused_text(
        &mut self,
        tree: &Tree,
        focused: Option<NodeId>,
        text: &str,
    ) -> FrameworkOutput {
        if text.is_empty() {
            return FrameworkOutput::default();
        }
        let Some(control_id) = self.focused_control_id(tree, focused) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.text_box_mut(&control_id) else {
            return FrameworkOutput::default();
        };
        runtime.clear_preedit();
        runtime.editor_mut().paste(text);
        output_for_editor(self, tree, &control_id).with_consumed(true)
    }

    pub(super) fn commit_text_input(&mut self, cx: &SystemCx<'_>, text: &str) -> FrameworkOutput {
        if text.is_empty() {
            return FrameworkOutput::default();
        }
        let Some(control_id) = self.focused_control_id_for(cx) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.text_box_mut(&control_id) else {
            return FrameworkOutput::default();
        };
        runtime.clear_preedit();
        runtime.editor_mut().insert_str(text);
        self.output_for_editor_for(cx, &control_id)
            .with_consumed(true)
    }

    pub(super) fn handle_ime_preedit(
        &mut self,
        cx: &SystemCx<'_>,
        text: &str,
        caret: Option<(usize, usize)>,
    ) -> FrameworkOutput {
        let Some(control_id) = self.focused_control_id_for(cx) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.text_box_mut(&control_id) else {
            return FrameworkOutput::default();
        };
        runtime.set_preedit(text, caret);
        FrameworkOutput::consumed()
    }
}
