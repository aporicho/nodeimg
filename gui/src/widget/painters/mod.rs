mod button;
mod checkbox;
mod collapsible;
mod dropdown;
mod radio;
mod slider;
mod text_input;
mod toggle;

use crate::renderer::{Color, RectStyle, Renderer};
use crate::theme::Theme;
use crate::tree::{NodeId, Tree};
use crate::widget::state::{InteractionStore, TextInputStore};

pub(crate) fn widget_visual_override(
    tree: &Tree,
    node_id: NodeId,
    interaction: Option<&InteractionStore>,
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
    renderer: &mut Renderer,
    tf: crate::tree::paint_helpers::PaintTransform,
    interaction: Option<&InteractionStore>,
    text_inputs: Option<&TextInputStore>,
    theme: &Theme,
    content: &str,
    font_size: f32,
    text_color: Color,
) -> bool {
    text_input::paint_text_leaf(
        tree,
        node_id,
        renderer,
        tf,
        interaction,
        text_inputs,
        theme,
        content,
        font_size,
        text_color,
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
