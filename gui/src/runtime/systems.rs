use crate::animation::AnimationStore;
use crate::canvas::node_template::CanvasNodeRenderView;
use crate::context::ImeRequest;
use crate::control::state::TextBoxStore;
use crate::control::systems::{SystemCx, TextBoxSystem};
use crate::control::ControlIntrinsic;
use crate::interaction::InteractionState;
use crate::output::FrameworkOutput;
use crate::overlay::OverlayRequest;
use crate::overlay::OverlaySystem;
use crate::shell::AppEvent;
use crate::template::TemplateRegistry;
use crate::theme::Theme;
use crate::tree::{MutationError, NodeId, Tree};

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
    text_box: TextBoxSystem,
}

impl RuntimeSystems {
    pub(crate) fn new() -> Self {
        Self {
            overlay: OverlaySystem::new(),
            text_box: TextBoxSystem::new(),
        }
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

        let text_output = {
            let text_cx = SystemCx::new(cx.tree, cx.animations, cx.interaction);
            self.text_box.handle_event(text_cx, event)
        };

        let cancel_gesture = output_has_pre_gesture_work(&text_output);

        RuntimeEventResult {
            output: text_output,
            cancel_gesture,
        }
    }

    pub(crate) fn open_overlay(&mut self, tree: &mut Tree, request: OverlayRequest) {
        self.overlay.open(tree, request);
    }

    pub(crate) fn close_overlay(&mut self, tree: &mut Tree, interaction: &mut InteractionState) {
        self.overlay.close(tree, interaction);
    }

    pub(crate) fn overlay_open(&self) -> bool {
        self.overlay.is_open()
    }

    pub(crate) fn sync_overlay_tree(
        &mut self,
        tree: &mut Tree,
        registry: &TemplateRegistry,
        theme: &Theme,
    ) -> Result<(), MutationError> {
        self.overlay.sync_tree(tree, registry, theme)
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

    pub(crate) fn sync_canvas_text_boxes(
        &mut self,
        tree: &Tree,
        views: &[CanvasNodeRenderView],
        measurer: &mut crate::renderer::TextMeasurer,
        theme: &crate::theme::Theme,
        focused: Option<NodeId>,
    ) {
        self.text_box
            .sync_canvas_text_boxes(tree, views, measurer, theme, focused);
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
