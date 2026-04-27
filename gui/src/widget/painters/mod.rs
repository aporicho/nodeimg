mod button;
mod checkbox;
mod collapsible;
mod dropdown;
mod owner;
mod radio;
mod slider;
mod text_box;
mod toggle;

use crate::interaction::InteractionState;
use crate::renderer::{Color, Rect, RectStyle, TextStyle};
use crate::theme::Theme;
use crate::tree::paint_target::PaintTarget;
use crate::tree::{NodeId, Tree};
use crate::widget::state::TextBoxStore;

pub(crate) fn widget_visual_override(
    tree: &Tree,
    node_id: NodeId,
    interaction: Option<&InteractionState>,
    theme: &Theme,
) -> Option<(RectStyle, Color)> {
    let node = tree.get(node_id)?;
    button::visual_override(node_id, &node.kind, interaction, theme)
        .or_else(|| checkbox::visual_override(tree, node_id, interaction, theme))
        .or_else(|| collapsible::visual_override(tree, node_id, interaction, theme))
        .or_else(|| dropdown::visual_override(tree, node_id, interaction, theme))
        .or_else(|| radio::visual_override(tree, node_id, interaction, theme))
        .or_else(|| toggle::visual_override(tree, node_id, interaction, theme))
        .or_else(|| slider::visual_override(tree, node_id, interaction, theme))
        .or_else(|| text_box::visual_override(tree, node_id, interaction, theme))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_text_leaf_override(
    tree: &Tree,
    node_id: NodeId,
    target: &mut dyn PaintTarget,
    node_rect: Rect,
    interaction: Option<&InteractionState>,
    text_boxes: Option<&TextBoxStore>,
    theme: &Theme,
    content: &str,
    text_style: &TextStyle,
) -> bool {
    let handled = text_box::paint_text_leaf(
        tree,
        node_id,
        target,
        node_rect,
        interaction,
        text_boxes,
        theme,
        content,
        text_style,
    );
    if !handled && !content.is_empty() {
        if let Some(node) = tree.get(node_id) {
            let node_id_str = node.id.as_ref();
            if node_id_str.ends_with("::value") {
                tracing::trace!(
                    target: "gui::widget::text_area",
                    node_id = %node_id_str,
                    content_bytes = content.len(),
                    "text value leaf will use default text renderer"
                );
            }
        }
    }
    handled
}

fn transparent_style() -> RectStyle {
    RectStyle {
        color: Color::TRANSPARENT,
        border: None,
        radius: [0.0; 4],
        shadow: None,
    }
}
