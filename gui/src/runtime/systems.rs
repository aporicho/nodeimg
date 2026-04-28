use crate::animation::AnimationStore;
use crate::canvas::node_template::CanvasNodeRenderView;
use crate::context::ImeRequest;
use crate::interaction::InteractionState;
use crate::output::FrameworkOutput;
use crate::overlay::OverlayRequest;
use crate::overlay::OverlaySystem;
#[cfg(test)]
use crate::renderer::{Rect, TextMeasurer};
use crate::runtime::ControlIntrinsic;
use crate::shell::AppEvent;
#[cfg(test)]
use crate::theme::Theme;
#[cfg(test)]
use crate::tree::Desc;
use crate::tree::{NodeId, Tree};
use crate::widget::state::TextBoxStore;
use crate::widget::systems::{DropdownSystem, OverlaySystemCx, SystemCx, TextBoxSystem};

#[cfg(test)]
pub(crate) struct RuntimeSyncCx<'a> {
    pub(crate) tree: &'a mut Tree,
    pub(crate) interaction: &'a InteractionState,
    pub(crate) measurer: &'a mut TextMeasurer,
    pub(crate) theme: &'a Theme,
}

pub(crate) struct RuntimeEventCx<'a> {
    pub(crate) tree: &'a mut Tree,
    pub(crate) interaction: &'a mut InteractionState,
    pub(crate) animations: Option<&'a AnimationStore>,
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeEventResult {
    pub(crate) output: FrameworkOutput,
    pub(crate) cancel_gesture: bool,
}

pub(crate) struct RuntimeSystems {
    overlay: OverlaySystem,
    dropdown: DropdownSystem,
    text_box: TextBoxSystem,
}

impl RuntimeSystems {
    pub(crate) fn new() -> Self {
        Self {
            overlay: OverlaySystem::new(),
            dropdown: DropdownSystem::new(),
            text_box: TextBoxSystem::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn compose_desc(&mut self, tree: &Tree, desc: Desc, root_rect: Rect) -> Desc {
        self.overlay.compose_desc(tree, desc, root_rect)
    }

    #[cfg(test)]
    pub(crate) fn sync_with_tree(&mut self, cx: RuntimeSyncCx<'_>) {
        self.text_box.sync_with_tree(
            cx.tree,
            cx.measurer,
            cx.theme,
            cx.interaction.focused(),
            cx.interaction.captured(),
        );
        self.dropdown.sync_with_tree(cx.tree, &self.overlay);
    }

    pub(crate) fn handle_pre_gesture_event(
        &mut self,
        cx: RuntimeEventCx<'_>,
        event: &AppEvent,
    ) -> RuntimeEventResult {
        {
            if self
                .overlay
                .handle_event(cx.tree, cx.animations, cx.interaction, event)
            {
                return RuntimeEventResult {
                    output: FrameworkOutput::consumed(),
                    cancel_gesture: true,
                };
            }
        }

        let dropdown_output = {
            let dropdown_cx =
                OverlaySystemCx::new(cx.tree, cx.animations, cx.interaction, &mut self.overlay);
            self.dropdown.handle_event(dropdown_cx, event)
        };
        let text_output = {
            let text_cx = SystemCx::new(cx.tree, cx.animations, cx.interaction);
            self.text_box.handle_event(text_cx, event)
        };

        let cancel_gesture = output_has_pre_gesture_work(&dropdown_output)
            || output_has_pre_gesture_work(&text_output);

        RuntimeEventResult {
            output: dropdown_output.merge(text_output),
            cancel_gesture,
        }
    }

    pub(crate) fn open_overlay(&mut self, tree: &Tree, request: OverlayRequest) {
        self.overlay.open(tree, request);
    }

    pub(crate) fn close_overlay(&mut self, tree: &Tree, interaction: &mut InteractionState) {
        self.overlay.close(tree, interaction);
    }

    pub(crate) fn overlay_open(&self) -> bool {
        self.overlay.is_open()
    }

    pub(crate) fn ime_request(&self, tree: &Tree, focused: Option<NodeId>) -> ImeRequest {
        self.text_box.ime_request(tree, focused)
    }

    pub(crate) fn paste_focused_text(
        &mut self,
        tree: &Tree,
        focused: Option<NodeId>,
        text: &str,
    ) -> FrameworkOutput {
        self.text_box.paste_focused_text(tree, focused, text)
    }

    pub(crate) fn text_box_store(&self) -> &TextBoxStore {
        self.text_box.store()
    }

    pub(crate) fn control_intrinsics(&self) -> Vec<ControlIntrinsic> {
        self.text_box.store().control_intrinsics()
    }

    pub(crate) fn take_dirty_control_intrinsics(&mut self) -> Vec<ControlIntrinsic> {
        self.text_box.store_mut().take_dirty_control_intrinsics()
    }

    pub(crate) fn take_text_box_dirty_intrinsics(&mut self) -> std::collections::BTreeSet<String> {
        self.text_box.store_mut().take_dirty_intrinsics()
    }

    pub(crate) fn text_box_dirty_intrinsics(&self) -> Vec<String> {
        self.text_box.store().dirty_intrinsic_ids()
    }

    pub(crate) fn sync_retained_canvas_text_boxes(
        &mut self,
        tree: &Tree,
        views: &[CanvasNodeRenderView],
        measurer: &mut crate::renderer::TextMeasurer,
        theme: &crate::theme::Theme,
        focused: Option<NodeId>,
    ) {
        self.text_box
            .sync_retained_canvas_text_boxes(tree, views, measurer, theme, focused);
    }
}

impl Default for RuntimeSystems {
    fn default() -> Self {
        Self::new()
    }
}

fn output_has_pre_gesture_work(output: &FrameworkOutput) -> bool {
    output.consumed
        || !output.events.is_empty()
        || !output.actions.is_empty()
        || !output.effects.is_empty()
}
