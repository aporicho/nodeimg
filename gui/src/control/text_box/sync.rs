use super::retained_spec::retained_control_text_box_spec;
use super::system::TextBoxSystem;
use crate::canvas::canvas_node_stable_id;
use crate::canvas::node_template::CanvasNodeRenderView;
use crate::control::SystemCx;
use crate::renderer::TextMeasurer;
use crate::theme::Theme;
use crate::tree::{NodeId, Tree};

impl TextBoxSystem {
    pub(crate) fn sync_canvas_text_boxes(
        &mut self,
        tree: &Tree,
        views: &[CanvasNodeRenderView],
        measurer: &mut TextMeasurer,
        theme: &Theme,
        focused: Option<NodeId>,
    ) {
        let focused_control_id = self.focused_control_id(tree, focused);
        for view in views {
            let stable_id = canvas_node_stable_id(&view.state.owner_id);
            for param in &view.template.params {
                let control_id =
                    format!("{stable_id}::body::param::{}::control::content", param.key);
                let Some(spec) = retained_control_text_box_spec(&param.control, theme) else {
                    continue;
                };
                self.store.sync_text_box(
                    tree,
                    measurer,
                    theme,
                    control_id,
                    spec,
                    focused_control_id.as_deref(),
                );
            }
        }
        self.sync_sessions(tree, focused, None);
    }

    pub(super) fn focused_control_id(
        &self,
        tree: &Tree,
        focused: Option<NodeId>,
    ) -> Option<String> {
        self.store.focused_control_id(tree, focused)
    }

    pub(super) fn captured_control_id(
        &self,
        tree: &Tree,
        captured: Option<NodeId>,
    ) -> Option<String> {
        self.store.focused_control_id(tree, captured)
    }

    pub(super) fn focused_control_id_for(&self, cx: &SystemCx<'_>) -> Option<String> {
        self.focused_control_id(cx.tree(), cx.focused_node())
    }

    pub(super) fn captured_control_id_for(&self, cx: &SystemCx<'_>) -> Option<String> {
        self.captured_control_id(cx.tree(), cx.captured_node())
    }

    pub(super) fn sync_sessions_for(&mut self, cx: &SystemCx<'_>) {
        self.sync_sessions(cx.tree(), cx.focused_node(), cx.captured_node());
    }

    fn sync_sessions(&mut self, tree: &Tree, focused: Option<NodeId>, captured: Option<NodeId>) {
        let focused = self.focused_control_id(tree, focused);
        self.store.clear_unfocused_preedit(focused.as_deref());
        self.store.revert_unfocused_numbers(focused.as_deref());

        let keep_drag = self
            .active_drag_text_box
            .as_ref()
            .is_some_and(|control_id| {
                Some(control_id.as_str()) == focused.as_deref()
                    && Some(control_id.as_str())
                        == self.captured_control_id(tree, captured).as_deref()
                    && self.store.text_box(control_id).is_some()
            });
        if !keep_drag {
            self.active_drag_text_box = None;
        }
    }
}
