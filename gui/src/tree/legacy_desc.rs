use super::desc::Desc;
use super::diff;
use super::tree::Tree;
use crate::renderer::Rect;
use crate::renderer::TextMeasurer;
use crate::theme::Theme;
use crate::tree::layout;
use crate::widget::props::WidgetBuildCx;

/// Legacy retained-tree boundary for `Desc` reconciliation.
///
/// Owner: UI engine migration.
/// Deletion condition: all workspace/canvas/widget production updates use
/// `TemplateRegistry` plus `TreeMutation`, and legacy `Desc` paths are test-only.
pub fn reconcile(tree: &mut Tree, desc: Desc, cx: WidgetBuildCx<'_>) {
    diff::reconcile(tree, desc, cx);
}

pub(crate) fn update_tree_from_desc(
    tree: &mut Tree,
    desc: Desc,
    root_rect: Rect,
    measurer: &mut TextMeasurer,
    theme: &Theme,
    force_rebuild: bool,
) {
    let build_cx = WidgetBuildCx {
        theme,
        force_rebuild,
    };
    reconcile(tree, desc, build_cx);
    if let Some(root) = tree.root() {
        layout(tree, root, root_rect, &mut |text, style| {
            measurer.measure_with_style(text, style)
        });
        tree.mark_paint_dirty(root, super::repaint::PaintDirtyReason::Structure);
    }
}
