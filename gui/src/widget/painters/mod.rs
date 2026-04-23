mod button;
mod checkbox;
mod collapsible;
mod dropdown;
mod radio;
mod slider;
mod text_input;
mod toggle;

use crate::interaction::InteractionState;
use crate::renderer::{Color, Rect, RectStyle, TextStyle};
use crate::theme::Theme;
use crate::tree::paint_target::PaintTarget;
use crate::tree::{NodeId, Tree};
use crate::widget::state::TextInputStore;

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
        .or_else(|| text_input::visual_override(tree, node_id, interaction, theme))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_text_leaf_override(
    tree: &Tree,
    node_id: NodeId,
    target: &mut dyn PaintTarget,
    node_rect: Rect,
    interaction: Option<&InteractionState>,
    text_inputs: Option<&TextInputStore>,
    theme: &Theme,
    content: &str,
    text_style: &TextStyle,
) -> bool {
    text_input::paint_text_leaf(
        tree,
        node_id,
        target,
        node_rect,
        interaction,
        text_inputs,
        theme,
        content,
        text_style,
    )
}

fn transparent_style() -> RectStyle {
    RectStyle {
        color: Color::TRANSPARENT,
        border: None,
        radius: [0.0; 4],
        shadow: None,
    }
}
