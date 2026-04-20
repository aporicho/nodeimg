use crate::context::ImeRequest;
use crate::interaction::InteractionState;
use crate::output::FrameworkOutput;
use crate::renderer::{Rect, TextMeasurer};
use crate::shell::AppEvent;
use crate::theme::Theme;
use crate::tree::{Desc, NodeId, Tree};
use crate::widget::state::TextInputStore;
use crate::widget::systems::{
    DropdownSystem, OverlayRequest, OverlaySystemCx, PopupSystem, SystemCx, TextInputSystem,
};

pub(crate) struct RuntimeSyncCx<'a> {
    pub(crate) tree: &'a Tree,
    pub(crate) interaction: &'a InteractionState,
    pub(crate) measurer: &'a mut TextMeasurer,
    pub(crate) theme: &'a Theme,
}

pub(crate) struct RuntimeEventCx<'a> {
    pub(crate) tree: &'a Tree,
    pub(crate) interaction: &'a mut InteractionState,
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeEventResult {
    pub(crate) output: FrameworkOutput,
    pub(crate) cancel_gesture: bool,
}

pub(crate) struct RuntimeSystems {
    popup: PopupSystem,
    dropdown: DropdownSystem,
    text_input: TextInputSystem,
}

impl RuntimeSystems {
    pub(crate) fn new() -> Self {
        Self {
            popup: PopupSystem::new(),
            dropdown: DropdownSystem::new(),
            text_input: TextInputSystem::new(),
        }
    }

    pub(crate) fn compose_desc(&mut self, tree: &Tree, desc: Desc, root_rect: Rect) -> Desc {
        self.popup.compose_desc(tree, desc, root_rect)
    }

    pub(crate) fn sync_with_tree(&mut self, cx: RuntimeSyncCx<'_>) {
        self.text_input.sync_with_tree(
            cx.tree,
            cx.measurer,
            cx.theme,
            cx.interaction.focused(),
            cx.interaction.captured(),
        );
        self.dropdown.sync_with_tree(cx.tree, &self.popup);
    }

    pub(crate) fn handle_pre_gesture_event(
        &mut self,
        cx: RuntimeEventCx<'_>,
        event: &AppEvent,
    ) -> RuntimeEventResult {
        {
            let popup_cx = SystemCx::new(cx.tree, cx.interaction);
            if self.popup.handle_event(popup_cx, event) {
                return RuntimeEventResult {
                    output: FrameworkOutput::consumed(),
                    cancel_gesture: true,
                };
            }
        }

        let dropdown_output = {
            let dropdown_cx = OverlaySystemCx::new(cx.tree, cx.interaction, &mut self.popup);
            self.dropdown.handle_event(dropdown_cx, event)
        };
        let text_output = {
            let text_cx = SystemCx::new(cx.tree, cx.interaction);
            self.text_input.handle_event(text_cx, event)
        };

        let cancel_gesture = output_has_pre_gesture_work(&dropdown_output)
            || output_has_pre_gesture_work(&text_output);

        RuntimeEventResult {
            output: dropdown_output.merge(text_output),
            cancel_gesture,
        }
    }

    pub(crate) fn open_overlay(&mut self, tree: &Tree, request: OverlayRequest) {
        self.popup.open(tree, request);
    }

    pub(crate) fn close_overlay(&mut self, tree: &Tree, interaction: &mut InteractionState) {
        self.popup.close(SystemCx::new(tree, interaction));
    }

    pub(crate) fn overlay_open(&self) -> bool {
        self.popup.is_open()
    }

    pub(crate) fn ime_request(&self, tree: &Tree, focused: Option<NodeId>) -> ImeRequest {
        self.text_input.ime_request(tree, focused)
    }

    pub(crate) fn paste_focused_text(
        &mut self,
        tree: &Tree,
        focused: Option<NodeId>,
        text: &str,
    ) -> FrameworkOutput {
        self.text_input.paste_focused_text(tree, focused, text)
    }

    pub(crate) fn text_input_store(&self) -> &TextInputStore {
        self.text_input.store()
    }
}

impl Default for RuntimeSystems {
    fn default() -> Self {
        Self::new()
    }
}

fn output_has_pre_gesture_work(output: &FrameworkOutput) -> bool {
    output.consumed || !output.events.is_empty() || !output.effects.is_empty()
}
